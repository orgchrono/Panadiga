/**
 * @file p7501_drv_pana_codec.h
 * @brief Portable ISO C11 Matshita R6-MV4 Drive Factory CLI (`DRVPANA`) codec.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_DRV_PANA_CODEC_H
#define P7501_DRV_PANA_CODEC_H

#include "uart/p7501_uart_codec.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Verified Matshita R6-MV4 Drive Factory CLI unlock password command. */
#define P7501_DRVPANA_PASS_LINE "PASS 0x4d415453 0x48495441\r\n"

typedef struct p7501_drv_pana_spec {
    uint32_t timeout_ms;
    p7501_boot_action_t boot_action;
    uint32_t jig_hold_ms;
    bool send_pass;
    bool diag_macro;
    char cmd[96];
} p7501_drv_pana_spec_t;

/** Parse `DRVPANA [ms] [NORMAL|JIG[:ms]|NOPULL] [NOPASS] <cmd...>`. */
bool p7501_drv_pana_parse_args(const char *arg, p7501_drv_pana_spec_t *out);

/** Format `<cmd>\r\n` TX payload into `out`; returns byte count or 0 on overflow. */
size_t p7501_drv_pana_format_cmd(const p7501_drv_pana_spec_t *spec, char *out, size_t cap);

#ifdef __cplusplus
}
#endif

#endif /* P7501_DRV_PANA_CODEC_H */
