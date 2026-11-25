/**
 * @file picofw_types.h
 * @brief Strict type definitions, P7501 GPIO mapping, and physical constants
 *        for the Panasonic DMR-EH55 P7501 Async UART (115200 8N1) & Boot Probe.
 *
 * Verified Physical Connector: P7501 (JST B6B-ZR-3R4, "TO JIGU", Main PCB NA p.55/62-64)
 * Target Digital Board:       VEP79138K via P59001 Interconnect (NA p.97)
 * Target Processor Family:    Matsushita UniPhier AM34 / MN103S
 * Host Controller:            Universal RP2 Family (RP2040 / RP2350 Dual-Core)
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#ifndef PICOFW_TYPES_H
#define PICOFW_TYPES_H

#include <stdint.h>
#include <stdbool.h>
#include "generated/picofw_ssot.h"

#ifdef __cplusplus
extern "C" {
#endif

/* =========================================================================
 * Centralized Compile-Time Platform Capability Flags
 * ========================================================================= */

#if defined(PICO_RP2350) && (PICO_RP2350 != 0)
    #define PICOFW_IS_RP2350_BUILD 1
#else
    #define PICOFW_IS_RP2350_BUILD 0
#endif

#if defined(PICO_RP2350_RISCV) && (PICO_RP2350_RISCV != 0)
    #define PICOFW_IS_RISCV_BUILD 1
#else
    #define PICOFW_IS_RISCV_BUILD 0
#endif

#if defined(PICOFW_BLE_ENABLED) && (PICOFW_BLE_ENABLED != 0)
    #define PICOFW_HAS_BLE_BUILD 1
#else
    #define PICOFW_HAS_BLE_BUILD 0
#endif

/* =========================================================================
 * Physical GPIO Pin Allocations (RP2040 / RP2350 -> Panasonic P7501)
 * ========================================================================= */

/**
 * @brief Host GPIO pin assignments mapped 1:1 to Panasonic P7501 (B6B-ZR-3R4).
 *
 * Electrical Safety Invariant:
 *   - GP3 (Pin 5 NSW5V / X_SW_5R2V) connects to a ~5.15V upstream rail through
 *     D7510 and MUST ALWAYS remain High-Z Input with internal pulls disabled.
 *   - GP5 (Pin 2 XRESET) is strictly Open-Drain (drive 0V LOW, release to Hi-Z).
 *   - GP2 (Pin 6 UARTP2M_T) is always input RX from the Main SoC.
 */
enum p7501_gpio_pin {
    P7501_PIN_RXD         = 2,  /**< GP2 (P7501 Pin 6 UARTP2M_T): 115200 8N1 RX (Target -> Pico) */
    P7501_PIN_NSW5V       = 3,  /**< GP3 (P7501 Pin 5 NSW5V): Rail Sense ONLY (Keep Hi-Z Input!) */
    P7501_PIN_TXD         = 4,  /**< GP4 (P7501 Pin 3 UARTM2P_T): 115200 8N1 TX (Pico -> Target) */
    P7501_PIN_XRESET      = 5,  /**< GP5 (P7501 Pin 2 XRESET): Active-Low Reset (Open-Drain ONLY) */
    PICOFW_PIN_SD_CLK      = 10, /**< GP10 (Optional SDSNIFF Hi-Z): SD_CLK rising-edge clock */
    PICOFW_PIN_SD_CMD      = 11, /**< GP11 (Optional SDSNIFF Hi-Z): SD_CMD (sample bit 0) */
    PICOFW_PIN_SD_DAT0     = 12, /**< GP12 (Optional SDSNIFF Hi-Z): SD_DAT0 (sample bit 1) */
    PICOFW_PIN_SD_DAT1     = 13, /**< GP13 (Optional SDSNIFF Hi-Z): SD_DAT1 (sample bit 2) */
    PICOFW_PIN_SD_DAT2     = 14, /**< GP14 (Optional SDSNIFF Hi-Z): SD_DAT2 (sample bit 3) */
    PICOFW_PIN_SD_DAT3     = 15, /**< GP15 (Optional SDSNIFF Hi-Z): SD_DAT3 (sample bit 4) */
    P7501_PIN_SW_UART_ADC = 26, /**< GP26 (P7501 Pin 1 SW_UART / ADC0): Serial Boot Mux & VDD Sense */
};
typedef enum p7501_gpio_pin p7501_gpio_pin_t;

/**
 * @brief ADC Input Channel index corresponding to P7501_PIN_SW_UART_ADC (GP26).
 */
enum picofw_adc_channel {
    PICOFW_ADC_CH_VDD = 0, /**< ADC Channel 0 corresponds to GPIO 26 */
};
typedef enum picofw_adc_channel picofw_adc_channel_t;

