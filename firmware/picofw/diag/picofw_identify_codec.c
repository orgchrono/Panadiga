/**
 * @file picofw_identify_codec.c
 * @brief Pure ISO C11 implementation of 4-stage target identification and `@PICO_IDENT` formatting.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_identify_codec.h"
#include "common/picofw_hex.h"
#include "uart/picofw_uart_codec.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static bool buffer_contains_ascii(const uint8_t *buf, size_t len, const char *needle) {
    if ((buf == NULL) || (needle == NULL) || (needle[0] == '\0')) {
        return false;
    }
    const size_t nlen = strlen(needle);
    if (len < nlen) {
        return false;
    }
    for (size_t i = 0u; (i + nlen) <= len; ++i) {
        if (memcmp(&buf[i], needle, nlen) == 0) {
            return true;
        }
    }
    return false;
}

static void extract_loader_banner(const uint8_t *buf, size_t len, char out_banner[32]) {
    out_banner[0] = '\0';
    if ((buf == NULL) || (len == 0u)) {
        (void)snprintf(out_banner, 32u, "NONE");
        return;
    }
    for (size_t i = 0u; i < PICOFW_SSOT_LOADER_FP_COUNT; ++i) {
        const char *needle = k_picofw_ssot_loader_fingerprints[i].needle;
        if (buffer_contains_ascii(buf, len, needle)) {
            (void)snprintf(out_banner, 32u, "%s", needle);
            return;
        }
    }
    (void)snprintf(out_banner, 32u, "CAPTURED_%uB", (unsigned)len);
}

void picofw_identify_evaluate(
    uint32_t vdd_mv,
    uint32_t sw_pu_mv,
    bool nsw5v_high,
    bool rxd_high,
    bool txd_high,
    const uint8_t *uart_rx,
    size_t uart_rx_len,
    uint16_t jedec_mfg,
    uint16_t jedec_dev0,
    uint32_t ipl_crc32,
    bool apply_to_session,
    picofw_ident_report_t *out_rep
) {
    if (out_rep == NULL) {
        return;
    }
    memset(out_rep, 0, sizeof(*out_rep));
    out_rep->vdd_mv = vdd_mv;
    out_rep->sw_pu_mv = sw_pu_mv;
    out_rep->nsw5v_high = nsw5v_high;
    out_rep->jedec_mfg = jedec_mfg;
    out_rep->jedec_dev0 = jedec_dev0;
    out_rep->ipl_crc32 = ipl_crc32;
    out_rep->ssot_hash = PICOFW_SSOT_HASH_STR;
    (void)rxd_high;

    out_rep->elec_state = picofw_classify_standby_state_mv(
        vdd_mv, sw_pu_mv, nsw5v_high, txd_high, false
    );
    extract_loader_banner(uart_rx, uart_rx_len, out_rep->loader_banner);

    picofw_ssot_target_profile_id_t prof_id = picofw_session_default_ctx()->active_profile;
    out_rep->hw_board = "UNVERIFIED";
    out_rep->tv_system = "UNKNOWN";
    for (size_t i = 0u; i < PICOFW_SSOT_LOADER_FP_COUNT; ++i) {
        if (k_picofw_ssot_loader_fingerprints[i].profile_id == prof_id) {
            out_rep->hw_board = k_picofw_ssot_loader_fingerprints[i].hw_board;
            out_rep->tv_system = k_picofw_ssot_loader_fingerprints[i].tv_system;
            break;
        }
    }

    for (size_t i = 0u; i < PICOFW_SSOT_LOADER_FP_COUNT; ++i) {
        const picofw_ssot_loader_fp_t *fp = &k_picofw_ssot_loader_fingerprints[i];
        if (buffer_contains_ascii(uart_rx, uart_rx_len, fp->needle)) {
            prof_id = fp->profile_id;
            out_rep->hw_board = fp->hw_board;
            out_rep->tv_system = fp->tv_system;
            out_rep->matched_from_banner = true;
            out_rep->xip_main_ro_locked = fp->lock_xip_main_ro;
            break;
        }
    }

    if (apply_to_session && out_rep->matched_from_banner) {
        (void)picofw_flash_set_target_profile_by_id(prof_id);
    }
    out_rep->matched_profile = prof_id;

    const picofw_ssot_target_profile_t *prof =
        ((size_t)prof_id < PICOFW_TARGET_PROFILE_COUNT)
            ? &k_picofw_ssot_target_profiles[prof_id]
            : &k_picofw_ssot_target_profiles[PICOFW_TARGET_PROFILE_DMR_EH55];

    if (prof->protected_ro_end > prof->protected_ro_start) {
        out_rep->xip_main_ro_locked = true;
    }

    uint16_t dev_arr[3] = { jedec_dev0, 0u, 0u };
    const picofw_ssot_flash_chip_t *chip =
        (jedec_mfg != 0u) ? picofw_flash_find_chip_by_jedec(jedec_mfg, dev_arr, 1u) : NULL;
    if (chip == NULL) {
        chip = ((size_t)prof->flash_id < PICOFW_FLASH_ID_COUNT)
            ? &k_picofw_ssot_flash_chips[prof->flash_id]
            : picofw_flash_find_chip_by_size(prof->total_bytes);
    }

    const bool offline = (!nsw5v_high && (vdd_mv < PICOFW_SSOT_VDD_MIN_MV));
    out_rep->flash_part = (chip != NULL) ? chip->part_number : "UNKNOWN";
    out_rep->model_name = prof->model_name;
    out_rep->tier = offline
        ? "OFFLINE_UNPOWERED"
        : (out_rep->matched_from_banner ? "VERIFIED_BANNER" : "PROFILE_ACTIVE");
    if (offline) {
        out_rep->target_state = "STANDBY_OR_OFF";
        out_rep->caps_mask = 0u;
        return;
    }

    if (buffer_contains_ascii(uart_rx, uart_rx_len, "BGO driver init failed")) {
        out_rep->target_state = "U99_BGO_FAULT";
    } else {
        out_rep->target_state = "READY";
    }

    uint32_t caps = PICOFW_CAP_VAULT_BACKUP;
    if (prof->writable_nvram && !prof->full_nor_readonly) {
        caps |= PICOFW_CAP_NVRAM_WRITE;
    }
    if (prof->upper_nor_unlock_allowed && !prof->full_nor_readonly) {
        caps |= PICOFW_CAP_UPPER_NOR_UNLOCK;
    }
    if (out_rep->xip_main_ro_locked) {
        caps |= PICOFW_CAP_XIP_MAIN_RO_LOCK;
    }
    if (prof->full_nor_readonly) {
        caps |= PICOFW_CAP_FULL_NOR_RO_LOCK;
    }
    out_rep->caps_mask = caps;
}

size_t picofw_identify_format_line(
    const picofw_ident_report_t *rep,
    char *out_buf,
    size_t cap
) {
    if ((rep == NULL) || (out_buf == NULL) || (cap == 0u)) {
        return 0u;
    }
    const uint32_t jedec_packed = (((uint32_t)rep->jedec_mfg) << 16u) | ((uint32_t)rep->jedec_dev0);
    const int n = snprintf(
        out_buf,
        cap,
        "@PICO_IDENT vdd_mv=%" PRIu32 " nsw5v=%u elec=%s loader=\"%s\" jedec=0x%08" PRIX32
        " flash=%s ipl_crc32=0x%08" PRIX32 " hw=\"%s\" tv=%s state=%s model=%s tier=%s caps=0x%02" PRIX32
        " ssot=%s",
        rep->vdd_mv,
        rep->nsw5v_high ? 1u : 0u,
        (rep->elec_state != NULL) ? rep->elec_state : "UNKNOWN",
        rep->loader_banner[0] ? rep->loader_banner : "NONE",
        jedec_packed,
        (rep->flash_part != NULL) ? rep->flash_part : "UNKNOWN",
        rep->ipl_crc32,
        (rep->hw_board != NULL) ? rep->hw_board : "UNVERIFIED",
        (rep->tv_system != NULL) ? rep->tv_system : "UNKNOWN",
        (rep->target_state != NULL) ? rep->target_state : "READY",
        (rep->model_name != NULL) ? rep->model_name : "DMR-EH55",
        (rep->tier != NULL) ? rep->tier : "PROFILE_ACTIVE",
        rep->caps_mask,
        (rep->ssot_hash != NULL) ? rep->ssot_hash : PICOFW_SSOT_HASH_STR
    );
    if ((n <= 0) || ((size_t)n >= cap)) {
        out_buf[0] = '\0';
        return 0u;
    }
    return (size_t)n;
}
