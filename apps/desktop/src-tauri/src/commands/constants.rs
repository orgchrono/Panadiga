#![allow(dead_code)]

//! Centralized constants for Tauri IPC commands, hardware diagnostics, and desktop paths.
//!
//! Eliminates inline magic literals across `commands/` and `pico/` by re-exporting
//! core AM34/Spansion constants from `panadiga_core::constants` and defining all
//! desktop-specific IPC channels, limits, timeouts, candidate paths, and tool scripts.

pub use panadiga_core::constants::{
    AM34_NVRAM_BASE, DEFAULT_BAUD_RATE, DEFAULT_FRM_CONTAINER_PATH, DEFAULT_GDB_LISTEN_SOCKET,
    DEFAULT_HDD_EXTRACT_DIR, DEFAULT_HDD_IMAGE_PATH, PANA_DIGADIAG_FRM_SIZE,
    PANA_EXTRHEAD_DEFAULT_ADLER_STEP, SPANSION_FLASH_SIZE_BYTES,
};

// 1. Tauri IPC Reactive Event Channels & Transport Prefixes (SSOT in panadiga-core::ssot::events)
pub use panadiga_core::ssot::events::*;

pub const TRANSPORT_PREFIX_BLE: &str = "BLE";
pub const TRANSPORT_PREFIX_ANDROID_OTG: &str = "ANDROID-USB-OTG";

// 2. Live NVRAM Capture, Telemetry Stream & Serial Port Timeouts

pub const DEFAULT_NVRAM_CAPTURE_WORDS: u32 = 16_384;
pub const MIN_NVRAM_CAPTURE_WORDS: u32 = 16;
pub const MAX_NVRAM_CAPTURE_WORDS: u32 = 32_768;
pub const NVRAM_DUMP_CHUNK_WORDS: u32 = 2_048;
pub const BYTES_PER_U32_WORD: u32 = 4;

pub const TELEMETRY_WORKER_POLL_MS: u64 = 2_000;
pub const PORT_OBSERVER_POLL_MS: u64 = 1_000;
pub const VDD_ZERO_EPSILON_VOLTS: f32 = 0.01;
pub const VDD_TARGET_MIN_VOLTS: f32 = 2.50;
pub const VDD_TARGET_FLOATING_CEIL_VOLTS: f32 = 3.45;

pub const SERIAL_OPEN_TIMEOUT_MS: u64 = 1_500;
pub const SERIAL_PING_STATUS_TIMEOUT_MS: u64 = 1_500;
pub const SERIAL_RETRY_BACKOFF_MS: u64 = 40;
pub const SERIAL_UART_SYNC_TIMEOUT_MS: u64 = 800;
pub const SERIAL_AUTODETECT_TIMEOUT_MS: u64 = 1_500;
pub const SERIAL_CHUNK_READ_TIMEOUT_MS: u64 = 180;
pub const SERIAL_IDLE_COMPLETE_MS: u64 = 350;
pub const SERIAL_READ_CHUNK_BYTES: usize = 1_024;

pub const DIAG_DURATION_SLACK_MS: u64 = 2_500;
pub const DIAG_MIN_TIMEOUT_MS: u64 = 2_000;
pub const DIAG_MAX_TIMEOUT_MS: u64 = 185_000;

pub const DEFAULT_COLD_BREAKIN_MODE: u8 = 2;
pub const DEFAULT_OCD_BUS_FREQ_HZ: u32 = 1_000_000;
pub const DEFAULT_PICO_FW_FALLBACK: &str = "v2.0-P7501-UART";
pub const CHIP_ID_RP2350: &str = "RP2350-A2";
pub const CHIP_ID_RP2040: &str = "RP2040-B2";
pub const DEFAULT_PIO_UART_FRAMING_8N1_CMD: &str = "FRAMING 8N1";
pub const DEFAULT_PIO_UART_BAUD_115200_CMD: &str = "BAUD 115200";
pub const SOURCE_PICO_LIVE_NVRAM: &str = "pico-live-nvram-capture";
pub const SOURCE_PICO_DIAG_DUMP: &str = "pico-diagnostic-dump";
pub const DEFAULT_LIVE_NVRAM_LABEL: &str = "Pico Live NVRAM Capture";
pub const DEFAULT_DUMP_DIAGNOSTIC_CMD: &str = "DUMP";

