/**
 * @file picofw_flash_cli.c
 * @brief CLI command implementations for NOR Flash probing, CFI, SRAM loader, and EEMUL NVRAM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "flash/picofw_flash_cli.h"
#include "cli/picofw_cli.h"
#include "flash/picofw_flash.h"
#include "common/picofw_hex.h"
#include "gdb/gdb_rsp.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_led.h"

#include <inttypes.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>

void picofw_flash_cmd_autodetect(const char *arg) {
    (void)arg;
    const picofw_flash_geometry_t geo = picofw_probe_flash_geometry();
    if (geo.bytes == 0U) {
        picofw_cli_set_result("ERR", "EOFFLINE");
        printf("[AUTODETECT] ERROR: Async monitor offline; run PANARSP JEDEC_ID or FLASH_SIZE <2|4|8|16>.\n");
        printf("AUTODETECT: ERROR (offline)\n");
        return;
    }
    printf("[AUTODETECT] Passive Address Aliasing Probe: %" PRIu32 " MB NOR Flash detected (EEMUL NVRAM @ 0x%08" PRIX32 " / Uncached 0x%08" PRIX32 ").\n",
           geo.mb, geo.nvram_base, geo.hw_nvram_base);
    printf("AUTODETECT: OK (%" PRIu32 " MB)\n", geo.mb);
}

void picofw_flash_cmd_cfi(const char *arg) {
    (void)arg;
    uint32_t cfi_bytes = 0U;
    if (picofw_hw_query_cfi(&cfi_bytes)) {
        printf("[CFI] Spansion QRY valid: %" PRIu32 " bytes (%" PRIu32 " MB).\n",
               cfi_bytes, cfi_bytes / (1024UL * 1024UL));
        printf("CFI: OK (%" PRIu32 " MB)\n", cfi_bytes / (1024UL * 1024UL));
    } else {
        picofw_cli_set_result("ERR", "ECFI");
        printf("[CFI] No valid QRY response (check monitor attach & P7501 connections).\n");
        printf("CFI: FAILED\n");
    }
}

static void flash_print_geometry_summary(const char *prefix, const picofw_flash_geometry_t *geo) {
    if ((geo->nvram_size == 0UL) || (geo->nvram_end <= geo->nvram_base)) {
        printf("%s %" PRIu32 " MB (0x%08" PRIX32 ") | NVRAM: NONE (FULL_RO) | Uncached NOR: 0x%08" PRIX32 "\n",
               prefix, geo->mb, geo->bytes, geo->hw_uncached_base);
    } else {
        printf("%s %" PRIu32 " MB (0x%08" PRIX32 ") | GDB NVRAM: 0x%08" PRIX32 "..0x%08" PRIX32
               " | Uncached NOR: 0x%08" PRIX32 " (NVRAM 0x%08" PRIX32 "..0x%08" PRIX32 ")\n",
               prefix, geo->mb, geo->bytes,
               geo->nvram_base, geo->nvram_end - 1UL,
               geo->hw_uncached_base, geo->hw_nvram_base, geo->hw_nvram_end - 1UL);
    }
    if ((geo->bgo_sector_count > 0U) && (geo->bgo_hw_end > geo->bgo_hw_base)) {
        printf("[FLASH_GEO] BGO/[nfrom]: 0x%08" PRIX32 "..0x%08" PRIX32 " (%" PRIu32 " x 64 KB)\n",
               geo->bgo_hw_base, geo->bgo_hw_end - 1UL, geo->bgo_sector_count);
    }
}

void picofw_flash_cmd_size(const char *arg) {
    if ((arg == NULL) || (*arg == '\0')) {
        const picofw_flash_geometry_t geo = picofw_flash_get_geometry(gdb_rsp_get_flash_size());
        flash_print_geometry_summary("[FLASH_SIZE] Active:", &geo);
        printf("FLASH_SIZE: OK (%" PRIu32 " MB)\n", geo.mb);
        return;
    }

    uint32_t req = 0U;
    if (!picofw_parse_u32_auto(arg, &req)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("FLASH_SIZE: ERROR (Invalid size '%s'. Valid values: 2, 4, 8, 16 MB)\n", arg);
        return;
    }
    if (req <= 16U) {
        req *= 1024UL * 1024UL;
    }
    if (gdb_rsp_set_flash_size(req)) {
        const picofw_flash_geometry_t geo = picofw_flash_get_geometry(req);
        flash_print_geometry_summary("[FLASH_SIZE] Configured:", &geo);
        printf("FLASH_SIZE: OK (%" PRIu32 " MB)\n", geo.mb);
    } else {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("FLASH_SIZE: ERROR (Unsupported size. Valid values: 2, 4, 8, 16 MB)\n");
    }
}

void picofw_flash_cmd_target_profile(const char *arg) {
    if ((arg == NULL) || (*arg == '\0')) {
        const picofw_session_ctx_t *ctx = picofw_session_default_ctx();
        const picofw_ssot_target_profile_t *prof = picofw_flash_get_target_profile();
        const picofw_flash_geometry_t geo = picofw_flash_get_geometry(ctx->active_flash_size);
        printf("[TARGET_PROFILE] active=%s (%s) bytes=0x%08" PRIX32 " nvram_writable=%s upper_unlock_ok=%s full_ro=%s ro=0x%08" PRIX32 "..0x%08" PRIX32 "\n",
               prof->model_name,
               ctx->profile_selected ? "EXPLICIT" : "DEFAULT",
               geo.bytes,
               prof->writable_nvram ? "YES" : "NO",
               prof->upper_nor_unlock_allowed ? "YES" : "NO",
               prof->full_nor_readonly ? "YES" : "NO",
               prof->protected_ro_start,
               prof->protected_ro_end);
        printf("TARGET_PROFILE: OK (%s)\n", prof->model_name);
        return;
    }
    if (!picofw_flash_set_target_profile_name(arg)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("[TARGET_PROFILE] ERROR: Unknown model '%s'. Valid: DMR-EH55, DMR-ES35V, DMR-EZ485V, DMR-ES10\n", arg);
        return;
    }
    const picofw_ssot_target_profile_t *prof = picofw_flash_get_target_profile();
    (void)gdb_rsp_set_flash_size(prof->total_bytes);
    const picofw_flash_geometry_t geo = picofw_flash_get_geometry(prof->total_bytes);
    flash_print_geometry_summary("[TARGET_PROFILE] Selected:", &geo);
    printf("TARGET_PROFILE: OK (%s)\n", prof->model_name);
}
