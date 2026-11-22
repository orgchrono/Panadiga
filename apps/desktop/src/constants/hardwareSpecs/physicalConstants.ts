/**
 * @file constants/hardwareSpecs/physicalConstants.ts
 * @brief Immutable Physical Hardware Constants (Spansion NOR Flash, AM34 Memory Map, MEIHFS, Feistel, Pico USB).
 */

import type { HardwareThresholds, PicoUsbIdentity } from '@/models';
import { SSOT_AM34_BUS_CONSTANTS } from '@/models/generated/ssot';

/* =========================================================================
 * 1. Spansion S29JL064H NOR Flash & NVRAM Physical Constants
 * ========================================================================= */

/** Standard binary unit multiplier (IEC KiB; MiB lives in catalogTokens) */
export const KIB_BYTES = 1024;
export const BITS_PER_BYTE = 8;
export const HEX32_PAD_WIDTH = 8;
export const P7501_GND_PIN_NUMBER = 4;

/** Candidate Spansion NOR Flash sizes in bytes (2MB, 4MB, 8MB, 16MB) */
export const FLASH_2MB_BYTES = 0x0020_0000;
export const FLASH_4MB_BYTES = 0x0040_0000;
export const FLASH_16MB_BYTES = 0x0100_0000;

/** Total physical capacity of Spansion S29JL064H NOR Flash (8 MB / 64 Mbit) */
export const SPANSION_FLASH_SIZE_BYTES = 8 * KIB_BYTES * KIB_BYTES; // 8,388,608 bytes

/** Base memory offset of the top NVRAM emulation sector in 8MB dump (0x007F_0000) */
export const SPANSION_NVRAM_BASE_OFFSET = 0x007f0000;

/** Total size of the NVRAM parameter storage partition (64 KB) */
export const SPANSION_NVRAM_SIZE_BYTES = 64 * 1024; // 65,536 bytes

/** Size of each individual microsector in top-boot partition (8 KB) */
export const SPANSION_MICROSECTOR_SIZE_BYTES = 8 * 1024; // 8,192 bytes

/** Total count of 8 KB microsectors (SA134 through SA141) */
export const SPANSION_MICROSECTOR_COUNT = 8;

/** First microsector numeric index in Spansion datasheet */
export const SPANSION_MICROSECTOR_BASE_INDEX = 134;

/** Microsector index where U99 error crash logs are written */
export const SPANSION_U99_MICROSECTOR_INDEX = 141;

/** Flash erased byte value */
export const FLASH_ERASED_BYTE_VALUE = 0xff;

/* =========================================================================
 * 2. Panasonic AM34 (MN103S Core) Memory Map Architecture
 * ========================================================================= */

/** Internal SRAM Base Physical Address (256 KB) */
export const AM34_SRAM_BASE_ADDR = 0x00000000;

/** Internal SRAM Size in Bytes */
export const AM34_SRAM_SIZE_BYTES = 0x00040000; // 262,144 bytes

/** Injected RAM Loader Entrypoint Physical Address */
export const AM34_RAM_LOADER_ENTRY_ADDR = 0x00004000;

/** Flash Memory Hardware Mirror Base Address (8 MB) */
export const AM34_FLASH_BASE_ADDR = SSOT_AM34_BUS_CONSTANTS.flashCachedBaseAddr;

/** NVRAM Top-Boot Partition Base Physical Address in CPU space */
export const AM34_NVRAM_BASE_ADDR = SSOT_AM34_BUS_CONSTANTS.nvramCachedBaseAddr;

/** Panasonic UniPhier ASIC MMIO Base Physical Address */
export const AM34_MMIO_BASE_ADDR = 0xc0000000;

/** PATA / IDE Controller MMIO Base Physical Address */
export const AM34_PATA_BASE_ADDR = 0xd4000000;

/* =========================================================================
 * 3. HDD & MEIHDFS Filesystem Forensic Constants
 * ========================================================================= */

/** Standard ATA/IDE sector size in bytes */
export const LBA_SECTOR_SIZE_BYTES = 512;

/** Panasonic MEIHDFS Storage Header magic string in ASCII */
export const MEIHDFS_MBR_SIGNATURE_ASCII = 'SH';

/** MEIHDFS backup MBR mirror fixed sector offset (LBA 57,344) */
export const MEIHDFS_BACKUP_MBR_LBA = 57344;

/** MEIHDFS WAL journal ring byte offset (8 MB) and Firmware Partition 1 offset (32 MB) */
export const MEIHDFS_JOURNAL_OFFSET_BYTES = 8 * KIB_BYTES * KIB_BYTES;
export const MEIHDFS_PART1_OFFSET_BYTES = 32 * KIB_BYTES * KIB_BYTES;

/** MEIHDFS partition block unit size in bytes */
export const MEIHDFS_BLOCK_SIZE_BYTES = 2048;

/* =========================================================================
 * 4. PANA_DVD.FRM Cryptanalysis & Feistel Cipher Constants
 * ========================================================================= */

/** Known Feistel 64-bit ECB Ciphertext for 8 consecutive 0x00 bytes */
export const ECB_CIPHERTEXT_NULL_BLOCK_HEX = 'E5D1F34A518B2690';

