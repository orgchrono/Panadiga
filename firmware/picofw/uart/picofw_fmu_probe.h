/**
 * @file picofw_fmu_probe.h
 * @brief Panasonic Bootrom FMU handshake sweep & payload probes (`FMUSCAN`, `FMUTEST`, `FMUBATCH`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_FMU_PROBE_H
#define PICOFW_FMU_PROBE_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Sweep 256 TX bytes around the `wait 1 second.` bootrom FMU window (`FMUSCAN`).
 */
void picofw_cmd_fmu_scan(const char *arg);

/**
 * @brief Inject a custom hex payload to test FMU bootloader synchronization (`FMUTEST`).
 */
void picofw_cmd_fmu_test(const char *arg);

/**
 * @brief Probe a slice of bytes across isolated reset cycles (`FMUBATCH`).
 */
void picofw_cmd_fmu_batch(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_FMU_PROBE_H */