pub const STATUS_REFRESH_COMMANDS: &[&str] = &[
    "STATUS",
    "WDT",
    "VDD",
    "JIGPIN",
    "PINCHECK",
    "IDENTIFY",
    "RESET",
    "AUTODETECT",
    "FLASH_SIZE",
    "ISOLATE",
    "ENGAGE",
];

pub const VALID_BREAK_IN_MODES: &[&str] = &["0", "1", "2", "3"];
pub const VALID_FLASH_SIZE_ARGS: &[&str] = &["2", "4", "8", "16"];
pub const VALID_LED_MODE_ARGS: &[&str] =
    &["IDLE", "ACTIVE", "GDB", "FAULT", "OFF", "AUTO", "STATUS"];
pub const VARIABLE_DURATION_COMMANDS: &[&str] = &[
    "SNIFF",
    "BOOTCAP",
    "UARTMON",
    "JIGBOOT",
    "UARTSTREAM",
    "HEXCAP",
    "BAUDSCAN",
    "PWRTRACE",
    "BOOTTRACE",
    "PIOLOGIC",
    "SDSNIFF",
    "PANARSP",
    "DIAGWATCH",
    "DRVPANA",
    "PWROSC",
    "WAITTX",
    "IRRX",
    "TBUSSNIFF",
];

pub const COMPLETION_MARKERS: &[&str] = &[
    "DUMP: OK",
    "PINCHECK: OK",
    "BUSDIAG: OK",
    "CFI: OK",
    "CFI: FAILED",
    "BOOTCAP: OK",
    "UARTMON: OK",
    "JIGBOOT: OK",
    "UARTSEND: OK",
    "UARTPROBE: OK",
    "BAUD: OK",
    "BAUD: ERROR",
    "FRAMING: OK",
    "FRAMING: ERROR",
    "BAUDSCAN: OK",
    "BAUDSCAN: ERROR",
    "UARTSTREAM: OK",
    "HEXCAP: OK",
    "UARTTXHEX: OK",
    "UARTTXHEX: ERROR",
    "WAITTX: OK",
    "WAITTX: ERROR",
    "PWRTRACE: OK",
    "BOOTTRACE: OK",
    "PIOLOGIC: OK",
    "PIOLOGIC: ERROR",
    "SDSNIFF: OK",
    "SDSNIFF: ERROR",
    "JIGSWEEP: OK",
    "JIGSWEEP: ERROR",
    "DIAGWATCH: OK",
    "PANARSP: OK",
    "PANARSP: ERROR",
    "DRVPANA: OK",
    "DRVPANA: ERROR",
    "IRTX: OK",
    "IRTX: ERROR",
    "IRRX: OK",
    "IRRX: ERROR",
    "IRRX: TIMEOUT",
    "SERVMACRO: OK",
    "SERVMACRO: ERROR",
    "BRIDGE: OK",
    "BRIDGE: ERROR",
    "LED: OK",
    "LED: ERROR",
    "WDT: OK",
    "FMUSCAN: OK",
    "FMUTEST: OK",
    "FMUBATCH: OK",
    "IDENTIFY: OK",
    "IDENTIFY: ERROR",
    "IDENTIFY: OFFLINE",
    "JOB: OK",
    "JOB: ERROR",
    "VAULT: OK",
    "VAULT: ERROR",
    "BLACKBOX: OK",
    "BLACKBOX: ERROR",
    "PROBE_CFG: OK",
    "PROBE_CFG: ERROR",
    "RECIPE: OK",
    "SELFTEST: OK",
    "SELFTEST: ERROR",
    "TARGET_PROFILE: OK",
    "TARGET_PROFILE: ERROR",
    "BGO_INSPECT: OK",
    "BGO_INSPECT: ERROR",
    "EEMUL_INSPECT: OK",
    "EEMUL_INSPECT: ERROR",
    "MMIO_SNAP: OK",
    "MMIO_SNAP: ERROR",
    "HOTPLUG: OK",
    "HOTPLUG: ERROR",
    "LIVE_MAP: OK",
    "LIVE_MAP: ERROR",
    "STREAM: OK",
    "STREAM: ERROR",
    "TRAP: OK",
    "TRAP: ERROR",
    "PAYLOAD: OK",
    "PAYLOAD: ERROR",
    "SECTOR: OK",
    "SECTOR: ERROR",
    "CRYPTO: OK",
    "CRYPTO: ERROR",
    "DECOMP: OK",
    "DECOMP: ERROR",
    "GPS: OK",
    "GPS: ERROR",
    "ROUTE: OK",
    "ROUTE: ERROR",
    "XRAY: OK",
    "XRAY: ERROR",
    "SURG: OK",
    "SURG: ERROR",
    "PWROSC: OK",
    "PWROSC: ERROR",
    "TBUSSNIFF: OK",
    "TBUSSNIFF: ERROR",
    "M0KEEPALIVE: OK",
    "M0KEEPALIVE: ERR",
    "M0KEEPALIVE: ERROR",
    "I2C_SCAN: OK",
    "I2C_SCAN: ERROR",
    "EEPROM: OK",
    "EEPROM: ERROR",
    "USBMODE: OK",
    "USBMODE: ERR",
    "USBMODE: ERROR",
    "BLE: OK",
    "BLE: ERR",
    "BLE_NOR_AUTH: OK",
    "BLE_NOR_AUTH: ERR",
    "BLE_NOR_AUTH: ERROR",
    "SAFETY INTERLOCK",
    "BREAK_IN: OK",
    "SNIFF: OK",
    "ISOLATE: OK",
    "ENGAGE: OK",
    "AUTODETECT: OK",
    "AUTODETECT: ERROR",
    "FLASH_SIZE: OK",
    "FLASH_SIZE: ERROR",
    "Auto-ready",
    "PONG",
];

