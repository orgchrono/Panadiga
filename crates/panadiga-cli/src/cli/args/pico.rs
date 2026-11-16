//! CLI argument definitions for RP2040/RP2350 Pico hardware control and GDB RSP bridge.

use clap::{Args, Subcommand};
use panadiga_core::constants::{
    DEFAULT_BAUD_RATE, DEFAULT_GDB_LISTEN_PORT, DEFAULT_GDB_LISTEN_SOCKET, DEFAULT_SERIAL_PORT,
};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct PicoArgs {
    #[command(subcommand)]
    pub action: PicoActions,
}

#[derive(Subcommand, Debug)]
pub enum PicoActions {
    /// Lists all attached serial COM ports and identifies RP2040 vs RP2350 Pico 2 hardware
    List,

    /// Probes a Pico board, verifies communication, and reads target 3.3V VDD rail voltage
    Ping {
        /// Serial port identifier (e.g. COM10 or /dev/ttyACM0)
        #[arg(short, long, value_name = "PORT")]
        port: String,

        /// Baud rate (defaults to 115200)
        #[arg(short, long, default_value_t = DEFAULT_BAUD_RATE)]
        baud: u32,
    },

    /// Queries Pico live status, VDD voltage, and AM34 OCD DCSR register state (HALT vs CDDIAG/Running)
    Status {
        /// Serial port identifier (e.g. COM10 or /dev/ttyACM0)
        #[arg(short, long, default_value = DEFAULT_SERIAL_PORT, value_name = "PORT")]
        port: String,

        /// Baud rate (defaults to 115200)
        #[arg(short, long, default_value_t = DEFAULT_BAUD_RATE)]
        baud: u32,
    },

    /// Reads and decodes AM34 Debug Control/Status Register (DCSR) bits without altering NVRAM
    Dcsr {
        /// Serial port identifier (e.g. COM10 or /dev/ttyACM0)
        #[arg(short, long, default_value = DEFAULT_SERIAL_PORT, value_name = "PORT")]
        port: String,

        /// Baud rate (defaults to 115200)
        #[arg(short, long, default_value_t = DEFAULT_BAUD_RATE)]
        baud: u32,
    },

    /// Executes P7501 discrete boot/reset profile mode (SW_UART+XRESET; 0=Hi-Z, 1=Serial Pulse, 2=JIG Hold LOW, 3=Extended Hi-Z)
    #[command(name = "breakin", alias = "break-in")]
    BreakIn {
        /// Serial port identifier (e.g. COM10 or /dev/ttyACM0)
        #[arg(short, long, default_value = DEFAULT_SERIAL_PORT, value_name = "PORT")]
        port: String,

        /// Cold Break-In mode (0..3, default 2)
        #[arg(short, long, default_value_t = 2)]
        mode: u8,

        /// Baud rate (defaults to 115200)
        #[arg(short, long, default_value_t = DEFAULT_BAUD_RATE)]
        baud: u32,
    },

    /// Reads 32-bit little-endian words in-circuit from AM34 memory/Flash/NVRAM via Pico DUMP
    Dump {
        /// Serial port identifier (e.g. COM10 or /dev/ttyACM0)
        #[arg(short, long, default_value = DEFAULT_SERIAL_PORT, value_name = "PORT")]
        port: String,

        /// Start bus address in hex (e.g. 0x807F0000 for NVRAM or 0x80000000 for Spansion IPL)
        #[arg(short, long, default_value = "0x807F0000", value_name = "HEX_ADDR")]
        addr: String,

        /// Number of 32-bit words to read (1..=16384, default 64 words = 256 bytes)
        #[arg(short, long, default_value_t = 64)]
        words: usize,

        /// Optional output file path to store raw captured binary bytes
        #[arg(short, long, value_name = "FILE")]
        out: Option<PathBuf>,

        /// Baud rate (defaults to 115200)
        #[arg(short, long, default_value_t = DEFAULT_BAUD_RATE)]
        baud: u32,
    },

    /// Starts a TCP-to-Serial GDB Remote Serial Protocol (RSP) bridge server
    GdbBridge {
        /// Serial port identifier connected to Raspberry Pi Pico (e.g. COM10)
        #[arg(short, long, value_name = "PORT")]
        port: String,

        /// Serial baud rate (defaults to 115200)
        #[arg(short, long, default_value_t = DEFAULT_BAUD_RATE)]
        baud: u32,

        /// TCP listener address and port for incoming GDB connections
        #[arg(short, long, default_value = DEFAULT_GDB_LISTEN_SOCKET)]
        listen: String,
    },

    /// Automatically flashes the Panadiga P7501 Async UART firmware to a detected Raspberry Pi Pico (RP2040/RP2350)
    Flash {
        /// Optional serial port of a running Pico to trigger 1200-baud touch reset into BOOTSEL mode
        #[arg(short, long, value_name = "PORT")]
        port: Option<String>,

        /// Path to custom .uf2 firmware file (defaults to scanning dist/, firmware_bins, or embedded UF2)
        #[arg(short, long, value_name = "UF2_FILE")]
        file: Option<PathBuf>,
    },

