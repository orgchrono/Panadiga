//! Trace, capture, guided bench wizard, log decoder, RSP codec, and A/B boot CLI args.

use clap::Args;
use panadiga_core::constants::DEFAULT_BAUD_RATE;
use std::path::PathBuf;

/// Interactive guided live bench battery (Standby wake, BAUDSCAN, SW_UART & Error Dict correlation).
#[derive(Args, Debug)]
pub struct BenchWizardArgs {
    /// Serial port identifier (auto-detects connected RP2040/RP2350 Pico when omitted)
    #[arg(short, long, value_name = "PORT")]
    pub port: Option<String>,

    /// CDC baud rate (defaults to 115200)
    #[arg(short, long, default_value_t = DEFAULT_BAUD_RATE)]
    pub baud: u32,

    /// Capture window in milliseconds for Standby wakeup / BOOTTRACE
    #[arg(long, default_value_t = 12000)]
    pub window_ms: u32,

    /// Boot capture profile: passive | prejig | diagwatch | jig | all
    #[arg(long, value_name = "PROFILE")]
    pub profile: Option<String>,

    /// Standby WAKE arm timeout in ms (interactive default: 45000; 0 skips WAKE wait)
    #[arg(long, value_name = "MS")]
    pub wake_timeout_ms: Option<u32>,

    /// Hold duration (ms) for SW_UART LOW during PREJIG / JIG boot probes
    #[arg(long, default_value_t = 3000)]
    pub jig_hold_ms: u32,

    /// Run without stdin prompts (uses --vfd-cue and --active-jig flags directly)
    #[arg(long, default_value_t = false)]
    pub non_interactive: bool,

    /// Visual front-panel cue override for --non-interactive (1..9 or text)
    #[arg(long, value_name = "CUE")]
    pub vfd_cue: Option<String>,

    /// Authorize active SW_UART LOW + XRESET probe during --non-interactive run
    #[arg(long, default_value_t = false)]
    pub active_jig: bool,

    /// Destination folder for timestamped .md and .json bench reports
    #[arg(long, value_name = "OUT_DIR")]
    pub out_dir: Option<PathBuf>,
}

/// Decodes offline COM10 / AM34 UART0 / Pico capture logs (UTF-16LE or UTF-8).
#[derive(Args, Debug)]
pub struct DecodeLogArgs {
    /// Path to captured UART / Pico `.log` or `.txt` file (`-` reads stdin bytes)
    #[arg(short, long, value_name = "LOG_FILE")]
    pub file: PathBuf,

    /// Optional JSON output path (`-` emits compact JSON to stdout)
    #[arg(long, value_name = "JSON_OUT")]
    pub json: Option<PathBuf>,
}

/// Parses structured Pico CLI telemetry replies (`status`, `pincheck`, `busdiag`, `piologic`, `panarsp`, `bootclass`, `tbus-audit`, `fat-error`).
#[derive(Args, Debug)]
pub struct ParseDiagArgs {
    /// Kind: status|pincheck|busdiag|piologic|panarsp|bootclass|tbus-audit|raw-log|fat-error
    #[arg(long, value_name = "KIND")]
    pub kind: String,

    /// Captured output text file (or `-` for stdin)
    #[arg(short, long, value_name = "TEXT_FILE")]
    pub file: Option<PathBuf>,

    /// Numeric negative status code for `--kind fat-error`
    #[arg(long, allow_negative_numbers = true, value_name = "I32")]
    pub code: Option<i32>,

    /// JSON output path (`-` emits stdout JSON)
    #[arg(long, value_name = "JSON_OUT")]
    pub json: Option<PathBuf>,
}

/// Validates a P7501 bench command against Rust `SAFE_BENCH_COMMANDS` allowlist.
#[derive(Args, Debug)]
pub struct ValidateBenchCmdArgs {
    /// Raw command string to validate
    #[arg(short, long, value_name = "CMD")]
    pub cmd: String,

    /// JSON output path (`-` emits stdout JSON)
    #[arg(long, value_name = "JSON_OUT")]
    pub json: Option<PathBuf>,
}

/// Pure GDB RSP frame codec, E05 IPL NOR write guard, reply scanner, and `$g` decoder.
#[derive(Args, Debug)]
pub struct RspCodecArgs {
    /// Operation kind: checksum|is-ipl-readonly|build-frame|build-write|parse-reply|decode-hex|decode-g|validate-dump
    #[arg(long, value_name = "KIND")]
    pub kind: String,

    /// Input byte payload file (or `-` for stdin)
    #[arg(short, long, value_name = "BYTES")]
    pub input: Option<PathBuf>,

    /// Target 32-bit address in hex (for write/guard/range ops)
    #[arg(long, value_name = "HEX_VA")]
    pub addr: Option<String>,

    /// Length in bytes for read/write guard checks
    #[arg(long, default_value_t = 1)]
    pub len: usize,

    /// Chunk size in bytes for `--kind validate-dump`
    #[arg(long, default_value_t = 512)]
    pub chunk_size: usize,

    /// Optional max chunk override for `--kind validate-dump`
    #[arg(long)]
    pub max_chunk_size: Option<usize>,

    /// Optional fixed payload length hint for binary `$x` replies
    #[arg(long)]
    pub expected_len: Option<usize>,

    /// Permit mutating/execution RSP frames (`M`, `X`, `c`, `s`, `k`)
    #[arg(long, default_value_t = false)]
    pub allow_write: bool,

    /// Use binary escaped `$X` encoding for `--kind build-write`
    #[arg(long, default_value_t = false)]
    pub binary_x: bool,

    /// JSON output path (`-` emits stdout JSON)
    #[arg(long, value_name = "JSON_OUT")]
    pub json: Option<PathBuf>,
}

/// Automated A/B cold-boot statistical harness helpers (`build-cmd`, `classify`, `summarize`, `csv`).
#[derive(Args, Debug)]
pub struct AbBootArgs {
    /// Operation kind: build-cmd|classify|summarize|csv
    #[arg(long, value_name = "KIND")]
    pub kind: String,

    /// Boot trial arm mode (`NORMAL` or `JIG`)
    #[arg(long, default_value = "NORMAL", value_name = "MODE")]
    pub mode: String,

    /// Capture window milliseconds
    #[arg(long, default_value_t = 6000)]
    pub window_ms: u32,

    /// Input file (raw capture log or JSON array of runs, or `-` for stdin)
    #[arg(short, long, value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Trial run index for `--kind classify`
    #[arg(long, default_value_t = 0)]
    pub run_index: usize,

    /// Command string recorded for `--kind classify`
    #[arg(long, default_value = "", value_name = "CMD")]
    pub command: String,

    /// Measured capture duration in ms for `--kind classify`
    #[arg(long, default_value_t = 0.0)]
    pub duration_ms: f64,

    /// JSON output path (`-` emits stdout JSON)
    #[arg(long, value_name = "JSON_OUT")]
    pub json: Option<PathBuf>,
}