// ============================================================================
// 3. FRM Packing, Unbrick Workbench & NOR Dumper Stub Defaults
// ============================================================================

pub const DEFAULT_DIGADIAG_DST_HEX: &str = "0x50400000";
pub const DEFAULT_DIGADIAG_TAG: &str = "FVU ";
pub const DEFAULT_DIGADIAG_BUILD: &str = "DIAG01";
pub const DEFAULT_FRM_VERSION_HEX: &str = "0x0001";
pub const DEFAULT_FRM_HW_FAMILY: &str = "M6930";
pub const DEFAULT_FRM_MODEL_GROUP: &str = "NP-309";
pub const FRM_COLLECTION_MAX_DEPTH: usize = 2;
pub const FRM_IGNORED_SUBDIR: &str = "bkp";
pub const FRM_FILE_EXTENSION: &str = "frm";
pub const FRM_UNPACK_DEFAULT_MODE: &str = "auto";
pub const FRM_WRITE_EMPTY_PATH_ERR: &str = "Caminho de saída obrigatório.";

pub const DEFAULT_BGO_PATCH_MODE: &str = "bypass";
pub const DEFAULT_MAIN_PASSTHROUGH_MODE: &str = "av_passthrough";
pub const DEFAULT_NOR_DUMP_START_HEX: &str = "0x84000000";
pub const DEFAULT_NOR_DUMP_LEN_HEX: &str = "0x800000";
pub const DEFAULT_NOR_DUMP_BLOCK_SIZE: u32 = 4_096;
pub const NOR_STUB_HEX_PREVIEW_BYTES: usize = 128;

pub const MATSHITA_DRV_PASS_UNLOCK_CMD: &str = "PASS 0x4d415453 0x48495441";
pub const SAMPLE_UNLOCK_PAYLOAD_BYTE: u8 = 0xCB;
pub const SAMPLE_UNLOCK_PAYLOAD_LEN: usize = 1_024;
pub const DEFAULT_MATSHITA_INJECT_MODE: u8 = 2;

// ============================================================================
// 4. Storage, Manuals, Workspace & Hardware Flash Profile Metadata
// ============================================================================

pub const DEFAULT_MANUAL_SEARCH_TERM: &str = "U99";
pub const DEFAULT_MANUAL_FALLBACK_NAME: &str = "manual.pdf";
pub const DEFAULT_FRM_ENTROPY_TOP_N: usize = 25;
pub const BYTES_PER_KIB_F64: f64 = 1_024.0;
pub const BYTES_PER_MIB_F64: f64 = 1_024.0 * 1_024.0;

