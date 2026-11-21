/**
 * @file p7501_qspi_storage.c
 * @brief RP2040/RP2350 QSPI Flash runtime handlers for `BLACKBOX` (`P4`), `PROBE_CFG` (`P5`), and `RECIPE` (`P3`).
 *
 * Low-level QSPI erase/program primitives live in `flash/p7501_qspi_hal.c`.
 * `BLACKBOX DUMP` outputs valid ring pages in strict chronological (`sequence`) order.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_qspi_storage.h"
#include "cli/p7501_cli.h"
#include "common/p7501_hex.h"
#include "flash/p7501_flash.h"
#include "flash/p7501_qspi_map.h"
#include "hal/p7501_hw.h"
#include "uart/p7501_uart_codec.h"

#include "pico/stdlib.h"
#include "pico/time.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static void blackbox_scan_sequences(uint32_t seqs[P7501_BLACKBOX_PAGES], uint32_t *out_valid_count) {
    const uint32_t total = p7501_qspi_total_bytes();
    uint32_t count = 0u;
    for (uint8_t i = 0u; i < P7501_BLACKBOX_PAGES; ++i) {
        seqs[i] = 0u;
        uint32_t off = 0UL;
        if (!p7501_blackbox_page_qspi_offset(total, i, &off)) {
            continue;
        }
        p7501_blackbox_rec_t rec;
        memcpy(&rec, p7501_qspi_xip_ptr(off), sizeof(rec));
        if (p7501_blackbox_verify_record(&rec)) {
            seqs[i] = rec.sequence;
            ++count;
        }
    }
    if (out_valid_count != NULL) {
        *out_valid_count = count;
    }
}

bool p7501_blackbox_append_hw(
    uint32_t vdd_mv,
    uint32_t nsw5v_drops,
    uint32_t event_mask,
    const char *model_tag,
    const char *summary
) {
    uint32_t seqs[P7501_BLACKBOX_PAGES];
    blackbox_scan_sequences(seqs, NULL);
    uint32_t next_seq = 1u;
    const uint8_t page = p7501_blackbox_select_next_page(seqs, &next_seq);
    uint32_t off = 0UL;
    if (!p7501_blackbox_page_qspi_offset(p7501_qspi_total_bytes(), page, &off)) {
        return false;
    }

    p7501_blackbox_rec_t rec;
    const uint32_t uptime_ms = to_ms_since_boot(get_absolute_time());
    p7501_blackbox_build_record(
        &rec, next_seq, uptime_ms, vdd_mv, nsw5v_drops, event_mask, model_tag, summary
    );
    return p7501_qspi_write_sector_page(P7501_QSPI_PART_BLACKBOX, off, &rec, sizeof(rec));
}

void p7501_probe_cfg_load_active_hw(p7501_probe_cfg_t *out_cfg) {
    if (out_cfg == NULL) {
        return;
    }
    p7501_probe_cfg_t a;
    p7501_probe_cfg_t b;
    memcpy(&a, p7501_qspi_xip_ptr(P7501_QSPI_CFG_BANK_A_OFFSET), sizeof(a));
    memcpy(&b, p7501_qspi_xip_ptr(P7501_QSPI_CFG_BANK_B_OFFSET), sizeof(b));
    if (p7501_probe_cfg_select_ab(&a, &b, out_cfg) < 0) {
        p7501_probe_cfg_init(out_cfg, "BUILTIN");
    }
}

static void blackbox_dump_sorted(const uint32_t seqs[P7501_BLACKBOX_PAGES]) {
    uint8_t order[P7501_BLACKBOX_PAGES];
    uint8_t count = 0u;
    for (uint8_t i = 0u; i < P7501_BLACKBOX_PAGES; ++i) {
        if (seqs[i] != 0u) {
            uint8_t pos = count;
            while ((pos > 0u) && (seqs[order[pos - 1u]] > seqs[i])) {
                order[pos] = order[pos - 1u];
                --pos;
            }
            order[pos] = i;
            ++count;
        }
    }
    const uint32_t total = p7501_qspi_total_bytes();
    for (uint8_t k = 0u; k < count; ++k) {
        const uint8_t page = order[k];
        uint32_t off = 0UL;
        if (!p7501_blackbox_page_qspi_offset(total, page, &off)) {
            continue;
        }
        p7501_blackbox_rec_t rec;
        memcpy(&rec, p7501_qspi_xip_ptr(off), sizeof(rec));
        if (p7501_blackbox_verify_record(&rec)) {
            printf("  [page=%02u seq=%" PRIu32 " up=%" PRIu32 "ms] vdd=%" PRIu32 "mV drops=%" PRIu32 " ev=0x%04" PRIX32 " model=%s msg=\"%s\"\n",
                   (unsigned)page, rec.sequence, rec.uptime_ms, rec.vdd_mv,
                   rec.nsw5v_drops, rec.event_mask, rec.model_tag, rec.summary);
        }
    }
}

void p7501_cmd_blackbox(const char *arg) {
    char tok1[24] = {0};
    char tok2[48] = {0};
    const char *cur = p7501_next_token(arg, tok1, sizeof(tok1));
    if (cur != NULL) {
        (void)p7501_next_token(cur, tok2, sizeof(tok2));
    }

    if (p7501_ascii_ieq(tok1, "SNAP")) {
        const uint32_t vdd_mv = p7501_hw_read_vdd_mv();
        const uint32_t drops = p7501_hw_get_nsw5v_dropouts();
        const char *model = p7501_flash_get_target_profile()->model_name;
        const bool ok = p7501_blackbox_append_hw(
            vdd_mv, drops, 0u, model, (tok2[0] != '\0') ? tok2 : "CLI_SNAP"
        );
        if (!ok) {
            p7501_cli_set_result("ERR", "EQSPI");
        }
        printf("BLACKBOX: %s (action=SNAP vdd_mv=%" PRIu32 " drops=%" PRIu32 " model=%s)\n",
               ok ? "OK" : "ERROR", vdd_mv, drops, model);
        return;
    }

    if (p7501_ascii_ieq(tok1, "CLEAR") && p7501_ascii_ieq(tok2, "CONFIRM")) {
        const bool ok = p7501_qspi_erase_range_safe(
            P7501_QSPI_PART_BLACKBOX, P7501_QSPI_P4_BLACKBOX_OFFSET, P7501_QSPI_P4_BLACKBOX_SIZE
        );
        if (!ok) {
            p7501_cli_set_result("ERR", "EQSPI");
            printf("BLACKBOX: ERROR (erase blocked @ 0x%06" PRIX32 ")\n",
                   (uint32_t)P7501_QSPI_P4_BLACKBOX_OFFSET);
            return;
        }
        printf("BLACKBOX: OK (cleared 48 pages @ 0x%06" PRIX32 ")\n",
               (uint32_t)P7501_QSPI_P4_BLACKBOX_OFFSET);
        return;
    }

    if ((tok1[0] != '\0') && !p7501_ascii_ieq(tok1, "STATUS") && !p7501_ascii_ieq(tok1, "DUMP")) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("BLACKBOX: ERROR (Usage: BLACKBOX [STATUS|DUMP|SNAP [tag]|CLEAR CONFIRM])\n");
        return;
    }

    uint32_t seqs[P7501_BLACKBOX_PAGES];
    uint32_t valid_count = 0u;
    blackbox_scan_sequences(seqs, &valid_count);
    uint32_t next_seq = 1u;
    const uint8_t next_page = p7501_blackbox_select_next_page(seqs, &next_seq);

    printf("[BLACKBOX] part=P4_BLACKBOX off=0x%06" PRIX32 " size=%" PRIu32 " valid_pages=%" PRIu32 "/%u next_page=%u next_seq=%" PRIu32 "\n",
           (uint32_t)P7501_QSPI_P4_BLACKBOX_OFFSET,
           (uint32_t)P7501_QSPI_P4_BLACKBOX_SIZE,
           valid_count,
           (unsigned)P7501_BLACKBOX_PAGES,
           (unsigned)next_page,
           next_seq);

    if (p7501_ascii_ieq(tok1, "DUMP")) {
        blackbox_dump_sorted(seqs);
    }
    printf("BLACKBOX: OK\n");
}

void p7501_cmd_probe_cfg(const char *arg) {
    char action_tok[24] = {0};
    const char *rest = p7501_next_token(arg, action_tok, sizeof(action_tok));

    p7501_probe_cfg_t a;
    p7501_probe_cfg_t b;
    p7501_probe_cfg_t active;
    memcpy(&a, p7501_qspi_xip_ptr(P7501_QSPI_CFG_BANK_A_OFFSET), sizeof(a));
    memcpy(&b, p7501_qspi_xip_ptr(P7501_QSPI_CFG_BANK_B_OFFSET), sizeof(b));
    const int bank = p7501_probe_cfg_select_ab(&a, &b, &active);
    if (bank < 0) {
        p7501_probe_cfg_init(&active, "BUILTIN");
    }

    if (p7501_ascii_ieq(action_tok, "CAL_ADC")) {
        char gain_tok[24] = {0};
        char off_tok[24] = {0};
        const char *after_gain = (rest != NULL) ? p7501_next_token(rest, gain_tok, sizeof(gain_tok)) : NULL;
        if (after_gain != NULL) {
            (void)p7501_next_token(after_gain, off_tok, sizeof(off_tok));
        }
        uint32_t gain_ppm = P7501_ADC_GAIN_UNITY_PPM;
        if (!p7501_parse_u32_strict(gain_tok, &gain_ppm)) {
            p7501_cli_set_result("ERR", "EINVAL");
            printf("PROBE_CFG: ERROR (Usage: PROBE_CFG CAL_ADC <gain_ppm 800000..1200000> [offset_mv])\n");
            return;
        }
        int32_t offset_mv = 0;
        if (off_tok[0] != '\0') {
            const bool neg = (off_tok[0] == '-');
            uint32_t mag = 0u;
            if (!p7501_parse_u32_strict(neg ? (off_tok + 1) : off_tok, &mag) ||
                (mag > (uint32_t)P7501_ADC_OFFSET_MAX_ABS_MV)) {
                p7501_cli_set_result("ERR", "EINVAL");
                printf("PROBE_CFG: ERROR (offset_mv out of range +-500)\n");
                return;
            }
            offset_mv = neg ? -(int32_t)mag : (int32_t)mag;
        }
        active.sequence += 1u;
        active.adc0_gain_ppm = gain_ppm;
        active.adc0_offset_mv = offset_mv;
        p7501_probe_cfg_resign(&active);
        if (!p7501_probe_cfg_verify(&active)) {
            p7501_cli_set_result("ERR", "EINVAL");
            printf("PROBE_CFG: ERROR (calibration values out of bounds)\n");
            return;
        }
        const uint32_t target_off = (bank == 0) ? P7501_QSPI_CFG_BANK_B_OFFSET : P7501_QSPI_CFG_BANK_A_OFFSET;
        if (!p7501_qspi_write_sector_page(P7501_QSPI_PART_PROBE_CFG, target_off, &active, sizeof(active))) {
            p7501_cli_set_result("ERR", "EQSPI");
            printf("PROBE_CFG: ERROR (QSPI program failed)\n");
            return;
        }
        printf("PROBE_CFG: OK (bank=%s seq=%" PRIu32 " gain_ppm=%" PRIu32 " offset_mv=%" PRId32 ")\n",
               (target_off == P7501_QSPI_CFG_BANK_A_OFFSET) ? "A" : "B",
               active.sequence, active.adc0_gain_ppm, active.adc0_offset_mv);
        return;
    }

    if (p7501_ascii_ieq(action_tok, "RESET")) {
        char confirm_tok[24] = {0};
        if (rest != NULL) {
            (void)p7501_next_token(rest, confirm_tok, sizeof(confirm_tok));
        }
        if (p7501_ascii_ieq(confirm_tok, "CONFIRM")) {
            p7501_probe_cfg_init(&active, "BUILTIN");
            const bool ok_a = p7501_qspi_write_sector_page(
                P7501_QSPI_PART_PROBE_CFG, P7501_QSPI_CFG_BANK_A_OFFSET, &active, sizeof(active));
            const bool ok_b = p7501_qspi_write_sector_page(
                P7501_QSPI_PART_PROBE_CFG, P7501_QSPI_CFG_BANK_B_OFFSET, &active, sizeof(active));
            if (!ok_a || !ok_b) {
                p7501_cli_set_result("ERR", "EQSPI");
                printf("PROBE_CFG: ERROR (reset QSPI failed)\n");
                return;
            }
            printf("PROBE_CFG: OK (reset A/B banks to defaults)\n");
            return;
        }
        p7501_cli_set_result("ERR", "EINVAL");
        printf("PROBE_CFG: ERROR (Usage: PROBE_CFG RESET CONFIRM)\n");
        return;
    }

    if ((action_tok[0] != '\0') && !p7501_ascii_ieq(action_tok, "STATUS")) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("PROBE_CFG: ERROR (Usage: PROBE_CFG [STATUS|CAL_ADC <gain_ppm> [offset_mv]|RESET CONFIRM])\n");
        return;
    }

    const uint32_t raw_mv = p7501_hw_read_vdd_mv();
    const uint32_t cal_mv = p7501_probe_cfg_apply_adc_cal_mv(raw_mv, &active);
    printf("PROBE_CFG: OK (bank=%s seq=%" PRIu32 " gain_ppm=%" PRIu32 " offset_mv=%" PRId32 " raw_mv=%" PRIu32 " cal_mv=%" PRIu32 " ssot=%s)\n",
           (bank == 0) ? "A" : ((bank == 1) ? "B" : "DEFAULT"),
           active.sequence,
           active.adc0_gain_ppm,
           active.adc0_offset_mv,
           raw_mv,
           cal_mv,
           active.ssot_hash[0] ? active.ssot_hash : "BUILTIN");
}

bool p7501_recipe_load_active_hw(p7501_recipe_hdr_t *out_hdr) {
    p7501_recipe_hdr_t hdr;
    memcpy(&hdr, p7501_qspi_xip_ptr(P7501_QSPI_P3_RECIPE_OFFSET), sizeof(hdr));
    const uint8_t *payload = p7501_qspi_xip_ptr(P7501_QSPI_P3_RECIPE_OFFSET + (uint32_t)sizeof(hdr));
    const bool valid = p7501_recipe_hdr_verify(&hdr, payload, hdr.payload_bytes);
    if (valid && (out_hdr != NULL)) {
        *out_hdr = hdr;
    }
    return valid;
}

void p7501_cmd_recipe(const char *arg) {
    (void)arg;
    p7501_recipe_hdr_t hdr;
    if (p7501_recipe_load_active_hw(&hdr)) {
        printf("RECIPE: OK (part=P3_RECIPE off=0x%06" PRIX32 " valid=YES id=%s model=%s len=%" PRIu32 " crc32=0x%08" PRIX32 ")\n",
               (uint32_t)P7501_QSPI_P3_RECIPE_OFFSET,
               hdr.recipe_id,
               hdr.target_model,
               hdr.payload_bytes,
               hdr.payload_crc32);
    } else {
        printf("RECIPE: OK (part=P3_RECIPE off=0x%06" PRIX32 " valid=EMPTY)\n",
               (uint32_t)P7501_QSPI_P3_RECIPE_OFFSET);
    }
}
