/**
 * @file picofw_cli_sys.h
 * @brief System, telemetry, GPIO, LED, SELFTEST, GDB, and BOOTSEL CLI command handlers.
 *
 * Extracted from `picofw_cli.c` so the command table and dispatcher remain decoupled
 * from low-level hardware control handlers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_CLI_SYS_H
#define PICOFW_CLI_SYS_H

#include "common/picofw_probe_fsm.h"
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Dispatch a probe FSM event against a fresh hardware rail snapshot and execute resulting effects. */
bool picofw_probe_dispatch_event(picofw_ev_t ev, bool active_vdd_sample);

/** Return current probe FSM state maintained by the CLI/main shell. */
picofw_ps_t picofw_probe_get_state(void);

void picofw_cli_cmd_status(const char *arg);
void picofw_cli_cmd_jigpin(const char *arg);
void picofw_cli_cmd_led(const char *arg);
void picofw_cli_cmd_selftest(const char *arg);
void picofw_cli_cmd_gdb(const char *arg);
void picofw_cli_cmd_isolate(const char *arg);
void picofw_cli_cmd_faultack(const char *arg);
void picofw_cli_cmd_engage(const char *arg);
void picofw_cli_cmd_reset_pulse(const char *arg);
void picofw_cli_cmd_bootsel(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_CLI_SYS_H */
