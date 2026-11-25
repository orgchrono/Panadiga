/**
 * @file picofw_serv_macro_codec.c
 * @brief Pure ISO C11 lookup & argument parser for verified DIGA Service IR macros.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_serv_macro_codec.h"
#include "common/picofw_hex.h"
#include "diag/picofw_ir_codec.h"
#include "uart/picofw_uart_codec.h"

#include <stddef.h>

static const picofw_serv_macro_step_t k_steps_service_mode[] = {
    { "SERV", 500u, 2u },
    { "0",    450u, 1u },
    { "2",    450u, 1u },
};

static const picofw_serv_macro_step_t k_steps_self_check[] = {
    { "0",    450u, 1u },
    { "1",    900u, 1u },
    { "5",    450u, 1u },
    { "1",    900u, 1u },
    { "0",    450u, 1u },
    { "0",    450u, 1u },
};

static const picofw_serv_macro_step_t k_steps_optical_cal[] = {
    { "4",    450u, 1u },
    { "1",    900u, 1u },
    { "4",    450u, 1u },
    { "2",    900u, 1u },
    { "0",    450u, 1u },
    { "0",    450u, 1u },
};

static const picofw_serv_macro_step_t k_steps_err_history[] = {
    { "6",    450u, 1u },
    { "5",    700u, 1u },
    { "0",    450u, 1u },
    { "1",    450u, 1u },
};

static const picofw_serv_macro_step_t k_steps_release[] = {
    { "0",    450u, 1u },
    { "0",    450u, 1u },
};

typedef struct serv_macro_def {
    picofw_serv_macro_id_t id;
    const char *name;
    const char *alias;
    const picofw_serv_macro_step_t *steps;
    size_t step_count;
} serv_macro_def_t;

static const serv_macro_def_t k_macros[] = {
    { PICOFW_MACRO_SERVICE_MODE, "SERVICE_MODE", "SERV",
      k_steps_service_mode, sizeof(k_steps_service_mode) / sizeof(k_steps_service_mode[0]) },
    { PICOFW_MACRO_SELF_CHECK, "SELF_CHECK", "CHECK",
      k_steps_self_check, sizeof(k_steps_self_check) / sizeof(k_steps_self_check[0]) },
    { PICOFW_MACRO_OPTICAL_CAL, "OPTICAL_CAL", "OPTICAL",
      k_steps_optical_cal, sizeof(k_steps_optical_cal) / sizeof(k_steps_optical_cal[0]) },
    { PICOFW_MACRO_ERR_HISTORY, "ERR_HISTORY", "HIST",
      k_steps_err_history, sizeof(k_steps_err_history) / sizeof(k_steps_err_history[0]) },
    { PICOFW_MACRO_RELEASE, "RELEASE", "SERV00",
      k_steps_release, sizeof(k_steps_release) / sizeof(k_steps_release[0]) },
};

bool picofw_serv_macro_parse(
    const char *arg,
    picofw_serv_macro_id_t *out_id,
    uint32_t *out_gpio
) {
    if ((out_id == NULL) || (out_gpio == NULL)) {
        return false;
    }
    *out_id = PICOFW_MACRO_SELF_CHECK;
    *out_gpio = PICOFW_IR_DEFAULT_GPIO;
    if (arg == NULL) {
        return false;
    }

    char tok1[32];
    char tok2[24];
    const char *cur = picofw_next_token(arg, tok1, sizeof(tok1));
    if ((cur == NULL) || (tok1[0] == '\0')) {
        return false;
    }

    bool found = false;
    for (size_t i = 0u; i < (sizeof(k_macros) / sizeof(k_macros[0])); ++i) {
        if (picofw_ascii_ieq(tok1, k_macros[i].name) ||
            picofw_ascii_ieq(tok1, k_macros[i].alias)) {
            *out_id = k_macros[i].id;
            found = true;
            break;
        }
    }
    if (!found) {
        return false;
    }

    if ((picofw_next_token(cur, tok2, sizeof(tok2)) != NULL) && (tok2[0] != '\0')) {
        if (!picofw_ir_parse_gpio_token(tok2, out_gpio)) {
            return false;
        }
    }
    return true;
}

const char *picofw_serv_macro_name(picofw_serv_macro_id_t id) {
    for (size_t i = 0u; i < (sizeof(k_macros) / sizeof(k_macros[0])); ++i) {
        if (k_macros[i].id == id) {
            return k_macros[i].name;
        }
    }
    return "UNKNOWN";
}

size_t picofw_serv_macro_steps(
    picofw_serv_macro_id_t id,
    const picofw_serv_macro_step_t **out_steps
) {
    for (size_t i = 0u; i < (sizeof(k_macros) / sizeof(k_macros[0])); ++i) {
        if (k_macros[i].id == id) {
            if (out_steps != NULL) {
                *out_steps = k_macros[i].steps;
            }
            return k_macros[i].step_count;
        }
    }
    if (out_steps != NULL) {
        *out_steps = NULL;
    }
    return 0u;
}