/* =========================================================================
 * Electrical and Temporal Constants
 * ========================================================================= */

/** Nominal P7501 asynchronous UART baud rate (115200 8N1). */
#define PICOFW_UART_BAUD_RATE         115200UL

/** PIO UART state machine oversampling rate (8 cycles per bit = 921.6 kHz). */
#define PICOFW_PIO_UART_CYCLES_PER_BIT 8UL

/** Threshold voltage below which target SW_UART sense is unpowered/disconnected. */
#define PICOFW_VDD_THRESHOLD_V        2.5f

/** Standby wake-edge rising threshold (V) monitored by `DIAGWATCH WAKE[:ms]`. */
#define PICOFW_VDD_WAKE_THRESHOLD_V   2.0f

/** Default Standby wake wait window (ms) when `WAKE` is given without explicit duration. */
#define PICOFW_WAKE_DEFAULT_WAIT_MS   30000U

/**
 * @brief Pull-down discrimination floor on GP26 (ADC0).
 *
 * Connected P7501 Pin 1 clamps around 2.68-2.70 V under RP2 internal pull-down
 * (~50 kOhm against Q7510/R7562/R7559), while a floating or unpowered wire drops
 * below 1.50 V (~0.00 V).
 */
#define PICOFW_VDD_PULLDOWN_PRESENT_V 1.50f

/**
 * @brief Upper sanity boundary for SW_UART ADC0 sense (3.3V VREF domain).
 *
 * Floating-pin discrimination inside picofw_hw_read_vdd() engages internal
 * pull-down briefly: an unconnected GP26 collapses to ~0 V, while connected P7501
 * Pin 1 clamps around 2.68-2.70 V (via Q7510/R7562/R7559) or reads ~3.25 V Hi-Z.
 */
#define PICOFW_VDD_FLOATING_CEIL_V    3.45f

/** Settling time (us) after engaging GP26 pull-down before sampling ADC0. */
#define PICOFW_ADC_PULLDOWN_SETTLE_US 300U

/** Number of ADC samples averaged per VDD sense reading. */
#define PICOFW_ADC_SAMPLE_COUNT       8U

/** RP2040/RP2350 ADC reference voltage (3.3V VREG/AVDD). */
#define PICOFW_ADC_VREF               3.3f

/** RP2040/RP2350 12-bit SAR ADC full-scale resolution (2^12). */
#define PICOFW_ADC_RESOLUTION         4096.0f

/** Target open-drain XRESET hold time (ms) for clean cold reboot. */
#define PICOFW_RESET_ASSERT_MS        10U

/** Extended diagnostic open-drain XRESET hold time (ms) for PIO/UART sync captures. */
#define PICOFW_RESET_DIAG_ASSERT_MS   15U

/** Pre-reset SW_UART settle window (ms) before asserting XRESET. */
#define PICOFW_RESET_POST_SETTLE_MS   5U

/** Settle window (us) between SW_UART setup and XRESET release. */
#define PICOFW_RESET_SETTLE_US        10U

/** Post-reset settle window (us). */
#define PICOFW_POST_RESET_DELAY_US    200U

/** Default JIG SW_UART LOW hold window (ms) matching Loader serial-boot strap latency. */
#define PICOFW_JIG_DEFAULT_HOLD_MS    2500U

/** Default / maximum capture & tracer window boundaries (ms). */
#define PICOFW_CAPTURE_DEFAULT_MS     7500U
#define PICOFW_CAPTURE_MAX_MS         120000U
#define PICOFW_TRACE_MAX_MS           90000U
#define PICOFW_FMU_PROBE_WINDOW_MS    4000U
#define PICOFW_FMU_BATCH_WINDOW_MS    2000U
#define PICOFW_UART_PROBE_WINDOW_MS   4500U

/** Fail-closed PIO TX FIFO wait ceiling (us) preventing infinite TX blocking. */
#define PICOFW_TX_FIFO_TIMEOUT_US     500000UL

/** Critical Boot IPL window (`SA0..SA10` = first 256 KiB: `0x80000000..0x8003FFFF`). */
#define PICOFW_FLASH_CRITICAL_IPL_SIZE    0x00040000UL

/** Standard Spansion main uniform sector size (64 KiB = 65536 bytes). */
#define SPANSION_MAIN_SECTOR_SIZE_BYTES  65536UL

/** Target AM34 internal SRAM workspace base for injected async stubs. */
#define AM34_SRAM_LOADER_BASE_ADDR       0x00004000UL

