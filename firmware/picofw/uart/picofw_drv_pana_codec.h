/**
 * @file picofw_drv_pana_codec.h
 * @brief Portable ISO C11 Matshita R6-MV4 Drive Factory CLI (`DRVPANA`) codec.
 *
 * Host-testable with zero Pico SDK dependencies (`C11_HOST_SOURCES`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_DRV_PANA_CODEC_H
#define PICOFW_DRV_PANA_CODEC_H

#include "uart/picofw_uart_codec.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Verified Matshita R6-MV4 Drive Factory CLI unlock password command. */
#define PICOFW_DRVPANA_PASS_LINE "PASS 0x4d415453 0x48495441\r\n"

typedef struct picofw_drv_pana_spec {
    uint32_t timeout_ms;
    picofw_boot_action_t boot_action;
    uint32_t jig_hold_ms;
    bool send_pass;
    bool diag_macro;
    char cmd[96];
} picofw_drv_pana_spec_t;

/** Parse `DRVPANA [ms] [NORMAL|JIG[:ms]|NOPULL] [NOPASS] <cmd...>`. */
bool picofw_drv_pana_parse_args(const char *arg, picofw_drv_pana_spec_t *out);

/** Format `<cmd>\r\n` TX payload into `out`; returns byte count or 0 on overflow. */
size_t picofw_drv_pana_format_cmd(const picofw_drv_pana_spec_t *spec, char *out, size_t cap);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_DRV_PANA_CODEC_H */
