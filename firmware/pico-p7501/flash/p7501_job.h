/**
 * @file p7501_job.h
 * @brief Runtime Atomic Probe Job executor (`JOB NVRAM_CLEAN|BGO_APPLY|VAULT_BACKUP`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef P7501_JOB_H
#define P7501_JOB_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Execute stand-alone NVRAM Vault backup into `P2_VAULT` slot `slot_index` (`1..3`). */
bool p7501_job_run_vault_backup_hw(uint8_t slot_index);

/** CLI command `JOB [STATUS|NVRAM_CLEAN CONFIRM|BGO_APPLY CONFIRM|VAULT_BACKUP [1..3]]`. */
void p7501_cmd_job(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* P7501_JOB_H */
