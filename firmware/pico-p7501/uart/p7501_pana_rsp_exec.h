/**
 * @file p7501_pana_rsp_exec.h
 * @brief Sequence, `DUMPNOR`, JEDEC ID/CFI/checksum, and JEDEC erase/program stub executors for `PANARSP`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_PANA_RSP_EXEC_H
#define P7501_PANA_RSP_EXEC_H

#include "uart/p7501_pana_rsp.h"
#include "uart/p7501_pana_rsp_report.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

size_t panarsp_tx_sequence(
    panarsp_rx_cap_t *cap,
    const char *const *payloads,
    size_t count,
    uint32_t per_step_ms
);

size_t panarsp_execute_dumpnor(
    panarsp_rx_cap_t *cap,
    const p7501_pana_rsp_spec_t *spec,
    bool *out_hdr_seen,
    bool *out_ftr_seen,
    uint32_t *out_stream_bytes,
    uint32_t *out_ff_blocks
);

size_t panarsp_execute_jedec_id_or_sum(
    panarsp_rx_cap_t *cap,
    const p7501_pana_rsp_spec_t *spec,
    bool *out_ok,
    uint32_t *out_bytes
);

size_t panarsp_execute_jedec(
    panarsp_rx_cap_t *cap,
    const p7501_pana_rsp_spec_t *spec,
    bool *out_ok_seen,
    bool *out_to_seen,
    uint32_t *out_stream_bytes,
    const char **out_err_code
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_PANA_RSP_EXEC_H */
