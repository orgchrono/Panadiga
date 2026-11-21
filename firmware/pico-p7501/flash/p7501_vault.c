/**
 * @file p7501_vault.c
 * @brief True Atomic A/B Copy-on-Write RP2040/RP2350 QSPI Flash Forensic Vault (`P2_VAULT`).
 *
 * Uses Bank A (`0x100000`, 256 KiB) and Bank B (`0x140000`, 256 KiB) with atomic
 * manifest flipping: new payloads and unchanged sibling slots are staged and
 * XIP-CRC32 verified in the inactive bank BEFORE the inactive bank's manifest is
 * committed with `write_count + 1`. Power loss at any point preserves the prior bank.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_vault_codec.h"
#include "cli/p7501_cli.h"
#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "flash/p7501_qspi_storage.h"
#include "hal/p7501_watchdog.h"
#include "uart/p7501_uart_codec.h"

#include "hardware/flash.h"
#include "pico/stdlib.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static uint8_t s_staging_bank = 0u;
static bool s_staging_active = false;

static int vault_load_active_ab(p7501_vault_manifest_t *out_active) {
    const uint32_t total = p7501_qspi_total_bytes();
    const uint32_t off_a = p7501_vault_bank_offset(total, 0u);
    const uint32_t off_b = p7501_vault_bank_offset(total, 1u);
    if ((off_a == 0UL) || (off_b == 0UL)) {
        p7501_vault_manifest_init(out_active);
        return -1;
    }
    p7501_vault_manifest_t ma;
    p7501_vault_manifest_t mb;
    memcpy(&ma, p7501_qspi_xip_ptr(off_a), sizeof(ma));
    memcpy(&mb, p7501_qspi_xip_ptr(off_b), sizeof(mb));
    return p7501_vault_select_ab(&ma, &mb, out_active);
}

static bool vault_verify_slot_xip_crc(
    uint32_t total_qspi,
    uint8_t bank_ab,
    uint8_t slot_index,
    const p7501_vault_slot_meta_t *sm
) {
    uint32_t slot_off = 0UL;
    if ((sm == NULL) || !sm->valid || (sm->byte_len == 0UL) ||
        (sm->byte_len > P7501_VAULT_SLOT_BYTES) ||
        !p7501_vault_slot_qspi_offset_ab(total_qspi, bank_ab, slot_index, &slot_off)) {
        return false;
    }
    uint32_t crc = 0UL;
    const uint8_t *base = p7501_qspi_xip_ptr(slot_off);
    for (uint32_t off = 0UL; off < sm->byte_len; off += 4096UL) {
        const uint32_t chunk = ((sm->byte_len - off) > 4096UL) ? 4096UL : (sm->byte_len - off);
        crc = p7501_crc32_ieee_update(crc, &base[off], chunk);
        p7501_wdt_kick();
    }
    return crc == sm->data_crc32;
}

static bool vault_copy_sibling_slots(
    uint32_t total_qspi,
    int active_bank,
    uint8_t target_bank,
    uint8_t skip_slot,
    const p7501_vault_manifest_t *active_m
) {
    if ((active_bank < 0) || (active_m == NULL)) {
        return true;
    }
    for (uint8_t s = 1u; s <= P7501_VAULT_PAYLOAD_SLOTS; ++s) {
        if (s == skip_slot) {
            continue;
        }
        const p7501_vault_slot_meta_t *sm = &active_m->slots[s - 1u];
        if (!sm->valid || (sm->byte_len == 0UL) || (sm->byte_len > P7501_VAULT_SLOT_BYTES)) {
            continue;
        }
        uint32_t src_off = 0UL;
        uint32_t dst_off = 0UL;
        if (!p7501_vault_slot_qspi_offset_ab(total_qspi, (uint8_t)active_bank, s, &src_off) ||
            !p7501_vault_slot_qspi_offset_ab(total_qspi, target_bank, s, &dst_off)) {
            return false;
        }
        if (!p7501_qspi_erase_range_safe(P7501_QSPI_PART_VAULT, dst_off, P7501_VAULT_SLOT_BYTES)) {
            return false;
        }
        /* Copy in 4 KiB chunks via arena SRAM (flash_range_program cannot read XIP directly). */
        uint8_t *copy_buf = g_p7501_cmd_arena.raw;
        for (uint32_t off = 0UL; off < sm->byte_len; off += FLASH_SECTOR_SIZE) {
            const uint32_t chunk = ((sm->byte_len - off) > FLASH_SECTOR_SIZE)
                ? FLASH_SECTOR_SIZE
                : (sm->byte_len - off);
            memcpy(copy_buf, p7501_qspi_xip_ptr(src_off + off), chunk);
            if (!p7501_qspi_program_range_safe(P7501_QSPI_PART_VAULT, dst_off + off, copy_buf, chunk)) {
                return false;
            }
        }
        if (!vault_verify_slot_xip_crc(total_qspi, target_bank, s, sm)) {
            return false;
        }
    }
    return true;
}

