/**
 * @file p7501_serv_macro_codec.h
 * @brief Pure ISO C11 verified Panasonic DIGA Service Mode IR macro sequences (`SERVMACRO`).
 *
 * Sequences mirror the verified DMR-EH55 Service Manual tables (`SERV[01]`, `SERV[02]`,
 * `SERV[51]`, `SERV[41]`, `SERV[42]`, `SERV[65]->[01]`, `SERV[00]`) from `ir_macros_ssot.json`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_SERV_MACRO_CODEC_H
#define P7501_SERV_MACRO_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum p7501_serv_macro_id {
    P7501_MACRO_SERVICE_MODE = 0,
    P7501_MACRO_SELF_CHECK = 1,
    P7501_MACRO_OPTICAL_CAL = 2,
    P7501_MACRO_ERR_HISTORY = 3,
    P7501_MACRO_RELEASE = 4
} p7501_serv_macro_id_t;

typedef struct p7501_serv_macro_step {
    const char *ir_preset;
    uint32_t delay_after_ms;
    uint32_t repeat;
} p7501_serv_macro_step_t;

/** Parse `SERVMACRO <SERVICE_MODE|SELF_CHECK|OPTICAL_CAL|ERR_HISTORY|RELEASE> [GPn]`. */
bool p7501_serv_macro_parse(
    const char *arg,
    p7501_serv_macro_id_t *out_id,
    uint32_t *out_gpio
);

/** Return canonical name of `id`. */
const char *p7501_serv_macro_name(p7501_serv_macro_id_t id);

/** Return read-only step sequence for `id`. */
size_t p7501_serv_macro_steps(
    p7501_serv_macro_id_t id,
    const p7501_serv_macro_step_t **out_steps
);

#ifdef __cplusplus
}
#endif

#endif /* P7501_SERV_MACRO_CODEC_H */