    /// Merges RP2040 and RP2350 .uf2 images into an RP2350-E10 compliant universal UF2
    #[command(name = "merge-uf2", alias = "merge_uf2")]
    MergeUf2 {
        /// Path to compiled RP2040 UF2 firmware
        #[arg(long, value_name = "RP2040_UF2")]
        rp2040: PathBuf,

        /// Path to compiled RP2350 UF2 firmware
        #[arg(long, value_name = "RP2350_UF2")]
        rp2350: PathBuf,

        /// Destination path for unified universal UF2
        #[arg(short, long, value_name = "OUT_UF2")]
        out: PathBuf,
    },

    /// Decodes offline COM10 / AM34 UART0 / Pico capture logs (UTF-16LE or UTF-8)
    #[command(name = "decode-log", alias = "decode_log")]
    DecodeLog {
        /// Path to captured UART / Pico `.log` or `.txt` file (`-` reads stdin bytes)
        #[arg(short, long, value_name = "LOG_FILE")]
        file: PathBuf,

        /// Optional JSON output path (`-` emits compact JSON to stdout)
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },

    /// Parses structured Pico CLI telemetry replies (`status`, `pincheck`, `busdiag`, `piologic`, `panarsp`, `fat-error`)
    #[command(name = "parse-diag", alias = "parse_diag")]
    ParseDiag {
        /// Kind: status|pincheck|busdiag|piologic|panarsp|fat-error
        #[arg(long, value_name = "KIND")]
        kind: String,

        /// Captured output text file (or `-` for stdin)
        #[arg(short, long, value_name = "TEXT_FILE")]
        file: Option<PathBuf>,

        /// Numeric negative status code for `--kind fat-error`
        #[arg(long, allow_negative_numbers = true, value_name = "I32")]
        code: Option<i32>,

        /// JSON output path (`-` emits stdout JSON)
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },

    /// Validates a P7501 bench command against Rust `SAFE_BENCH_COMMANDS` allowlist
    #[command(name = "validate-bench-cmd", alias = "validate_bench_cmd")]
    ValidateBenchCmd {
        /// Raw command string to validate
        #[arg(short, long, value_name = "CMD")]
        cmd: String,

        /// JSON output path (`-` emits stdout JSON)
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },

    /// Pure GDB RSP frame codec, E05 IPL NOR write guard, reply scanner, and `$g` decoder
    #[command(name = "rsp-codec", alias = "rsp_codec")]
    RspCodec {
        /// Operation kind: checksum|is-ipl-readonly|build-frame|build-write|parse-reply|decode-hex|decode-g|validate-dump
        #[arg(long, value_name = "KIND")]
        kind: String,

        /// Input byte payload file (or `-` for stdin)
        #[arg(short, long, value_name = "BYTES")]
        input: Option<PathBuf>,

        /// Target 32-bit address in hex (for write/guard/range ops)
        #[arg(long, value_name = "HEX_VA")]
        addr: Option<String>,

        /// Length in bytes for read/write guard checks
        #[arg(long, default_value_t = 1)]
        len: usize,

        /// Chunk size in bytes for `--kind validate-dump`
        #[arg(long, default_value_t = 512)]
        chunk_size: usize,

        /// Optional max chunk override for `--kind validate-dump`
        #[arg(long)]
        max_chunk_size: Option<usize>,

        /// Optional fixed payload length hint for binary `$x` replies
        #[arg(long)]
        expected_len: Option<usize>,

        /// Permit mutating/execution RSP frames (`M`, `X`, `c`, `s`, `k`)
        #[arg(long, default_value_t = false)]
        allow_write: bool,

        /// Use binary escaped `$X` encoding for `--kind build-write`
        #[arg(long, default_value_t = false)]
        binary_x: bool,

        /// JSON output path (`-` emits stdout JSON)
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },

    /// Automated A/B cold-boot statistical harness helpers (`build-cmd`, `classify`, `summarize`, `csv`)
    #[command(name = "ab-boot", alias = "ab_boot")]
    AbBoot {
        /// Operation kind: build-cmd|classify|summarize|csv
        #[arg(long, value_name = "KIND")]
        kind: String,

        /// Boot trial arm mode (`NORMAL` or `JIG`)
        #[arg(long, default_value = "NORMAL", value_name = "MODE")]
        mode: String,

        /// Capture window milliseconds
        #[arg(long, default_value_t = 6000)]
        window_ms: u32,

        /// Input file (raw capture log or JSON array of runs, or `-` for stdin)
        #[arg(short, long, value_name = "INPUT")]
        input: Option<PathBuf>,

        /// Trial run index for `--kind classify`
        #[arg(long, default_value_t = 0)]
        run_index: usize,

        /// Command string recorded for `--kind classify`
        #[arg(long, default_value = "", value_name = "CMD")]
        command: String,

        /// Measured capture duration in ms for `--kind classify`
        #[arg(long, default_value_t = 0.0)]
        duration_ms: f64,

        /// JSON output path (`-` emits stdout JSON)
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },
}

#[derive(Args, Debug)]
pub struct GdbConfigArgs {
    /// Serial port name (e.g. COM10, /dev/ttyACM0)
    #[arg(short, long, default_value = DEFAULT_SERIAL_PORT, value_name = "PORT")]
    pub port: String,

    /// TCP port for GDB remote protocol
    #[arg(short, long, default_value_t = DEFAULT_GDB_LISTEN_PORT, value_name = "TCP_PORT")]
    pub tcp_port: u16,

    /// TCP listen host address
    #[arg(short = 'H', long, default_value = "127.0.0.1", value_name = "HOST")]
    pub host: String,
}
