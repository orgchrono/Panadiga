/**
 * @file picofw_boottrace_codec.h
 * @brief Pure C11 real-time on-chip Panasonic DMR-EH55 boot log classifier (`@PICO_BOOT_CLASS`).
 *
 * Extracts deterministic fault counters, rejected `IC7501` `0x7F` opcodes, MAIN build tag,
 * and video/BGO/HDD bring-up milestones in O(line_len) directly on RP2040/RP2350 SRAM without
 * dynamic allocation or floating-point math.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_BOOTTRACE_CODEC_H
#define PICOFW_BOOTTRACE_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum picofw_boot_verdict {
    PICOFW_BOOT_VERDICT_NO_UART = 0,
    PICOFW_BOOT_VERDICT_LOADER_ONLY = 1,
    PICOFW_BOOT_VERDICT_HDD_BRIDGE_WAIT_FAIL = 2,
    PICOFW_BOOT_VERDICT_BGO_NFROM_FAULT = 3,
    PICOFW_BOOT_VERDICT_FPC_IC7501_MISMATCH = 4,
    PICOFW_BOOT_VERDICT_CLEAN_MAIN_OK = 5
} picofw_boot_verdict_t;

typedef enum picofw_main_build {
    PICOFW_MAIN_BUILD_UNKNOWN = 0,
    PICOFW_MAIN_BUILD_3060NP = 1,
    PICOFW_MAIN_BUILD_3410PQ = 2
} picofw_main_build_t;

typedef struct picofw_boot_classifier {
    uint32_t lines_seen;
    uint16_t hdd_lba_fail_cnt;
    uint16_t nfrom_bgo_err_cnt;
    uint16_t u99_cnt;
    uint16_t odd_80040904_cnt;
    uint16_t fpc_err_cnt;
    uint16_t fpc_to_cnt;
    uint16_t iic_err_cnt;
    uint8_t op7f_bitmap[32];
    uint8_t main_build;
    bool main_entered;
    bool hdd_autoboot_seen;
    bool avout_init_ok;
    bool avout_vmute_off;
    bool tv_ntsc_seen;
    bool sys_rsv_ok;
    bool hdd_opened;
    char hw_sys[16];
    char market[16];
    char timer_ver[8];
    char cav[12];
    char sys_state[8];
    char drive_rev[12];
    uint8_t region_code;
    bool has_region;
    int8_t hdd_writable;
    uint32_t hdd_format;
    bool has_hdd_format;
    int16_t gui_mode;
    uint8_t avsel_video;
    uint8_t avsel_audio;
    bool has_avsel;
    uint8_t avin_iic_target;
    uint8_t avin_iic_input;
    uint32_t avin_iic_line;
    bool has_avin_iic;
    bool avin_iic_pending;
} picofw_boot_classifier_t;

void picofw_boot_classifier_init(picofw_boot_classifier_t *st);

void picofw_boot_classifier_feed_line(picofw_boot_classifier_t *st, const char *line);

picofw_boot_verdict_t picofw_boot_classifier_verdict(const picofw_boot_classifier_t *st);

uint32_t picofw_boot_classifier_format_op7f(
    const picofw_boot_classifier_t *st,
    char *out,
    size_t cap
);

size_t picofw_boot_classifier_format_summary(
    const picofw_boot_classifier_t *st,
    char *out,
    size_t cap
);

size_t picofw_boot_classifier_format_deep(
    const picofw_boot_classifier_t *st,
    char *out,
    size_t cap
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_BOOTTRACE_CODEC_H */