/** Known Feistel 64-bit ECB Ciphertext for 8 consecutive 0xFF bytes (Virgin Flash) */
export const ECB_CIPHERTEXT_VIRGIN_FLASH_HEX = '035065C87BB8E090';

/** Feistel block cipher chunk size in bytes */
export const FEISTEL_BLOCK_SIZE_BYTES = 8;

/** Optical Firmware Container Magic Signature */
export const FRM_CONTAINER_MAGIC_ASCII = 'PANA_DVD_ROM';

/* =========================================================================
 * 5. Hardware Interfacing & Raspberry Pi Pico Bench Constants
 * ========================================================================= */

/** Nominal 3.3V power supply rail for Spansion and Panasonic digital logic */
export const PICO_NOMINAL_VDD_VOLTS = 3.3;

/** Minimum acceptable VDD threshold before signaling target powered / brownout (matches p7501_types.h 2.50V) */
export const PICO_MIN_VDD_THRESHOLD_VOLTS = 2.5;

/** Safe nominal LVTTL operating minimum rail (3.00V) */
export const PICO_SAFE_OPERATING_MIN_VOLTS = 3.0;

/** Maximum safe VDD ceiling (3.45V) to protect Spansion I/O pins (matches p7501_types.h 3.45V) */
export const PICO_MAX_VDD_CEILING_VOLTS = 3.45;

/** Alias kept for backwards compatibility */
export const PICO_MAX_VDD_THRESHOLD_VOLTS = PICO_MAX_VDD_CEILING_VOLTS;

/** Default serial baud rate for USB CDC UART bridge */
export const PICO_DEFAULT_BAUD_RATE = SSOT_AM34_BUS_CONSTANTS.defaultBaudRate;

/** GDB Remote Serial Protocol TCP default listening port */
export const GDB_DEFAULT_TCP_PORT = 3333;

/** GDB Remote Serial Protocol default loopback interface */
export const GDB_DEFAULT_LISTEN_ADDR = '127.0.0.1:3333';

/** Raspberry Pi USB Vendor ID (VID) */
export const RASPBERRY_PI_VID = 0x2e8a;

/** RP2040 CDC Serial Product ID (PID) */
export const PICO_RP2040_CDC_PID = 0x000a;

/** RP2040 W CYW43 CDC Serial Product ID (PID) */
export const PICO_RP2040_W_CDC_PID = 0x0014;

/** RP2350 (Pico 2) CDC Serial Product ID (PID) */
export const PICO_RP2350_CDC_PID = 0x000f;

/** RP2350 W (Pico 2W) CDC Serial Product ID (PID) */
export const PICO_RP2350_W_CDC_PID = 0x001f;

/** RP2350 RISC-V Hazard3 CDC Serial Product ID (PID) */
export const PICO_RP2350_RISCV_CDC_PID = 0x000e;

/** RP2040 / RP2350 USB BOOTSEL Mass Storage PID */
export const PICO_BOOTSEL_PID = 0x0003;

export const HARDWARE_THRESHOLDS_SPEC: HardwareThresholds = Object.freeze({
  bus_freq_hz: 1_000_000,
  nominal_vdd_volts: PICO_NOMINAL_VDD_VOLTS,
  min_vdd_threshold_volts: PICO_MIN_VDD_THRESHOLD_VOLTS,
  safe_operating_min_volts: PICO_SAFE_OPERATING_MIN_VOLTS,
  max_vdd_ceiling_volts: PICO_MAX_VDD_CEILING_VOLTS,
  default_baud_rate: PICO_DEFAULT_BAUD_RATE,
  default_gdb_socket: GDB_DEFAULT_LISTEN_ADDR,
  ram_loader_base_hex: '0x00004000',
  default_nvram_base_hex: SSOT_AM34_BUS_CONSTANTS.nvramCachedBaseHex,
});

export const PICO_USB_IDENTITY_SPEC: PicoUsbIdentity = Object.freeze({
  raspberry_pi_vid: RASPBERRY_PI_VID,
  rp2040_cdc_pid: PICO_RP2040_CDC_PID,
  rp2040_w_cdc_pid: PICO_RP2040_W_CDC_PID,
  rp2350_cdc_pid: PICO_RP2350_CDC_PID,
  rp2350_w_cdc_pid: PICO_RP2350_W_CDC_PID,
  rp2350_riscv_cdc_pid: PICO_RP2350_RISCV_CDC_PID,
  bootsel_pid: PICO_BOOTSEL_PID,
});

export const INDEXED_SERVICE_MANUALS_COUNT = 30;

/** Uniform Spansion S29JL064H main array sector size in bytes (64 KiB) */
export const SPANSION_STANDARD_SECTOR_SIZE_BYTES = 64 * 1024;

/** Fixed primary bootloader bank sector count (SA0..SA10, 256 KiB E05 IPL) */
export const SPANSION_BOOTLOADER_SECTOR_COUNT = 11;

/** Total physical Spansion S29JL064H sector geometry count (SA0..SA141: 8x8K + 126x64K + 8x8K) */
export const SPANSION_UNIFIED_TOTAL_SECTORS = 142;