bool p7501_vault_write_slot_begin_hw(
    uint8_t slot_index,
    size_t total_len,
    uint32_t *out_slot_off
) {
    const uint32_t total = p7501_qspi_total_bytes();
    p7501_vault_manifest_t active_m;
    const int active_bank = vault_load_active_ab(&active_m);
    const uint8_t target_bank = (active_bank == 0) ? 1u : 0u;

    uint32_t slot_off = 0UL;
    if ((out_slot_off == NULL) || (total_len == 0u) || (total_len > P7501_VAULT_SLOT_BYTES) ||
        !p7501_vault_slot_qspi_offset_ab(total, target_bank, slot_index, &slot_off) ||
        !p7501_qspi_erase_range_safe(P7501_QSPI_PART_VAULT, slot_off, P7501_VAULT_SLOT_BYTES)) {
        s_staging_active = false;
        return false;
    }
    s_staging_bank = target_bank;
    s_staging_active = true;
    *out_slot_off = slot_off;
    return true;
}

bool p7501_vault_write_slot_chunk_hw(
    uint32_t slot_off,
    uint32_t rel_off,
    const uint8_t *data,
    size_t len
) {
    if (!s_staging_active || (data == NULL) || (len == 0u) ||
        ((rel_off % FLASH_PAGE_SIZE) != 0UL) ||
        (((uint64_t)rel_off + (uint64_t)len) > P7501_VAULT_SLOT_BYTES)) {
        s_staging_active = false;
        return false;
    }
    if (!p7501_qspi_program_range_safe(P7501_QSPI_PART_VAULT, slot_off + rel_off, data, len)) {
        s_staging_active = false;
        return false;
    }
    return true;
}

bool p7501_vault_write_slot_commit_hw(
    uint8_t slot_index,
    uint32_t target_addr,
    size_t total_len,
    uint32_t data_crc32,
    const char *model_tag
) {
    if (!s_staging_active) {
        return false;
    }
    s_staging_active = false;
    const uint32_t total = p7501_qspi_total_bytes();
    const uint8_t target_bank = s_staging_bank;
    const uint32_t target_hdr_off = p7501_vault_bank_offset(total, target_bank);
    if (target_hdr_off == 0UL) {
        return false;
    }

    p7501_vault_manifest_t active_m;
    const int active_bank = vault_load_active_ab(&active_m);

    /* 1. Verify newly programmed slot payload CRC32 directly over XIP before touching manifest. */
    p7501_vault_manifest_t next_m = active_m;
    if (!p7501_vault_manifest_record_slot_crc(
            &next_m, slot_index, target_addr, total_len, data_crc32, model_tag)) {
        return false;
    }
    if (!vault_verify_slot_xip_crc(total, target_bank, slot_index, &next_m.slots[slot_index - 1u])) {
        return false;
    }

    /* 2. Copy unchanged valid sibling slots into inactive bank so bank switch is atomic. */
    if (!vault_copy_sibling_slots(total, active_bank, target_bank, slot_index, &active_m)) {
        return false;
    }

    /* 3. Atomically commit inactive bank manifest (higher write_count makes it active). */
    return p7501_qspi_write_sector_page(
        P7501_QSPI_PART_VAULT, target_hdr_off, &next_m, sizeof(next_m)
    );
}

bool p7501_vault_write_slot_hw(
    uint8_t slot_index,
    uint32_t target_addr,
    const uint8_t *payload,
    size_t payload_len,
    const char *model_tag
) {
    uint32_t slot_off = 0UL;
    if ((payload == NULL) ||
        !p7501_vault_write_slot_begin_hw(slot_index, payload_len, &slot_off) ||
        !p7501_vault_write_slot_chunk_hw(slot_off, 0u, payload, payload_len)) {
        s_staging_active = false;
        return false;
    }
    return p7501_vault_write_slot_commit_hw(
        slot_index, target_addr, payload_len, p7501_crc32_ieee(payload, payload_len), model_tag
    );
}

