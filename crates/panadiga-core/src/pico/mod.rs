//! Multi-Platform Host Controller (RP2040 / RP2350 / RP2 Family) Serial & GDB RSP Bridge Module
//!
//! Provides cross-platform hardware discovery, diagnostic telemetry probing, and a TCP-to-Serial
//! GDB Remote Serial Protocol (RSP) bridge connecting high-level debuggers
//! (mn10300-elf-gdb, gdb-multiarch, Ghidra, IDA Pro) for Panasonic P7501 Async UART (115200 8N1) & Discrete Boot/Reset.
//! Supports Raspberry Pi Pico/Pico W, Pico 2 (ARM & RISC-V), Arduino Nano RP2040 Connect,
//! Seeed XIAO, Adafruit, SparkFun, and Waveshare RP2-family controllers across Windows, Linux, and macOS.

pub mod async_gdb_proxy;
pub mod bench_decoders;
pub mod bench_wizard;
pub mod bridge;
pub mod cdc;
pub mod diag_rows;
pub mod diagnostics;
pub mod flasher;
pub mod log_decoder;
pub mod panamon1;
pub mod platforms;
pub mod rsp_codec;

pub use async_gdb_proxy::GdbBridgeTargetMode;
pub use bench_decoders::*;
pub use bench_wizard::*;
pub use bridge::{run_gdb_bridge, run_gdb_bridge_with_mode};
pub use diagnostics::ping_pico;
pub use flasher::*;
pub use log_decoder::*;
pub use platforms::*;
pub use rsp_codec::*;
