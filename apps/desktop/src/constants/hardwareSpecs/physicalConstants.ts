/**
 * @file constants/hardwareSpecs/physicalConstants.ts
 * @brief Immutable Physical Hardware Constants (Spansion NOR Flash, AM34 Memory Map, MEIHFS, Feistel, Pico USB).
 */

import type { HardwareThresholds, PicoUsbIdentity } from '@/models';

/* =========================================================================
 * 1. Spansion S29JL064H NOR Flash & NVRAM Physical Constants
 * ========================================================================= */

/** Total physical capacity of Spansion S29JL064H NOR Flash (8 MB / 64 Mbit) */
export const SPANSION_FLASH_SIZE_BYTES = 8 * 1024 * 1024; // 8,388,608 bytes

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
export const AM34_FLASH_BASE_ADDR = 0x80000000;

/** NVRAM Top-Boot Partition Base Physical Address in CPU space */
export const AM34_NVRAM_BASE_ADDR = 0x807f0000;

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

/** Backup MBR mirror fixed sector offset (LBA 57,344) */
export const MEIHDFS_BACKUP_MBR_LBA = 57344;

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
export const PICO_DEFAULT_BAUD_RATE = 115200;

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
  default_nvram_base_hex: '0x807F0000',
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
