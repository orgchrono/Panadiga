/**
 * @file p7501_job.c
 * @brief Runtime executor for atomic on-probe jobs (`NVRAM_CLEAN`, `BGO_APPLY`, `VAULT_BACKUP`).
 *
 * Uses `p7501_hw_read_block_checked()` so bus errors can NEVER masquerade as `0xFF`
 * erased flash during pre-erase backups, stages into atomic A/B `P2_VAULT`, enforces
 * F14 wireless origin checks, and guarantees Upper NOR relock on ALL exit paths.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/p7501_job.h"
#include "cli/p7501_cli.h"
#include "common/p7501_cmd_arena.h"
#include "common/p7501_hex.h"
#include "flash/p7501_flash.h"
#include "flash/p7501_job_codec.h"
#include "flash/p7501_probe_cfg_codec.h"
#include "flash/p7501_qspi_storage.h"
#include "flash/p7501_vault_codec.h"
#include "hal/p7501_hw.h"
#include "hal/p7501_watchdog.h"
#include "uart/p7501_pana_jedec.h"
#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_pana_rsp_report.h"
#include "uart/p7501_uart_pio_engine.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static uint8_t s_job_stub_buf[P7501_JEDEC_MAX_STUB_SIZE];

static bool job_erase_nvram_sectors_hw(const p7501_flash_geometry_t *geo) {
    if ((geo == NULL) || (geo->nvram_size == 0UL) || (geo->sector_count == 0u)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("JOB: ERROR code=EINVAL reason=NO_WRITABLE_NVRAM_GEOMETRY\n");
        return false;
    }
    const uint32_t step = geo->nvram_size / geo->sector_count;
    panarsp_rx_cap_t *const cap = &g_p7501_cmd_arena.panarsp;
    const size_t stub_len = p7501_jedec_build_erase_range_stub(
        geo->hw_nvram_base, step, geo->sector_count,
        P7501_JEDEC_STUB_ADDR, s_job_stub_buf, sizeof(s_job_stub_buf)
    );
    if (stub_len == 0u) {
        p7501_cli_set_result("ERR", "E05");
        printf("JOB: ERROR code=E05 reason=NVRAM_RANGE_REJECTED addr=0x%08" PRIX32 " count=%" PRIu32 "\n",
               geo->hw_nvram_base, geo->sector_count);
        return false;
    }

    printf("@PROG step=1/2 msg=NVRAM_ERASE_SA%lu_COUNT_%" PRIu32 "\n",
           (unsigned long)geo->base_sector, geo->sector_count);
    fflush(stdout);

    memset(cap, 0, sizeof(*cap));
    p7501_pio_begin_rx_session(P7501_BOOT_ACTION_PASSIVE);
    p7501_pio_tx_bytes(k_p7501_rsp_handshake, P7501_RSP_HANDSHAKE_LEN, false);
    panarsp_drain_for_ms(cap, 30u, false, cap->len);

    bool hdr_unused = false;
    bool ok_seen = false;
    bool to_seen = false;
    uint32_t rx_bytes = 0u;
    (void)panarsp_stream_until_tokens(
        cap,
        P7501_JEDEC_STUB_ADDR,
        s_job_stub_buf,
        stub_len,
        15000u,
        NULL,
        P7501_JEDEC_OK_TOKEN,
        P7501_JEDEC_TO_TOKEN,
        &hdr_unused,
        &ok_seen,
        &to_seen,
        &rx_bytes
    );
    p7501_pio_end_session();

    if (!ok_seen || to_seen) {
        p7501_cli_set_result("ERR", "EJEDEC");
        printf("JOB: ERROR code=EJEDEC reason=RANGE_ERASE_FAILED base=SA%" PRIu32 " ok=%s to=%s\n",
               geo->base_sector, ok_seen ? "YES" : "NO", to_seen ? "YES" : "NO");
        return false;
    }

    printf("@PROG step=2/2 msg=NVRAM_ERASE_VERIFIED_FF\n");
    fflush(stdout);
    p7501_wdt_kick();
    return true;
}

bool p7501_job_run_vault_backup_hw(uint8_t slot_index) {
    p7501_hw_enforce_nsw5v_hiz();
    if (!p7501_hw_is_target_powered() || !p7501_hw_is_target_online()) {
        printf("[JOB] ERROR: Target offline or unpowered; vault backup aborted.\n");
        return false;
    }

    const p7501_flash_geometry_t geo = p7501_flash_get_geometry(p7501_flash_get_active_size());
    const char *model = p7501_flash_get_target_profile()->model_name;
    if (geo.nvram_size == 0UL) {
        printf("[JOB] ERROR: Model %s has no writable EEMUL NVRAM partition.\n", model);
        return false;
    }

    uint32_t snap_bytes = geo.nvram_size;
    if (snap_bytes > P7501_VAULT_SLOT_BYTES) {
        printf("[JOB] WARNING: Truncating 16MB EEMUL snapshot from %" PRIu32 " to %" PRIu32 " B vault slot.\n",
               geo.nvram_size, (uint32_t)P7501_VAULT_SLOT_BYTES);
        snap_bytes = P7501_VAULT_SLOT_BYTES;
    }

    uint8_t *const buf = g_p7501_cmd_arena.uart_rx;
    uint32_t running_crc = 0u;
    uint32_t slot_off = 0UL;
    bool vault_begun = false;
    bool all_ok = true;

    printf("@PROG step=1/3 msg=READ_TARGET_NVRAM\n");
    fflush(stdout);

    for (uint32_t base_off = 0u; base_off < snap_bytes; base_off += P7501_CMD_ARENA_BYTES) {
        uint32_t chunk = snap_bytes - base_off;
        if (chunk > P7501_CMD_ARENA_BYTES) {
            chunk = P7501_CMD_ARENA_BYTES;
        }
        /* Checked block read BEFORE touching QSPI: bus errors abort cleanly! */
        if (!p7501_hw_read_block_checked(geo.nvram_base + base_off, buf, chunk, true)) {
            printf("[JOB] ERROR: Target NVRAM read failed at offset 0x%08" PRIX32 "; aborting backup.\n",
                   base_off);
            all_ok = false;
            break;
        }
        if (!vault_begun) {
            printf("@PROG step=2/3 msg=WRITE_P2_VAULT_SLOT_%u\n", (unsigned)slot_index);
            fflush(stdout);
            if (!p7501_vault_write_slot_begin_hw(slot_index, snap_bytes, &slot_off)) {
                printf("[JOB] ERROR: Failed preparing P2_VAULT A/B inactive slot %u.\n",
                       (unsigned)slot_index);
                all_ok = false;
                break;
            }
            vault_begun = true;
        }
        running_crc = p7501_crc32_ieee_update(running_crc, buf, chunk);
        if (!p7501_vault_write_slot_chunk_hw(slot_off, base_off, buf, chunk)) {
            all_ok = false;
            break;
        }
    }

    if (all_ok && vault_begun) {
        all_ok = p7501_vault_write_slot_commit_hw(
            slot_index, geo.nvram_base, snap_bytes, running_crc, model
        );
    } else {
        all_ok = false;
    }

    printf("@PROG step=3/3 msg=%s_CRC32_0x%08" PRIX32 "\n",
           all_ok ? "VERIFY_OK" : "ABORTED", running_crc);
    (void)p7501_blackbox_append_hw(
        p7501_hw_read_vdd_mv(),
        p7501_hw_get_nsw5v_dropouts(),
        all_ok ? 1u : 0x8000u,
        model,
        all_ok ? "VAULT_BACKUP_OK" : "VAULT_BACKUP_ERR"
    );
    return all_ok;
}

