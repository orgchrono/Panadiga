//! Hardware break-in, aperture dump, UF2 flashing, and GDB RSP bridge CLI args.

use clap::Args;
use panadiga_core::constants::{
    DEFAULT_BAUD_RATE, DEFAULT_GDB_LISTEN_PORT, DEFAULT_GDB_LISTEN_SOCKET, DEFAULT_SERIAL_PORT,
};
use std::path::PathBuf;

/// Executes P7501 discrete boot/reset profile mode (SW_UART+XRESET; 0=Hi-Z, 1=Serial Pulse, 2=JIG Hold LOW, 3=Extended Hi-Z).
#[derive(Args, Debug)]
pub struct BreakInArgs {
    /// Serial port identifier (e.g. COM10 or /dev/ttyACM0)
    #[arg(short, long, default_value = DEFAULT_SERIAL_PORT, value_name = "PORT")]
    pub port: String,

    /// Cold Break-In mode (0..3, default 2)
    #[arg(short, long, default_value_t = 2)]
    pub mode: u8,

    /// Baud rate (defaults to 115200)
    #[arg(short, long, default_value_t = DEFAULT_BAUD_RATE)]
    pub baud: u32,
}

/// Reads 32-bit little-endian words in-circuit from AM34 memory/Flash/NVRAM via Pico DUMP.
#[derive(Args, Debug)]
pub struct PicoDumpArgs {
    /// Serial port identifier (e.g. COM10 or /dev/ttyACM0)
    #[arg(short, long, default_value = DEFAULT_SERIAL_PORT, value_name = "PORT")]
    pub port: String,

    /// Start bus address in hex (e.g. 0x807F0000 for NVRAM or 0x80000000 for Spansion IPL)
    #[arg(short, long, default_value = "0x807F0000", value_name = "HEX_ADDR")]
    pub addr: String,

    /// Number of 32-bit words to read (1..=16384, default 64 words = 256 bytes)
    #[arg(short, long, default_value_t = 64)]
    pub words: usize,

    /// Optional output file path to store raw captured binary bytes
    #[arg(short, long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    /// Baud rate (defaults to 115200)
    #[arg(short, long, default_value_t = DEFAULT_BAUD_RATE)]
    pub baud: u32,
}

/// Starts a TCP-to-Serial GDB Remote Serial Protocol (RSP) bridge server.
#[derive(Args, Debug)]
pub struct GdbBridgeArgs {
    /// Serial port identifier connected to Raspberry Pi Pico (e.g. COM10)
    #[arg(short, long, value_name = "PORT")]
    pub port: String,

    /// Serial baud rate (defaults to 115200)
    #[arg(short, long, default_value_t = DEFAULT_BAUD_RATE)]
    pub baud: u32,

    /// TCP listener address and port for incoming GDB connections
    #[arg(short, long, default_value = DEFAULT_GDB_LISTEN_SOCKET)]
    pub listen: String,

    /// Target monitor protocol behind P7501 async UART: ocd | panamon1 | panarsp
    #[arg(long, default_value = "ocd", value_name = "MODE")]
    pub target_mode: String,
}

/// Automatically flashes the Panadiga P7501 Async UART firmware to a detected Raspberry Pi Pico (RP2040/RP2350).
#[derive(Args, Debug)]
pub struct PicoFlashArgs {
    /// Optional serial port of a running Pico to trigger 1200-baud touch reset into BOOTSEL mode
    #[arg(short, long, value_name = "PORT")]
    pub port: Option<String>,

    /// Path to custom .uf2 firmware file (defaults to scanning dist/, firmware_bins, or embedded UF2)
    #[arg(short, long, value_name = "UF2_FILE")]
    pub file: Option<PathBuf>,
}

/// Merges RP2040 and RP2350 .uf2 images into an RP2350-E10 compliant universal UF2.
#[derive(Args, Debug)]
pub struct MergeUf2Args {
    /// Path to compiled RP2040 UF2 firmware
    #[arg(long, value_name = "RP2040_UF2")]
    pub rp2040: PathBuf,

    /// Path to compiled RP2350 UF2 firmware
    #[arg(long, value_name = "RP2350_UF2")]
    pub rp2350: PathBuf,

    /// Destination path for unified universal UF2
    #[arg(short, long, value_name = "OUT_UF2")]
    pub out: PathBuf,
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
