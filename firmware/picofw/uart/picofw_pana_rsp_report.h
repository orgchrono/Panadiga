/**
 * @file picofw_pana_rsp_report.h
 * @brief RX ring capture, drain loop, and diagnostic/register report printing for `PANARSP`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_PANA_RSP_REPORT_H
#define PICOFW_PANA_RSP_REPORT_H

#include "uart/picofw_pana_rsp.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PANARSP_RX_RAM_CAP 24576u
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
    const picofw_pana_rsp_spec_t *spec,
    const panarsp_rx_cap_t *cap,
    size_t post_tx_off,
    size_t primary_off,
    bool triggered,
    bool fifo_ovf
);
typedef void (*panarsp_post_hdr_byte_fn)(void *ctx, uint8_t b);

uint32_t panarsp_tx_frame(const char *payload);
size_t panarsp_stream_until_tokens_ex(
    panarsp_rx_cap_t *cap,
    uint32_t stub_addr,
    const uint8_t *stub,
    size_t stub_len,
    uint32_t timeout_ms,
    const char *tok_hdr,
    const char *tok_done,
    const char *tok_fail,
    uint32_t min_post_hdr_bytes,
    bool sliding_timeout,
    panarsp_post_hdr_byte_fn post_hdr_cb,
    void *post_hdr_ctx,
    bool *out_hdr_seen,
    bool *out_done_seen,
    bool *out_fail_seen,
    uint32_t *out_stream_bytes,
    uint32_t *out_post_hdr_bytes
);
size_t panarsp_stream_until_tokens(
    panarsp_rx_cap_t *cap,
    uint32_t stub_addr,
    const uint8_t *stub,
    size_t stub_len,
    uint32_t timeout_ms,
    const char *tok_hdr,
    const char *tok_done,
    const char *tok_fail,
    bool *out_hdr_seen,
    bool *out_done_seen,
    bool *out_fail_seen,
    uint32_t *out_stream_bytes
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_PANA_RSP_REPORT_H */
