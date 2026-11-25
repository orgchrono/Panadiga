/**
 * @file picofw_cli.h
 * @brief Interactive USB CDC command-line interface and dispatcher for Panadiga P7501 probe.
 *
 * Separates command parsing, help formatting, safety interlocks, and subsystem routing
 * from the microcontroller initialization & main polling loop.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_CLI_H
#define PICOFW_CLI_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Trim leading and trailing ASCII whitespace in-place.
 *
 * @param str Mutable string buffer (may be NULL).
 * @return Pointer to first non-whitespace character inside `str` (or NULL).
 */
char *picofw_cli_trim_whitespace(char *str);

/**
 * @brief Print startup identification banner and hardware status summary.
 */
void picofw_cli_print_banner(void);

/**
 * @brief Print complete command reference grouped by functional module.
 */
void picofw_cli_print_help(void);

/**
 * @brief Parse and execute a single null-terminated USB CDC CLI command line.
 *
 * @param line Mutable command line buffer received over USB CDC.
 */
void picofw_cli_process_command(char *line);

/** Set status/code metadata emitted by the `@PICO_RES` structured command epilogue. */
void picofw_cli_set_result(const char *status, const char *code);

/** Attach extra `key=value` fields to the pending `@PICO_RES` epilogue line. */
void picofw_cli_set_result_extra(const char *extra_kv);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_CLI_H */
