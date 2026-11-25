/**
 * @file picofw_flash_cli.h
 * @brief Flash geometry probe, CFI query, SRAM loader injection, and NVRAM erase CLI handlers.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_FLASH_CLI_H
#define PICOFW_FLASH_CLI_H

#ifdef __cplusplus
extern "C" {
#endif

void picofw_flash_cmd_autodetect(const char *arg);
void picofw_flash_cmd_cfi(const char *arg);
void picofw_flash_cmd_size(const char *arg);
void picofw_flash_cmd_target_profile(const char *arg);

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_FLASH_CLI_H */
