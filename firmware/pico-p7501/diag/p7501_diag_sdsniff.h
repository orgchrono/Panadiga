/**
 * @file p7501_diag_sdsniff.h
 * @brief Passive Hi-Z PIO0 SD Bus Sniffer command driver (`SDSNIFF`) on GP10..GP15.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_DIAG_SDSNIFF_H
#define P7501_DIAG_SDSNIFF_H

#ifdef __cplusplus
extern "C" {
#endif

/** Handle `SDSNIFF [ms] [NORMAL|JIG[:ms]|NOPULL] [LEN:64|512]` CLI command. */
void p7501_cmd_sdsniff(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_DIAG_SDSNIFF_H */
