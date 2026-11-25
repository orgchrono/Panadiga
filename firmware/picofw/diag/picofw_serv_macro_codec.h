/**
 * @file picofw_serv_macro_codec.h
 * @brief Pure ISO C11 verified Panasonic DIGA Service Mode IR macro sequences (`SERVMACRO`).
 *
 * Sequences mirror the verified DMR-EH55 Service Manual tables (`SERV[01]`, `SERV[02]`,
 * `SERV[51]`, `SERV[41]`, `SERV[42]`, `SERV[65]->[01]`, `SERV[00]`) from `ir_macros_ssot.json`.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_SERV_MACRO_CODEC_H
#define PICOFW_SERV_MACRO_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum picofw_serv_macro_id {
    PICOFW_MACRO_SERVICE_MODE = 0,
    PICOFW_MACRO_SELF_CHECK = 1,
    PICOFW_MACRO_OPTICAL_CAL = 2,
    PICOFW_MACRO_ERR_HISTORY = 3,
    PICOFW_MACRO_RELEASE = 4
} picofw_serv_macro_id_t;

typedef struct picofw_serv_macro_step {
    const char *ir_preset;
    uint32_t delay_after_ms;
    uint32_t repeat;
} picofw_serv_macro_step_t;

/** Parse `SERVMACRO <SERVICE_MODE|SELF_CHECK|OPTICAL_CAL|ERR_HISTORY|RELEASE> [GPn]`. */
bool picofw_serv_macro_parse(
    const char *arg,
    picofw_serv_macro_id_t *out_id,
    uint32_t *out_gpio
);

/** Return canonical name of `id`. */
const char *picofw_serv_macro_name(picofw_serv_macro_id_t id);

/** Return read-only step sequence for `id`. */
size_t picofw_serv_macro_steps(
    picofw_serv_macro_id_t id,
    const picofw_serv_macro_step_t **out_steps
);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_SERV_MACRO_CODEC_H */