/** Standard 16-bit word count spanning the 64 KiB EEMUL NVRAM aperture */
export const AM34_NVRAM_WORD_COUNT_16BIT = 16384;

/** Maximum 16-bit words per Android USB-OTG DUMP packet */
export const P7501_USB_OTG_DUMP_MAX_WORDS = 2048;

/** Maximum 16-bit words per BLE NUS DUMP packet */
export const P7501_BLE_DUMP_MAX_WORDS = 256;

/** Default P7501 live DUMP reply window in ms */
export const P7501_LIVE_DUMP_TIMEOUT_MS = 6000;

/** Wire protocol command prefix for memory word dumps */
export const P7501_DUMP_COMMAND_PREFIX = 'DUMP 0x';

/** USB device markers identifying RP2040/RP2350 BOOTSEL volumes */
export const P7501_BOOTSEL_USB_TOKEN = 'BOOTSEL';
export const P7501_BOOTSEL_PID_MARKER = 'PID:0003';

/** Hex Inspector default aperture constants */
export const HEX_INSPECTOR_MAX_IMAGE_BYTES = 100 * 1024 * 1024;
export const HEX_INSPECTOR_BYTES_PER_ROW = 16;
export const HEX_INSPECTOR_WINDOW_BYTES = LBA_SECTOR_SIZE_BYTES;

/** Terminal & SC0 Bridge rolling line retention caps */
export const SC0_BRIDGE_LOG_CAP = 200;
export const SERIAL_CONSOLE_LOG_CAP = 240;

/** Maximum IR repeat count supported by Panasonic HAL presets */
export const IR_MAX_REPEAT_COUNT = 3;

/** Safe RP2040 / RP2350 GPIO pins available for IRTX modulated output */
export const SAFE_PICO_IR_GPIOS: readonly number[] = Object.freeze([
  6, 7, 8, 9, 16, 17, 18, 19, 20, 21, 22,
]);

/** Candidate Spansion/NOR flash aperture sizes (MiB) scanned during address aliasing */
export const FLASH_CANDIDATE_SIZES_MB: readonly number[] = Object.freeze([16, 8, 4, 2]);

/** Pull-down threshold distinguishing floating vs driven 3.3V LVTTL logic pin */
export const PICO_FLOATING_PIN_VOLTS_THRESHOLD = 1.5;

/** Brownout alert floor distinguishing 0V idle/unplugged vs undervoltage fault */
export const PICO_BROWNOUT_FLOOR_VOLTS = 0.2;

/** Diagnostic Studio & serial/SC0 rolling capacities and timeouts */
export const PICO_DIAGNOSTIC_HISTORY_CAP = 30;
export const PICO_DIAGNOSTIC_DEFAULT_TIMEOUT_MS = 4000;
export const PICO_OTG_COMMAND_TIMEOUT_MS = 4000;
export const PICO_OTG_SHORT_TIMEOUT_MS = 3000;
export const PICO_STATUS_POLL_TIMEOUT_MS = 2000;
export const PICO_BLE_COMMAND_TIMEOUT_MS = 3500;
export const PICO_BLE_STATUS_TIMEOUT_MS = 2500;
export const PICO_BLE_DUMP_REPLY_TIMEOUT_MS = 5000;
export const PICO_BOOTSEL_REBOOT_WAIT_MS = 1600;
export const PICO_BOOTSEL_RECONNECT_POLL_MS = 500;
export const PICO_BOOTSEL_RECONNECT_MAX_ATTEMPTS = 12;
export const PICO_BOOTSEL_AUTOCONNECT_DELAY_MS = 400;
export const IN_CIRCUIT_LOG_REPLAY_STEP_MS = 80;
export const SC0_BRIDGE_MAX_LINES = 150;
export const SERIAL_LOG_TIMESTAMP_START = 11;
export const SERIAL_LOG_TIMESTAMP_END = 23;

/** Panasonic Kaseikyo / DIGA IR Runner SSOT constants */
export const PANASONIC_DVD_DEVICE_CODE = 0xb0;
export const PANASONIC_DEFAULT_SUBDEVICE = 0x00;
export const PANASONIC_BEACON_OBC = 0xfe;
export const IR_CAMERA_BEACON_PULSE_COUNT = 30;
export const IR_CAMERA_BEACON_INTERVAL_MS = 250;
export const IR_LONG_HOLD_THRESHOLD_MS = 1000;
export const IR_CATALOG_DEFAULT_SEARCH_LIMIT = 60;

/** Unsigned 32-bit integer upper bound & FAT16B (0x06) SD recovery card creator constants */
export const MAX_U32_VALUE = 0xffff_ffff;
export const SD_DEFAULT_VOLUME_MB = 32;
export const SD_MBR_FAT16B_TYPE_CODE = 0x06;
export const SD_MBR_FAT16B_TYPE_HEX = '0x06';
export const SD_MBR_START_LBA = 63;
export const SD_CONFIRM_TOKEN_PREFIX = 'CONFIRM-';
export const PICO_DEFAULT_FALLBACK_PORT = 'COM10';
