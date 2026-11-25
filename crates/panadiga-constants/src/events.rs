#![deny(unsafe_code)]

//! Canonical Tauri / Frontend IPC reactive event channel names and PicoFW/DigaOS
//! protocol SSOT constants shared across `panadiga-constants`, `panadiga-core`,
//! `panadiga-desktop`, firmware C11 headers, and generated TypeScript/Python bindings.

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

/// Canonical interactive CLI serial prompt for `PicoFW` (RP2040/RP2350 probe firmware).
pub const PICOFW_SERIAL_PROMPT: &str = "PICO>";

/// Canonical interactive CLI serial prompt for `DigaOS` (DIGA target bare-metal OS).
pub const DIGAOS_SERIAL_PROMPT: &str = "DIGAOS>";

/// Canonical `PicoFW` USB CDC structured line tags shared across C11, Rust, TypeScript, and Python.
pub const PICOFW_WIRE_TAG_RES: &str = "@PICO_RES";
pub const PICOFW_WIRE_TAG_EVT: &str = "@PICO_EVT";
pub const PICOFW_WIRE_TAG_FRAME: &str = "@PICO_FRAME";
pub const PICOFW_WIRE_TAG_IDENT: &str = "@PICO_IDENT";
pub const PICOFW_WIRE_TAG_SURG: &str = "@PICO_SURG";
pub const PICOFW_WIRE_TAG_BOOT_CLASS: &str = "@PICO_BOOT_CLASS";
pub const PICOFW_WIRE_TAG_DIAG_DEEP: &str = "@PICO_DIAG_DEEP";
pub const PICOFW_WIRE_TAG_TBUS_AUDIT: &str = "@PICO_TBUS_AUDIT";
pub const PICOFW_WIRE_TAG_HOTPLUG: &str = "@PICO_HOTPLUG";
pub const PICOFW_WIRE_TAG_LIVE_MAP: &str = "@PICO_LIVE_MAP";
pub const PICOFW_WIRE_TAG_GPS_ENV: &str = "@PICO_GPS_ENV";
pub const PICOFW_WIRE_TAG_ROUTE: &str = "@PICO_ROUTE";
pub const PICOFW_WIRE_TAG_XRAY: &str = "@PICO_XRAY";
pub const PICOFW_WIRE_TAG_PROG: &str = "@PICO_PROG";
pub const PICOFW_WIRE_TAG_CHUNK: &str = "@PICO_CHUNK";
pub const PICOFW_WIRE_TAG_BGO_SEC: &str = "@PICO_BGO_SEC";

/// Exhaustive `(SYMBOL, WIRE_TAG)` registry for `PicoFW` emitted into C11, TypeScript, and Python SSOT projections.
pub const PICOFW_WIRE_TAGS: &[(&str, &str)] = &[
    ("RES", PICOFW_WIRE_TAG_RES),
    ("EVT", PICOFW_WIRE_TAG_EVT),
    ("FRAME", PICOFW_WIRE_TAG_FRAME),
    ("IDENT", PICOFW_WIRE_TAG_IDENT),
    ("SURG", PICOFW_WIRE_TAG_SURG),
    ("BOOT_CLASS", PICOFW_WIRE_TAG_BOOT_CLASS),
    ("DIAG_DEEP", PICOFW_WIRE_TAG_DIAG_DEEP),
    ("TBUS_AUDIT", PICOFW_WIRE_TAG_TBUS_AUDIT),
    ("HOTPLUG", PICOFW_WIRE_TAG_HOTPLUG),
    ("LIVE_MAP", PICOFW_WIRE_TAG_LIVE_MAP),
    ("GPS_ENV", PICOFW_WIRE_TAG_GPS_ENV),
    ("ROUTE", PICOFW_WIRE_TAG_ROUTE),
    ("XRAY", PICOFW_WIRE_TAG_XRAY),
    ("PROG", PICOFW_WIRE_TAG_PROG),
    ("CHUNK", PICOFW_WIRE_TAG_CHUNK),
    ("BGO_SEC", PICOFW_WIRE_TAG_BGO_SEC),
];

/// Exhaustive `(SYMBOL, RESULT_CODE)` registry for `@PICO_RES` / `@P7501_RES`.
pub const PICOFW_RES_CODES: &[(&str, &str)] = &[
    ("OK", "0"),
    ("EINVAL", "EINVAL"),
    ("E05", "E05"),
    ("EOFFLINE", "EOFFLINE"),
    ("ETIMEOUT", "ETIMEOUT"),
    ("EDROPOUT", "EDROPOUT"),
    ("EBUS_POLICY", "EBUS_POLICY"),
    ("EFAULT_LATCHED", "EFaultLatched"),
];

/// Legacy `@P7501_*` tag constant preserved strictly for backward-compatible log replay & parser fallback.
pub const LEGACY_P7501_WIRE_TAG_RES: &str = "@P7501_RES";
/// Transitional alias for legacy code sites pending Wave 3/4 migration.
pub const P7501_WIRE_TAG_RES: &str = LEGACY_P7501_WIRE_TAG_RES;
pub const P7501_WIRE_TAGS: &[(&str, &str)] = PICOFW_WIRE_TAGS;
pub const P7501_RES_CODES: &[(&str, &str)] = PICOFW_RES_CODES;