void p7501_cmd_vault(const char *arg) {
    char tok1[24] = {0};
    char tok2[24] = {0};
    char tok3[24] = {0};
    const char *cur = p7501_next_token(arg, tok1, sizeof(tok1));
    cur = (cur != NULL) ? p7501_next_token(cur, tok2, sizeof(tok2)) : NULL;
    if (cur != NULL) {
        (void)p7501_next_token(cur, tok3, sizeof(tok3));
    }

    const uint32_t total = p7501_qspi_total_bytes();
    p7501_vault_manifest_t m;
    const int active_bank = vault_load_active_ab(&m);
    const bool valid = (active_bank >= 0);
    const uint32_t base = valid
        ? p7501_vault_bank_offset(total, (uint8_t)active_bank)
        : p7501_vault_base_offset(total);

    if ((tok1[0] == '\0') || p7501_ascii_ieq(tok1, "STATUS") || p7501_ascii_ieq(tok1, "LIST")) {
        printf("[VAULT] qspi=0x%08" PRIX32 " bank=%s vault_off=0x%08" PRIX32 " valid=%s writes=%" PRIu32 "\n",
               total,
               (active_bank == 1) ? "B" : ((active_bank == 0) ? "A" : "EMPTY"),
               base,
               valid ? "YES" : "EMPTY",
               valid ? m.write_count : 0u);
        for (uint8_t s = 1u; s <= P7501_VAULT_PAYLOAD_SLOTS; ++s) {
            const p7501_vault_slot_meta_t *sm = &m.slots[s - 1u];
            if (valid && sm->valid) {
                const bool crc_ok = vault_verify_slot_xip_crc(total, (uint8_t)active_bank, s, sm);
                printf("  slot=%u model=%s addr=0x%08" PRIX32 " len=%" PRIu32 " crc32=0x%08" PRIX32 " xip_crc=%s seq=%" PRIu32 "\n",
                       (unsigned)s, sm->model_tag[0] ? sm->model_tag : "GENERIC",
                       sm->target_addr, sm->byte_len, sm->data_crc32,
                       crc_ok ? "OK" : "CORRUPT", sm->sequence);
            } else {
                printf("  slot=%u EMPTY\n", (unsigned)s);
            }
        }
        printf("VAULT: OK\n");
        return;
    }

    uint32_t slot_num = 0u;
    if (!p7501_parse_u32_strict(tok2, &slot_num) || (slot_num == 0u) ||
        (slot_num > P7501_VAULT_PAYLOAD_SLOTS)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("[VAULT] ERROR: Specify slot 1..3 (Usage: VAULT [STATUS|LIST|DUMP <1..3>|ERASE <1..3> CONFIRM])\n");
        return;
    }

    if (p7501_ascii_ieq(tok1, "DUMP")) {
        const p7501_vault_slot_meta_t *sm = &m.slots[slot_num - 1u];
        uint32_t slot_off = 0UL;
        if (!valid || !sm->valid ||
            !p7501_vault_slot_qspi_offset_ab(total, (uint8_t)active_bank, (uint8_t)slot_num, &slot_off)) {
            p7501_cli_set_result("ERR", "EMPTY");
            printf("[VAULT] Slot %lu is empty.\n", (unsigned long)slot_num);
            return;
        }
        if (!vault_verify_slot_xip_crc(total, (uint8_t)active_bank, (uint8_t)slot_num, sm)) {
            p7501_cli_set_result("ERR", "ECRC");
            printf("[VAULT] ERROR: Slot %lu payload CRC32 mismatch on XIP readback!\n", (unsigned long)slot_num);
            return;
        }
        const uint8_t *p = p7501_qspi_xip_ptr(slot_off);
        const uint32_t preview = (sm->byte_len > 256u) ? 256u : sm->byte_len;
        printf("[VAULT] Dumping first %" PRIu32 "/%" PRIu32 " bytes of slot %lu (bank=%s crc32=0x%08" PRIX32 "):\n",
               preview, sm->byte_len, (unsigned long)slot_num,
               (active_bank == 1) ? "B" : "A", sm->data_crc32);
        for (uint32_t i = 0u; i < preview; ++i) {
            if ((i % 16u) == 0u) {
                printf("%04" PRIX32 ": ", i);
            }
            printf("%02X%c", p[i], ((i % 16u) == 15u) ? '\n' : ' ');
        }
        printf("\nVAULT: OK\n");
        return;
    }

    if (p7501_ascii_ieq(tok1, "ERASE") && p7501_ascii_ieq(tok3, "CONFIRM")) {
        const uint8_t target_bank = (active_bank == 0) ? 1u : 0u;
        const uint32_t target_hdr = p7501_vault_bank_offset(total, target_bank);
        uint32_t target_slot_off = 0UL;
        if ((target_hdr != 0UL) &&
            p7501_vault_slot_qspi_offset_ab(total, target_bank, (uint8_t)slot_num, &target_slot_off)) {
            p7501_vault_manifest_t next_m = m;
            (void)p7501_vault_manifest_clear_slot(&next_m, (uint8_t)slot_num);
            if (!vault_copy_sibling_slots(total, active_bank, target_bank, (uint8_t)slot_num, &m) ||
                !p7501_qspi_erase_range_safe(P7501_QSPI_PART_VAULT, target_slot_off, P7501_VAULT_SLOT_BYTES) ||
                !p7501_qspi_write_sector_page(P7501_QSPI_PART_VAULT, target_hdr, &next_m, sizeof(next_m))) {
                p7501_cli_set_result("ERR", "EQSPI");
                printf("[VAULT] ERROR: Atomic A/B QSPI erase failed on slot %lu.\n", (unsigned long)slot_num);
                return;
            }
            printf("VAULT: OK (Slot %lu atomically erased -> bank=%s)\n",
                   (unsigned long)slot_num, (target_bank == 1u) ? "B" : "A");
            return;
        }
    }
    p7501_cli_set_result("ERR", "EINVAL");
    printf("[VAULT] ERROR: Invalid syntax. Pass CONFIRM for ERASE.\n");
}
