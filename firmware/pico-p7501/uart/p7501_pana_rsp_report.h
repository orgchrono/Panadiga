/**
 * @file p7501_pana_rsp_report.h
 * @brief RX ring capture, drain loop, and diagnostic/register report printing for `PANARSP`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PANA_RSP_REPORT_H
#define P7501_PANA_RSP_REPORT_H

#include "uart/p7501_pana_rsp.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PANARSP_RX_RAM_CAP 4096u
#define PANARSP_PRETRIG_KEEP 512u

typedef struct panarsp_rx_cap {
    uint8_t buf[PANARSP_RX_RAM_CAP];
    size_t len;
    size_t pretrig_dropped;
    bool overflow;
} panarsp_rx_cap_t;

void panarsp_compact_pretrig(panarsp_rx_cap_t *cap, size_t keep);
void panarsp_push_rx(panarsp_rx_cap_t *cap, uint8_t b, bool compact_pretrig);
bool panarsp_drain_for_ms(
    panarsp_rx_cap_t *cap,
    uint32_t wait_ms,
    bool stop_on_rsp_frame,
    size_t inspect_from
);
void panarsp_print_report(
    const p7501_pana_rsp_spec_t *spec,
    const panarsp_rx_cap_t *cap,
    size_t post_tx_off,
    size_t primary_off,
    bool triggered,
    bool fifo_ovf
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PANA_RSP_REPORT_H */