/** Physical uncached CS0/SB0 and uncached hardware flash apertures (projected from Rust SSOT). */
#define SPANSION_FLASH_UNCACHED_BASE     PICOFW_SSOT_CS0_CACHED_BASE
#define SPANSION_FLASH_HW_UNCACHED_BASE  PICOFW_SSOT_CS0_UNCACHED_BASE
#define SPANSION_MICROSECTOR_SIZE_BYTES  0x00002000UL
#define SPANSION_NVRAM_SECTOR_COUNT      8U
#define SPANSION_NVRAM_BASE_OFFSET       0x007F0000UL
#define SPANSION_NVRAM_END_OFFSET        0x00800000UL
#define SPANSION_JEDEC_UNLOCK_ADDR1      (SPANSION_FLASH_UNCACHED_BASE + 0x0AAAUL)
#define SPANSION_JEDEC_UNLOCK_ADDR2      (SPANSION_FLASH_UNCACHED_BASE + 0x0554UL)
#define SPANSION_CMD_UNLOCK1             0x00AAU
#define SPANSION_CMD_UNLOCK2             0x0055U
#define SPANSION_CMD_ERASE_SETUP         0x0080U
#define SPANSION_CMD_SECTOR_ERASE        0x0030U
#define SPANSION_CMD_READ_RESET          0x00F0U
#define SPANSION_POLL_DQ7_MASK           0x0080U
#define SPANSION_ERROR_DQ5_MASK          0x0020U

/* =========================================================================
 * Discrete P7501 Boot & Reset Sequence Profiles (SW_UART + XRESET)
 * ========================================================================= */

/**
 * @brief Boot control sequences coordinating P7501 Pin 1 (SW_UART) & Pin 2 (XRESET).
 */
enum picofw_boot_mode {
    /** Mode 0: Normal Boot — SW_UART Hi-Z + open-drain XRESET pulse (200 us settle). */
    PICOFW_BOOT_MODE_NORMAL_HIZ         = 0,
    /** Mode 1: Serial Mux Pulse — SW_UART LOW during XRESET, released to Hi-Z after reset. */
    PICOFW_BOOT_MODE_SERIAL_LOW_RELEASE = 1,
    /** Mode 2: JIG Serial Boot Hold — SW_UART held LOW across XRESET (XFWE serial boot). */
    PICOFW_BOOT_MODE_SERIAL_LOW_HOLD    = 2,
    /** Mode 3: Normal Hi-Z Extended Settle — SW_UART Hi-Z + extended post-reset wait. */
    PICOFW_BOOT_MODE_HIZ_EXTENDED       = 3,
    /** Total number of discrete P7501 boot profiles. */
    PICOFW_BOOT_MODE_COUNT              = 4,
};
typedef enum picofw_boot_mode picofw_boot_mode_t;

/* =========================================================================
 * P7501 Monitor / Bus Response Constants
 * ========================================================================= */

/** Idle high line response (pull-up idle, no monitor responder driving low). */
#define PICOFW_BUS_RESP_OPEN_HIGH     0xFFFFFFFFUL

/** Grounded/shorted response. */
#define PICOFW_BUS_RESP_OPEN_LOW      0x00000000UL

/** Monitor probe sweep bounds. */
#define PICOFW_PROBE_MIN              0x00U
#define PICOFW_PROBE_MAX              0xFFU

/** Logical monitor protocol opcodes (forwarded over P7501 UART monitor when attached). */
#define PICOFW_MON_CMD_NOP            0x00U
#define PICOFW_MON_CMD_READ_DCSR      0x01U
#define PICOFW_MON_CMD_WRITE_DCSR     0x02U
#define PICOFW_MON_CMD_READ_MEM32     0x10U
#define PICOFW_MON_CMD_WRITE_MEM32    0x11U
#define PICOFW_MON_CMD_READ_PC        0x20U
#define PICOFW_MON_CMD_WRITE_PC       0x21U
#define PICOFW_MON_CMD_RESUME         0x30U
#define PICOFW_MON_CMD_HALT           0x31U

/* =========================================================================
 * Return Status Codes
 * ========================================================================= */

enum picofw_status {
    PICOFW_OK                   = 0,  /**< Operation completed successfully */
    PICOFW_ERR_INVALID_PARAM    = -1, /**< Invalid parameter passed to function */
    PICOFW_ERR_TARGET_UNPOWERED = -2, /**< Target SW_UART/VDD voltage below threshold */
    PICOFW_ERR_PIO_INIT_FAILED  = -3, /**< Failed to allocate PIO UART state machine */
    PICOFW_ERR_TIMEOUT          = -4, /**< Target response timed out */
    PICOFW_ERR_VERIFY_FAILED    = -5, /**< Memory verification mismatch */
};
typedef enum picofw_status picofw_status_t;

#ifdef __cplusplus
}
#endif

#endif /* PICOFW_TYPES_H */
