/**
 * @file picofw_hotplug_map.h
 * @brief Non-intrusive hotplug mapper & push controller (`HOTPLUG` & `LIVE_MAP`).
 *
 * Orchestrates deterministic staged live inspection (`ELEC`, `IC3001`, `MMIO`,
 * `NOR_HDR`, `DIAG_DEEP`) over the 4 KiB SRAM push bus without resetting the target.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_HOTPLUG_MAP_H
#define PICOFW_HOTPLUG_MAP_H

#ifdef __cplusplus
extern "C" {
#endif

/** CLI `HOTPLUG [ON|OFF|STATUS]`: control asynchronous `@PICO_HOTPLUG` rail push notifications. */
void picofw_cmd_hotplug(const char *arg);

/** CLI `LIVE_MAP [QUICK|FULL|VIDEO] [ms]`: non-intrusive deterministic live target census & push. */
void picofw_cmd_live_map(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_HOTPLUG_MAP_H */
