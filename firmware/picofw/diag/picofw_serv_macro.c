/**
 * @file picofw_serv_macro.c
 * @brief Runtime executor for multi-step DIGA Service Mode IR macros over hardware PWM.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_serv_macro.h"
#include "cli/picofw_cli.h"
#include "diag/picofw_ir_codec.h"
#include "diag/picofw_ir_kaseikyo.h"
#include "diag/picofw_serv_macro_codec.h"
#include "hal/picofw_watchdog.h"

#include <inttypes.h>
#include <stdio.h>

void picofw_cmd_serv_macro(const char *arg) {
    picofw_serv_macro_id_t macro_id = PICOFW_MACRO_SELF_CHECK;
    uint32_t gpio = PICOFW_IR_DEFAULT_GPIO;
    if (!picofw_serv_macro_parse(arg, &macro_id, &gpio)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("SERVMACRO: ERROR (Usage: SERVMACRO <SERVICE_MODE|SELF_CHECK|OPTICAL_CAL|ERR_HISTORY|RELEASE> [GP6..9|16..22])\n");
        return;
    }

    const picofw_serv_macro_step_t *steps = NULL;
    const size_t count = picofw_serv_macro_steps(macro_id, &steps);
    if ((steps == NULL) || (count == 0u)) {
        picofw_cli_set_result("ERR", "EINVAL");
        printf("SERVMACRO: ERROR (empty macro)\n");
        return;
    }

    const char *mname = picofw_serv_macro_name(macro_id);
    printf("SERVMACRO: START (macro=%s gpio=GP%" PRIu32 " steps=%u)\n",
           mname, gpio, (unsigned)count);

    for (size_t i = 0u; i < count; ++i) {
        char cmd_buf[40];
        picofw_ir_spec_t spec;
        (void)snprintf(cmd_buf, sizeof(cmd_buf), "%s GP%" PRIu32 " %" PRIu32,
                       steps[i].ir_preset, gpio, steps[i].repeat);
        if (!picofw_ir_parse_args(cmd_buf, &spec)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("SERVMACRO: ERROR (step %u preset=%s invalid)\n",
                   (unsigned)i, steps[i].ir_preset);
            return;
        }
        printf("@PROG step=%u/%u msg=%s_%s\n",
               (unsigned)(i + 1u), (unsigned)count, mname, steps[i].ir_preset);
        picofw_ir_transmit_spec(&spec);
        picofw_wdt_sleep_ms(steps[i].delay_after_ms);
    }

    printf("SERVMACRO: OK (macro=%s steps=%u)\n", mname, (unsigned)count);
}
