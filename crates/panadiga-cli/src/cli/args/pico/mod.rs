//! CLI argument definitions for RP2040/RP2350 Pico hardware control and GDB RSP bridge.

pub mod flash_cmds;
pub mod live_cmds;
pub mod trace_cmds;

pub use flash_cmds::{
    BreakInArgs, GdbBridgeArgs, GdbConfigArgs, MergeUf2Args, PicoDumpArgs, PicoFlashArgs,
};
pub use live_cmds::{CaptureNorArgs, DavisProbeArgs, SiliconReplayArgs, SiliconSnapshotArgs};
pub use trace_cmds::{
    AbBootArgs, BenchWizardArgs, DecodeLogArgs, ParseDiagArgs, RspCodecArgs, ValidateBenchCmdArgs,
};

use clap::{Args, Subcommand};
use panadiga_core::constants::{DEFAULT_BAUD_RATE, DEFAULT_SERIAL_PORT};
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
    BreakIn(BreakInArgs),

    /// Reads 32-bit little-endian words in-circuit from AM34 memory/Flash/NVRAM via Pico DUMP
    Dump(PicoDumpArgs),

    /// Starts a TCP-to-Serial GDB Remote Serial Protocol (RSP) bridge server
    GdbBridge(GdbBridgeArgs),

    /// Interactive guided live bench battery (Standby wake, BAUDSCAN, SW_UART & Error Dict correlation)
    #[command(name = "bench-wizard", alias = "bench_wizard")]
    BenchWizard(BenchWizardArgs),

    /// Queries embedded Panasonic / Matshita firmware error records (431 DRV + Core Boot/VFD/SENSE)
    #[command(name = "error-lookup", alias = "error_lookup")]
    ErrorLookup {
        /// Error code (0x83A7, 80040904, U99, -91006, -10025) or keyword
        #[arg(value_name = "QUERY")]
        query: String,

        /// Optional JSON output path (`-` emits stdout JSON)
        #[arg(long, value_name = "JSON_OUT")]
        json: Option<PathBuf>,
    },

    /// Automatically flashes the Panadiga P7501 Async UART firmware to a detected Raspberry Pi Pico (RP2040/RP2350)
    Flash(PicoFlashArgs),

    /// Merges RP2040 and RP2350 .uf2 images into an RP2350-E10 compliant universal UF2
    #[command(name = "merge-uf2", alias = "merge_uf2")]
    MergeUf2(MergeUf2Args),

    /// Decodes offline COM10 / AM34 UART0 / Pico capture logs (UTF-16LE or UTF-8)
    #[command(name = "decode-log", alias = "decode_log")]
    DecodeLog(DecodeLogArgs),

    /// Parses structured Pico CLI telemetry replies (`status`, `pincheck`, `busdiag`, `piologic`, `panarsp`, `fat-error`)
    #[command(name = "parse-diag", alias = "parse_diag")]
    ParseDiag(ParseDiagArgs),

    /// Validates a P7501 bench command against Rust `SAFE_BENCH_COMMANDS` allowlist
    #[command(name = "validate-bench-cmd", alias = "validate_bench_cmd")]
    ValidateBenchCmd(ValidateBenchCmdArgs),

    /// Pure GDB RSP frame codec, E05 IPL NOR write guard, reply scanner, and `$g` decoder
    #[command(name = "rsp-codec", alias = "rsp_codec")]
    RspCodec(RspCodecArgs),

    /// Automated A/B cold-boot statistical harness helpers (`build-cmd`, `classify`, `summarize`, `csv`)
    #[command(name = "ab-boot", alias = "ab_boot")]
    AbBoot(AbBootArgs),

    /// Captures live Spansion IC5001 NOR slices (`full|bgo|nvram|loader|custom`) via Pico PANAMON1>
    CaptureNor(CaptureNorArgs),

    /// Probes AM34 DAVIS CH0/CH1 taskfiles and 0xEC IDENTIFY via Pico PANAMON1> (guards 0xC000_100C)
    DavisProbe(DavisProbeArgs),

    /// Replays a captured `ndb>` COM10 Silicon Explorer evidence log deterministically
    #[command(name = "silicon-replay", alias = "silicon_replay")]
    SiliconReplay(SiliconReplayArgs),

    /// Reads an `ndb> dd` SDRAM/MMIO snapshot over live PicoFW `BRIDGE` with lethal/BCU/standby barriers
    #[command(name = "silicon-snapshot", alias = "silicon_snapshot")]
    SiliconSnapshot(SiliconSnapshotArgs),
}
