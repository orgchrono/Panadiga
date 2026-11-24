#![deny(unsafe_code)]

//! Canonical Tauri / Frontend IPC reactive event channel names shared across
//! `panadiga-core`, `panadiga-desktop`, and generated TypeScript bindings.

pub const EVENT_PICO_SERIAL_LOG: &str = "pico://serial-log";
pub const EVENT_PICO_DIAG_RESULT: &str = "pico://diagnostic-result";
pub const EVENT_PICO_TELEMETRY: &str = "pico://telemetry";
pub const EVENT_PICO_NVRAM_CAPTURE: &str = "pico://nvram-live-capture";
pub const EVENT_PICO_PORTS_CHANGED: &str = "pico://ports-changed";
pub const EVENT_PICO_BRIDGE_DATA: &str = "pico://bridge-data";
pub const EVENT_BENCH_STREAM: &str = "panadiga://bench-event";
pub const EVENT_HDD_FLASH_PROGRESS: &str = "hdd://flash-progress";
pub const EVENT_SD_FLASH_PROGRESS: &str = "sd://flash-progress";
pub const EVENT_DVD_BURN_PROGRESS: &str = "dvd://burn-progress";
pub const EVENT_NVRAM_CLEAN_PROGRESS: &str = "nvram://clean-progress";
pub const EVENT_NOR_DUMP_PROGRESS: &str = "nor://dump-progress";
pub const EVENT_BGO_REPAIR_PROGRESS: &str = "bgo://repair-progress";
pub const EVENT_DAVIS_PROBE_RESULT: &str = "davis://probe-result";
pub const EVENT_UNBRICK_COMPLETED: &str = "unbrick://completed";
pub const EVENT_NVRAM_CLEANED: &str = "nvram://cleaned";
pub const EVENT_HDD_INSPECTED: &str = "hdd://inspected";

/// Exhaustive SSOT registry of `(RUST_SYMBOL, CHANNEL_STRING)` pairs emitted to TypeScript.
pub const ALL_IPC_EVENT_CHANNELS: &[(&str, &str)] = &[
    ("PICO_SERIAL_LOG", EVENT_PICO_SERIAL_LOG),
    ("PICO_DIAG_RESULT", EVENT_PICO_DIAG_RESULT),
    ("PICO_TELEMETRY", EVENT_PICO_TELEMETRY),
    ("PICO_NVRAM_CAPTURE", EVENT_PICO_NVRAM_CAPTURE),
    ("PICO_PORTS_CHANGED", EVENT_PICO_PORTS_CHANGED),
    ("PICO_BRIDGE_DATA", EVENT_PICO_BRIDGE_DATA),
    ("BENCH_STREAM", EVENT_BENCH_STREAM),
    ("HDD_FLASH_PROGRESS", EVENT_HDD_FLASH_PROGRESS),
    ("SD_FLASH_PROGRESS", EVENT_SD_FLASH_PROGRESS),
    ("DVD_BURN_PROGRESS", EVENT_DVD_BURN_PROGRESS),
    ("NVRAM_CLEAN_PROGRESS", EVENT_NVRAM_CLEAN_PROGRESS),
    ("NOR_DUMP_PROGRESS", EVENT_NOR_DUMP_PROGRESS),
    ("BGO_REPAIR_PROGRESS", EVENT_BGO_REPAIR_PROGRESS),
    ("DAVIS_PROBE_RESULT", EVENT_DAVIS_PROBE_RESULT),
    ("UNBRICK_COMPLETED", EVENT_UNBRICK_COMPLETED),
    ("NVRAM_CLEANED", EVENT_NVRAM_CLEANED),
    ("HDD_INSPECTED", EVENT_HDD_INSPECTED),
];

/// Canonical Pico P7501 USB CDC structured line tags shared across C11, Rust, TypeScript, and Python.
pub const P7501_WIRE_TAG_RES: &str = "@P7501_RES";
pub const P7501_WIRE_TAG_EVT: &str = "@P7501_EVT";
pub const P7501_WIRE_TAG_FRAME: &str = "@P7501_FRAME";
pub const P7501_WIRE_TAG_IDENT: &str = "@P7501_IDENT";
pub const P7501_WIRE_TAG_SURG: &str = "@P7501_SURG";
pub const P7501_WIRE_TAG_BOOT_CLASS: &str = "@P7501_BOOT_CLASS";

/// Exhaustive `(SYMBOL, WIRE_TAG)` registry emitted into C11, TypeScript, and Python SSOT projections.
pub const P7501_WIRE_TAGS: &[(&str, &str)] = &[
    ("RES", P7501_WIRE_TAG_RES),
    ("EVT", P7501_WIRE_TAG_EVT),
    ("FRAME", P7501_WIRE_TAG_FRAME),
    ("IDENT", P7501_WIRE_TAG_IDENT),
    ("SURG", P7501_WIRE_TAG_SURG),
    ("BOOT_CLASS", P7501_WIRE_TAG_BOOT_CLASS),
];

/// Exhaustive `(SYMBOL, RESULT_CODE)` registry for `@P7501_RES` emitted across C11, Rust, TypeScript, and Python.
pub const P7501_RES_CODES: &[(&str, &str)] = &[
    ("OK", "0"),
    ("EINVAL", "EINVAL"),
    ("E05", "E05"),
    ("EOFFLINE", "EOFFLINE"),
    ("ETIMEOUT", "ETIMEOUT"),
    ("EDROPOUT", "EDROPOUT"),
    ("EBUS_POLICY", "EBUS_POLICY"),
    ("EFAULT_LATCHED", "EFaultLatched"),
];
