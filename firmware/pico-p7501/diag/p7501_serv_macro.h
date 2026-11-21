/**
 * @file p7501_serv_macro.h
 * @brief Runtime CLI handler for `SERVMACRO` IR service sequences.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_SERV_MACRO_H
#define P7501_SERV_MACRO_H

#ifdef __cplusplus
extern "C" {
#endif

/** CLI command handler for `SERVMACRO <SERVICE_MODE|SELF_CHECK|OPTICAL_CAL|ERR_HISTORY|RELEASE> [GPn]`. */
void p7501_cmd_serv_macro(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_SERV_MACRO_H */
