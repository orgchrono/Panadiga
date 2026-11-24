/**
 * @file p7501_surg_cli.h
 * @brief Pico CLI handler `SURG` for live runtime microsurgery, time-travel & MEIHFS probe.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_SURG_CLI_H
#define P7501_SURG_CLI_H

#ifdef __cplusplus
extern "C" {
#endif

/** CLI `SURG <SEARCH|DISASM|BIT|PATCH|TIMEOUT|HOOK|UNHOOK|PAUSE|CONT|JUMP|STEP|UNDO|REDO|REWIND|LOG|MEIHFS> ...`. */
void p7501_cmd_surg(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_SURG_CLI_H */
