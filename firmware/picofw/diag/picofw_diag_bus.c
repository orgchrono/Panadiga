/**
 * @file picofw_diag_bus.c
 * @brief P7501 boot profile trigger and asynchronous monitor aperture memory dump.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "diag/picofw_diag_bus.h"
#include "cli/picofw_cli.h"
#include "common/picofw_hex.h"
#include "flash/picofw_bus_policy.h"
#include "flash/picofw_flash.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_watchdog.h"
#include <stdio.h>
#include <stdlib.h>
#include <inttypes.h>

static const char *boot_mode_label(uint32_t mode) {
    switch (mode) {
        case PICOFW_BOOT_MODE_NORMAL_HIZ:         return "SW_UART Hi-Z, XRESET Pulse";
        case PICOFW_BOOT_MODE_SERIAL_LOW_RELEASE: return "SW_UART Low -> Hi-Z Release";
        case PICOFW_BOOT_MODE_SERIAL_LOW_HOLD:    return "SW_UART Low Hold (XFWE JIG)";
        default:                                 return "SW_UART Hi-Z Extended Settle";
    }
}

void picofw_diag_cmd_dump(const char *arg) {
    uint32_t addr = SPANSION_FLASH_UNCACHED_BASE;
    uint32_t count = 16U;

    if ((arg != NULL) && (*arg != '\0')) {
        const char *rest = NULL;
        if (!picofw_parse_hex_u32_prefix(arg, &addr, &rest)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("[DUMP] ERROR: Invalid hex start address '%s'.\n", arg);
            return;
        }
        if ((rest != NULL) && (*rest != '\0')) {
            if (!picofw_parse_u32_auto(rest, &count)) {
                picofw_cli_set_result("ERR", "EINVAL");
                printf("[DUMP] ERROR: Invalid word count '%s'.\n", rest);
                return;
            }
        }
    }
    addr &= ~0x03UL;

    if (count == 0U) {
        count = 1U;
    }
    if (count > 32768U) {
        count = 32768U;
    }

    const uint32_t total_bytes = count * 4U;
    if (picofw_bus_span_wraps_u32(addr, (size_t)total_bytes) ||
        picofw_bus_is_lethal_mmio_range(addr, total_bytes)) {
        picofw_cli_set_result("ERR", "E05");
        printf("[DUMP] ERROR E05: Range 0x%08" PRIX32 "+%" PRIu32 " wraps or overlaps lethal AM34 MMIO 0xC000100C.\n",
               addr, total_bytes);
        return;
    }

    printf("[DUMP] Reading %" PRIu32 " words (%" PRIu32 " bytes) from 0x%08" PRIX32 "...\n",
           count, total_bytes, addr);

    picofw_wdt_set_stage(PICOFW_WDT_STAGE_RX_WAIT);
    uint8_t chunk[64];
    uint32_t words_done = 0U;
    while (words_done < count) {
        if (picofw_wdt_poll_abort()) {
            picofw_cli_set_result("ERR", "EABORT");
            printf("\n[DUMP] Aborted by operator.\n");
            return;
        }
        const uint32_t batch_words = ((count - words_done) > 16U) ? 16U : (count - words_done);
        const uint32_t batch_addr = addr + (words_done * 4U);
        if (!picofw_hw_read_block_checked(batch_addr, chunk, batch_words * 4U, true)) {
            picofw_cli_set_result("ERR", "EIO");
            printf("\n[DUMP] ERROR EIO: Target bus read failed @ 0x%08" PRIX32 ".\n", batch_addr);
            return;
        }
        for (uint32_t j = 0U; j < batch_words; ++j) {
            const uint32_t idx = words_done + j;
            const uint32_t curr_addr = addr + (idx * 4U);
            const uint32_t val = picofw_read_u32_le(&chunk[j * 4U]);
            if ((idx % 4U) == 0U) {
                printf("\n%08" PRIX32 ": ", curr_addr);
            }
            printf("%08" PRIX32 " ", val);
        }
        words_done += batch_words;
    }
    printf("\n\nDUMP: OK\n");
}

void picofw_diag_cmd_boot_profile(const char *arg) {
    uint32_t mode = 0U;
    if ((arg != NULL) && (*arg != '\0')) {
        if (!picofw_parse_u32_strict(arg, &mode) || (mode >= (uint32_t)PICOFW_BOOT_MODE_COUNT)) {
            picofw_cli_set_result("ERR", "EINVAL");
            printf("BREAK_IN: ERROR (Invalid profile '%s'; expected 0..%u)\n",
                   arg, (unsigned)(PICOFW_BOOT_MODE_COUNT - 1));
            return;
        }
    }
    printf("[BREAK_IN] Executing P7501 SW_UART+XRESET profile %" PRIu32 " (%s)...\n",
           mode, boot_mode_label(mode));

    picofw_hw_boot_sequence((picofw_boot_mode_t)mode);
    printf("BREAK_IN: OK (Mode %" PRIu32 ")\n", mode);
}
