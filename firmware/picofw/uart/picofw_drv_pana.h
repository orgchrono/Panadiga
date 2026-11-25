/**
 * @file picofw_drv_pana.h
 * @brief Matshita R6-MV4 Drive Factory CLI command handler (`DRVPANA`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_DRV_PANA_H
#define PICOFW_DRV_PANA_H

#ifdef __cplusplus
extern "C" {
#endif

/** CLI command `DRVPANA [ms] [NORMAL|JIG[:ms]|NOPULL] [NOPASS] <cmd...>`. */
void picofw_cmd_drv_pana(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_DRV_PANA_H */
