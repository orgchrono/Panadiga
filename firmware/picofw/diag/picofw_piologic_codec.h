/**
 * @file picofw_piologic_codec.h
 * @brief Pure ISO C11 logic-capture analyzer for `PIOLOGIC` (`GP2`/`GP3`/`GP4`/`GP5`).
 *
 * Computes edge counts, shortest pulse widths, and Timer-MCU (`IC7501`) / `MN103S`
 * turnaround latencies (`GP4 TXD -> GP2 RXD`, `GP5 XRESET -> RXD`) deterministically.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_PIOLOGIC_CODEC_H
#define PICOFW_PIOLOGIC_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "uart/picofw_uart_codec.h"

#ifdef __cplusplus
extern "C" {
#endif

#define PICOFW_LOGIC_CH_COUNT    4u
#define PICOFW_LOGIC_CH_RXD      0u  /* GP2 */
#define PICOFW_LOGIC_CH_NSW5V    1u  /* GP3 (Hi-Z Input only) */
#define PICOFW_LOGIC_CH_TXD      2u  /* GP4 */
#define PICOFW_LOGIC_CH_XRESET   3u  /* GP5 */

typedef struct picofw_logic_edge {
    uint32_t t_us;
    uint8_t state4;
} picofw_logic_edge_t;

typedef struct picofw_logic_stats {
    uint32_t edges[PICOFW_LOGIC_CH_COUNT];
    uint32_t rising[PICOFW_LOGIC_CH_COUNT];
    uint32_t falling[PICOFW_LOGIC_CH_COUNT];
    uint32_t min_low_us[PICOFW_LOGIC_CH_COUNT];
    uint32_t min_high_us[PICOFW_LOGIC_CH_COUNT];
    bool saw_tx_to_rx;
    uint32_t min_tx_to_rx_us;
    uint32_t max_tx_to_rx_us;
    bool saw_reset_release;
    uint32_t reset_to_first_rxd_us;
} picofw_logic_stats_t;

/** Parse `PIOLOGIC [ms] [NORMAL|JIG[:ms]|NOPULL] [sample_khz] [CTRL]`. */
void picofw_logic_parse_args(
    const char *arg,
    picofw_window_boot_spec_t *out_boot,
    uint32_t *out_sample_khz
);

/** Extended `PIOLOGIC` parser supporting `CTRL` (skip GP2 data chatter after first RXD fall). */
void picofw_logic_parse_args_ex(
    const char *arg,
    picofw_window_boot_spec_t *out_boot,
    uint32_t *out_sample_khz,
    bool *out_ctrl_only
);

/** Analyze recorded state transitions across GP2..GP5 and derive FMU/boot timing metrics. */
void picofw_logic_analyze_edges(
    const picofw_logic_edge_t *edges,
    size_t count,
    uint8_t initial_state4,
    picofw_logic_stats_t *out
);

/** CLI command `PIOLOGIC`: high-resolution simultaneous PIO0 capture on GP2/GP3/GP4/GP5. */
void picofw_cmd_piologic(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_PIOLOGIC_CODEC_H */