pub const DEFAULT_HEX_DUMP_ROWS: usize = 24;
pub const MAX_HEX_DUMP_ROWS: usize = 256;
pub const HEX_DUMP_ROW_BYTES: usize = 16;
pub const HEX_DUMP_HALF_ROW_BYTES: usize = 8;

pub const FLASH_SIZE_2MB_BYTES: usize = 0x0020_0000;
pub const FLASH_SIZE_4MB_BYTES: usize = 0x0040_0000;
pub const FLASH_SIZE_8MB_BYTES: usize = SPANSION_FLASH_SIZE_BYTES;
pub const FLASH_SIZE_16MB_BYTES: usize = 0x0100_0000;

pub const FALLBACK_IC5001_FLASH_PATH: &str = "firmware_and_images/IC5001.bin";
pub const FIRMWARE_IMAGES_DIR_NAME: &str = "firmware_and_images";

pub const OVERVIEW_CORE_ARCH: &str = "32-bit AM34 / MN10300 RISC";
pub const OVERVIEW_OS_SYSTEM: &str = "Panasonic Real-Time OS + MEIHDFS v1.0";
pub const OVERVIEW_HDD_SPECS: &str = "PATA/IDE MEIHDFS (LBA 0 Master + LBA 57344 Mirror)";
pub const OVERVIEW_OPTICAL_DRIVE: &str = "DVD-RAM / DVD-RW / DVD-R Multi-Drive";
pub const OVERVIEW_DEBUG_PORT: &str =
    "P7501 (B6B-ZR-3R4 TO JIGU) Async UART 115200 8N1 (Pin 4 = GND)";
pub const OVERVIEW_ERROR_CODE: &str = "U99";
pub const OVERVIEW_ERROR_DESC: &str = "Falha de Handshake EEMUL NVRAM / Bootloader Trap";
pub const OVERVIEW_FW_VERSION: &str = "v1.18 EU / Global";

pub const CANDIDATE_MAIN_CAPACITY_PATHS: &[&str] = &[
    "firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin",
    "extracted_partitions/hdd_partition_firmware_0x02000000.bin",
    DEFAULT_HDD_IMAGE_PATH,
    "C:/ghidra/binaries/unpacked/EH55_MAIN_3410PQ.bin",
    "firmware_and_images/extracted_partitions/frm_unpacked/main3060np_unpacked.bin",
];

pub const CANDIDATE_HDD_IMAGE_PATHS: &[&str] = &[
    DEFAULT_HDD_IMAGE_PATH,
    "../../firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin",
];

pub const CANDIDATE_FRM_CONTAINER_PATHS: &[&str] = &[
    DEFAULT_FRM_CONTAINER_PATH,
    "../../firmware_and_images/PANA_DVD.FRM",
];

// 5. External Workbench Tool Launchers & Engineering Docs Policy
pub const WIN_SCRIPT_GDB: &str = "conectar_gdb_multiarch.cmd";
pub const WIN_SCRIPT_GHIDRA: &str = "iniciar_ghidra.cmd";
pub const WIN_SCRIPT_HXD: &str = "Abrir_HxD.cmd";

pub const DOCS_DIR_NAME: &str = "docs";
pub const DOCS_INDEX_FILE: &str = "INDEX.md";
pub const MAX_SRC_PREVIEW_BYTES: usize = 512 * 1024;
pub const DOC_LOCALES: &[&str] = &["en-US", "pt-BR", "audits", "pseudocode", "debug_logs"];
pub const ROOT_DOC_ALLOWLIST: &[&str] = &[
    "README.md",
    "README.pt-BR.md",
    "AUDITORIA_COMPLETA.md",
    "COMPLETE_AUDIT.md",
    "AGENTS.md",
];
pub const SRC_TOP_DIRS: &[&str] = &["crates", "firmware", "apps", "scripts", "tools"];
pub const SRC_EXTENSIONS: &[&str] = &[
    "rs", "c", "h", "pio", "py", "ts", "tsx", "css", "json", "toml", "yml", "yaml", "md", "txt",
    "log", "cmd", "sh",
];
pub const DOC_EXTENSIONS: &[&str] = &["md", "txt", "log"];
