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

/* ------------------------------------------------------------------------- *
 * Boot-cycle detector (`BOOTFOLLOW`): pure state machine, no hardware access.
 *
 * A "cycle" starts at a REAL boot event observed from the target and ends at the next boot
 * or at standby. All state derives from live pins/UART text, never from when the follower was
 * attached, so re-attaching is idempotent: a fresh `DDR CHECK: OK` banner or an XRESET falling
 * edge always restarts the relative clock at +0 ms.
 * ------------------------------------------------------------------------- */

/** Control bits as sampled by `read_ctrl3()`: bit0=NSW5V(GP3), bit1=TXD(GP4), bit2=XRESET(GP5). */
#define PICOFW_CTRL3_NSW5V  0x01u
#define PICOFW_CTRL3_XRESET 0x04u

/** First line printed by every Matsushita loader boot. */
#define PICOFW_BOOT_BANNER_LINE "DDR CHECK: OK"

typedef enum picofw_cycle_event {
    PICOFW_CYCLE_EV_NONE = 0,
    PICOFW_CYCLE_EV_START = 1,       /**< New cycle opened (see `picofw_boot_cycle_t::had_previous`). */
    PICOFW_CYCLE_EV_END = 2,         /**< Active cycle closed by standby (NSW5V falling). */
    PICOFW_CYCLE_EV_IDLE_REPORT = 3  /**< UART silent long enough: emit classifier report once. */
} picofw_cycle_event_t;

typedef enum picofw_cycle_reason {
    PICOFW_CYCLE_REASON_NONE = 0,
    PICOFW_CYCLE_REASON_ATTACH = 1,       /**< Follower attached while target already powered. */
    PICOFW_CYCLE_REASON_XRESET_FALL = 2,  /**< Real reset: XRESET falling edge. */
    PICOFW_CYCLE_REASON_WAKE_RISE = 3,    /**< Standby wake: NSW5V rising edge. */
    PICOFW_CYCLE_REASON_BOOT_BANNER = 4,  /**< `DDR CHECK: OK` seen. */
    PICOFW_CYCLE_REASON_ACTIVITY = 5,     /**< UART text while no cycle was open. */
    PICOFW_CYCLE_REASON_STANDBY_FALL = 6, /**< NSW5V falling edge (standby / power off). */
    PICOFW_CYCLE_REASON_ABORT = 7         /**< Operator aborted the follower (Ctrl+C). */
} picofw_cycle_reason_t;

typedef struct picofw_boot_cycle {
    uint32_t cycle_no;          /**< 1-based, monotonic across the follower session. */
    bool active;
    bool report_pending;        /**< Lines seen since last IDLE_REPORT / START. */
    bool had_previous;          /**< Last START replaced a still-open cycle (`prev_*` valid). */
    uint8_t prev_ctrl3;
    uint8_t start_reason;
    uint8_t end_reason;
    uint32_t lines_in_cycle;
    uint32_t idle_report_us;
    uint32_t prev_cycle_no;
    uint32_t prev_lines;
    uint64_t start_us;          /**< Absolute timer value at +0 ms of the open cycle. */
    uint64_t last_activity_us;
    uint64_t prev_start_us;
} picofw_boot_cycle_t;

/** Reset the detector; `idle_report_ms` is the UART silence that triggers one report. */
void picofw_boot_cycle_init(picofw_boot_cycle_t *cy, uint32_t idle_report_ms, uint8_t ctrl3);

/** Open cycle #1 at `now_us` (target already powered when the follower attached). */
void picofw_boot_cycle_attach(picofw_boot_cycle_t *cy, uint64_t now_us);

/** Feed a fresh control-pin sample; returns START (reset/wake), END (standby) or NONE. */
picofw_cycle_event_t picofw_boot_cycle_on_ctrl(picofw_boot_cycle_t *cy, uint64_t now_us, uint8_t ctrl3);

/**
 * Feed a completed UART line. `line_start_us` is when its first byte arrived. Must be called
 * BEFORE the line is fed to the classifier so a START can flush the previous cycle first.
 */
picofw_cycle_event_t picofw_boot_cycle_on_line(
    picofw_boot_cycle_t *cy,
    uint64_t line_start_us,
    const char *line
);

/** Periodic poll; returns IDLE_REPORT once after the UART has been silent. */
picofw_cycle_event_t picofw_boot_cycle_tick(picofw_boot_cycle_t *cy, uint64_t now_us);

/** Milliseconds since the open cycle's +0 (0 if `abs_us` precedes it). */
uint32_t picofw_boot_cycle_rel_ms(const picofw_boot_cycle_t *cy, uint64_t abs_us);

/** Microseconds since the open cycle's +0 (0 if `abs_us` precedes it). */
uint32_t picofw_boot_cycle_rel_us(const picofw_boot_cycle_t *cy, uint64_t abs_us);

const char *picofw_boot_cycle_reason_name(uint8_t reason);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_BOOTTRACE_CODEC_H */
