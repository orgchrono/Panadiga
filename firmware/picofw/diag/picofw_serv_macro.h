/**
 * @file picofw_serv_macro.h
 * @brief Runtime CLI handler for `SERVMACRO` IR service sequences.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_SERV_MACRO_H
#define PICOFW_SERV_MACRO_H

#ifdef __cplusplus
extern "C" {
#endif

/** CLI command handler for `SERVMACRO <SERVICE_MODE|SELF_CHECK|OPTICAL_CAL|ERR_HISTORY|RELEASE> [GPn]`. */
void picofw_cmd_serv_macro(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_SERV_MACRO_H */
