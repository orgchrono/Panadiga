/**
 * @file p7501_cli_sys.h
 * @brief System, telemetry, GPIO, LED, SELFTEST, GDB, and BOOTSEL CLI command handlers.
 *
 * Extracted from `p7501_cli.c` so the command table and dispatcher remain decoupled
 * from low-level hardware control handlers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_CLI_SYS_H
#define P7501_CLI_SYS_H

#ifdef __cplusplus
extern "C" {
#endif

void p7501_cli_cmd_status(const char *arg);
void p7501_cli_cmd_jigpin(const char *arg);
void p7501_cli_cmd_led(const char *arg);
void p7501_cli_cmd_selftest(const char *arg);
void p7501_cli_cmd_gdb(const char *arg);
void p7501_cli_cmd_isolate(const char *arg);
void p7501_cli_cmd_engage(const char *arg);
void p7501_cli_cmd_reset_pulse(const char *arg);
void p7501_cli_cmd_bootsel(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_CLI_SYS_H */
