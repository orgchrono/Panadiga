/**
 * @file picofw_fmu_codec.h
 * @brief Pure ISO C11 functional state machine and arg parsers for FMU boot probes (`FMU*`).
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_FMU_CODEC_H
#define PICOFW_FMU_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Maximum echo response latency (us) accepted during FMUSCAN byte sweep. */
#define PICOFW_FMU_ECHO_WINDOW_US 1800u

typedef struct picofw_fmu_phase_state {
    uint8_t match_ring[24];
    size_t match_ring_len;
    bool sw_uart_flipped;
    bool sent_after_serial_msg;
    bool wait1_seen;
    bool sd_started;
    uint32_t wait1_us;
} picofw_fmu_phase_state_t;

/** Initialize clean FMU phase-tracking state machine. */
void picofw_fmu_state_init(picofw_fmu_phase_state_t *st);

/**
 * @brief Feed one RX byte into pure FMU boot-stage detector.
 *
 * Detects `"serial boot mode"`, `"DDR CHECK"`, `"wait 1 second."`, and `"SD init start"`
 * via exact rolling match without fragile last-line character heuristics.
 */
void picofw_fmu_feed_byte(
    picofw_fmu_phase_state_t *st,
    uint32_t fmode,
    uint8_t rx_byte,
    uint32_t now_us,
    bool *out_should_release_sw_uart,
    bool *out_just_saw_serial_boot,
    bool *out_just_saw_wait1
);

/**
 * @brief Pure predicate validating whether an RX byte during FMUSCAN is an active probe echo.
 *
 * Rejects late Loader output after sweep exhaustion (`bval >= 256` or `sd_started`) or outside
 * the per-byte response slot (`dt_us > PICOFW_FMU_ECHO_WINDOW_US`).
 */
bool picofw_fmu_is_echo_hit(
    uint16_t next_probe_bval,
    bool waiting_reply,
    bool sd_started,
    uint32_t dt_since_tx_us
);

/** Decode hex TX payload or default cleanly to a single `0x0D` CR byte. */
size_t picofw_fmu_decode_or_default_cr(const char *hex_str, uint8_t *out, size_t max_len);

/** Parse `FMUTEST [mode] [hex]` into single-digit mode, repeat flag, and hex slice pointer. */
void picofw_fmu_parse_test_args(
    const char *arg,
    uint32_t *out_fmode,
    bool *out_repeat_25ms,
    const char **out_hex_str
);

/** Parse `FMUBATCH [start [count [offset_ms]]]` safely within `0..255` bounds. */
void picofw_fmu_parse_batch_args(
    const char *arg,
    uint32_t *out_start,
    uint32_t *out_count,
    uint32_t *out_offset_ms
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_FMU_CODEC_H */