void p7501_cmd_job(const char *arg) {
    p7501_job_spec_t spec;
    if (!p7501_job_parse_args(arg, &spec)) {
        p7501_cli_set_result("ERR", "EINVAL");
        printf("JOB: ERROR (Usage: JOB [STATUS|NVRAM_CLEAN CONFIRM|BGO_APPLY CONFIRM|VAULT_BACKUP [1..3]])\n");
        return;
    }

    const p7501_session_ctx_t *sess = p7501_session_default_ctx();
    p7501_wireless_auth_state_t *wireless = p7501_wireless_global_state();
    if (spec.kind == P7501_JOB_STATUS) {
        const p7501_ssot_target_profile_t *prof = p7501_flash_get_target_profile();
        printf("JOB: OK (profile=%s flash_bytes=%" PRIu32 " upper_nor=%s origin=%s nor_auth=%s)\n",
               prof->model_name,
               sess->active_flash_size,
               sess->upper_nor_unlocked ? "UNLOCKED" : "LOCKED",
               p7501_origin_name(wireless->active_origin),
               p7501_wireless_nor_write_allowed(wireless) ? "ALLOWED" : "LOCKED_E05");
        return;
    }

    const char *err_code = "0";
    const char *reason = "OK";
    if (!p7501_job_check_policy(&spec, sess, wireless, &err_code, &reason)) {
        p7501_flash_set_upper_nor_unlocked(false);
        p7501_cli_set_result("ERR", err_code);
        printf("JOB: ERROR code=%s reason=%s job=%s\n",
               err_code, reason, p7501_job_kind_name(spec.kind));
        return;
    }

    if (spec.kind == P7501_JOB_VAULT_BACKUP) {
        const bool ok = p7501_job_run_vault_backup_hw(spec.vault_slot);
        p7501_flash_set_upper_nor_unlocked(false);
        if (!ok) {
            p7501_cli_set_result("ERR", "EVAULT");
        }
        printf("JOB: %s (job=VAULT_BACKUP slot=%u)\n",
               ok ? "OK" : "ERROR", (unsigned)spec.vault_slot);
        return;
    }

    if (spec.kind == P7501_JOB_BGO_APPLY) {
        p7501_recipe_hdr_t hdr;
        (void)p7501_recipe_load_active_hw(&hdr);
        p7501_cli_set_result("ERR", "ERECIPE");
        p7501_flash_set_upper_nor_unlocked(false);
        printf("JOB: ERROR code=ERECIPE reason=NO_ONPROBE_BGO_RECIPE_USE_HOST_BGO_APPLY_LIVE\n");
        return;
    }

    /* Pre-mutation mandatory forensic backup to P2_VAULT Slot 1 (atomic A/B). */
    if (!p7501_job_run_vault_backup_hw(1u)) {
        p7501_flash_set_upper_nor_unlocked(false);
        p7501_cli_set_result("ERR", "E05");
        printf("JOB: ERROR code=E05 reason=PRE_VAULT_BACKUP_FAILED job=%s\n",
               p7501_job_kind_name(spec.kind));
        return;
    }

    const p7501_flash_geometry_t geo = p7501_flash_get_geometry(sess->active_flash_size);
    const bool erase_ok = job_erase_nvram_sectors_hw(&geo);
    p7501_wireless_consume_nor_op(wireless);
    p7501_flash_set_upper_nor_unlocked(false);
    if (!erase_ok) {
        return;
    }
    printf("JOB: OK (job=%s model=%s bytes=%" PRIu32 " relocked=YES)\n",
           p7501_job_kind_name(spec.kind),
           p7501_flash_get_target_profile()->model_name,
           geo.nvram_size);
}
