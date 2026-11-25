/**
 * @file test_pico_autonomous.h
 * @brief Host unit tests for autonomous Pico probe modules (`PICO_HOST_TEST=1`).
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef TEST_PICO_AUTONOMOUS_H
#define TEST_PICO_AUTONOMOUS_H

#ifdef __cplusplus
extern "C" {
#endif

/** Execute all autonomous probe host tests (push bus, Feistel, LZSS/zlib, TRAP, PAYLOAD, SECTOR, HOTPLUG, MMIO, DIAG_DEEP). */
int picofw_run_pico_autonomous_tests(void);

#ifdef __cplusplus
}
#endif

#endif /* TEST_PICO_AUTONOMOUS_H */
