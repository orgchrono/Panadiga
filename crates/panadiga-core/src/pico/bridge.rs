use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use super::async_gdb_proxy::{GdbBridgeTargetMode, serve_panamon1_tcp_client};
use super::bench_decoders::{evaluate_live_rail_gate, parse_status_output_report};
use super::cdc::send_and_read_until;
use super::rsp_codec::{
    NorWritePolicy, build_nor_lock_frame, filter_host_rsp_tcp_stream, parse_loopback_socket_addr,
};

struct BridgeIsolateGuard {
    port: Box<dyn serialport::SerialPort>,
    mode: GdbBridgeTargetMode,
}

impl Drop for BridgeIsolateGuard {
    fn drop(&mut self) {
        if self.mode != GdbBridgeTargetMode::OcdRaw {
            let _ = self.port.write_all(b"~~~");
            let _ = self.port.flush();
            thread::sleep(Duration::from_millis(60));
        }
        let _ = self.port.write_all(b"\r\nISOLATE\r\n");
        let _ = self.port.flush();
    }
}

fn pump_filtered_tcp_to_serial<R: Read, W: Write, C: Write>(
    mut tcp_rx: R,
    mut tcp_tx_err: C,
    mut serial_tx: W,
    flag: Arc<AtomicBool>,
) {
    let mut buf = [0u8; 2048];
    let mut acc = Vec::new();
    let mut nor_policy = NorWritePolicy::NvramOnly;

    while flag.load(Ordering::Relaxed) {
        match tcp_rx.read(&mut buf) {
            Ok(0) => {
                log_println!("{}", "[Bridge] GDB client disconnected.");
                break;
            }
            Ok(n) => {
                let res = filter_host_rsp_tcp_stream(&mut acc, &buf[..n], &mut nor_policy);
                if !res.forward_to_serial.is_empty()
                    && (serial_tx.write_all(&res.forward_to_serial).is_err()
                        || serial_tx.flush().is_err())
                {
                    break;
                }
                if !res.blocked_host_replies.is_empty()
                    && (tcp_tx_err.write_all(&res.blocked_host_replies).is_err()
                        || tcp_tx_err.flush().is_err())
                {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }
    if nor_policy == NorWritePolicy::UpperNorAppUnlocked {
        let lock_frame = build_nor_lock_frame();
        let _ = serial_tx.write_all(&lock_frame);
        let _ = serial_tx.flush();
    }
    flag.store(false, Ordering::Relaxed);
}

fn pump_stream<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    flag: Arc<AtomicBool>,
    on_eof_msg: Option<&str>,
) {
    let mut buf = [0u8; 2048];
    while flag.load(Ordering::Relaxed) {
        match reader.read(&mut buf) {
            Ok(0) => {
                if let Some(msg) = on_eof_msg {
                    log_println!("{}", msg);
                }
                break;
            }
            Ok(n) => {
                if writer.write_all(&buf[..n]).is_err() || writer.flush().is_err() {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }
    flag.store(false, Ordering::Relaxed);
}

/// Runs bidirectional TCP-to-Serial GDB Remote Serial Protocol (RSP) bridge (`ocd` legacy mode).
pub fn run_gdb_bridge(port_name: &str, baud_rate: u32, listen_addr: &str) -> Result<()> {
    run_gdb_bridge_with_mode(
        port_name,
        baud_rate,
        listen_addr,
        GdbBridgeTargetMode::OcdRaw,
    )
}

/// Runs TCP-to-Serial GDB RSP bridge selecting `ocd`, `panamon1`, or `panarsp` asynchronous UART mode.
pub fn run_gdb_bridge_with_mode(
    port_name: &str,
    baud_rate: u32,
    listen_addr: &str,
    mode: GdbBridgeTargetMode,
) -> Result<()> {
    let validated_listen = parse_loopback_socket_addr(listen_addr)?;

    log_println!("====================================================================");
    log_println!("   PANASONIC AM34 GDB RSP BRIDGE (P7501 UART 115200 8N1)");
    log_println!("   Host Architecture: Multi-Platform (RP2040 / RP2350 Dual-Core)");
    log_println!("   Serial Interface : {} @ {} baud", port_name, baud_rate);
    log_println!("   Target Mode      : {}", mode.label());
    log_println!("   TCP GDB Listener : {}", validated_listen);
    log_println!("====================================================================");

    let mut serial_port =
        crate::pico::cdc::open_cdc(port_name, baud_rate, Duration::from_millis(100))
            .with_context(|| format!("Failed to open serial port '{}'", port_name))?;

    let status_reply = send_and_read_until(
        &mut *serial_port,
        "STATUS",
        crate::pico::cdc::PICO_STATUS_STOP_TOKENS,
        Duration::from_millis(1_500),
    );
    let status_report = parse_status_output_report(&status_reply);
    if let Err(err) = evaluate_live_rail_gate(&status_report) {
        let _ = serial_port.write_all(b"\r\nISOLATE\r\n");
        let _ = serial_port.flush();
        return Err(err);
    }

    let mut guard = BridgeIsolateGuard {
        port: serial_port,
        mode,
    };

    match mode {
        GdbBridgeTargetMode::OcdRaw => {
            log_info!(
                "[Bridge] Requesting Pico native GDB handler (note: P7501 lacks DWire32A clock)..."
            );
            let _ = guard.port.write_all(b"\r\nGDB\r\n");
        }
        GdbBridgeTargetMode::Panamon1 | GdbBridgeTargetMode::Panarsp => {
            log_info!("[Bridge] Entering Pico SC0 UART passthrough (BRIDGE 115200 PASSIVE)...");
            let _ = guard.port.write_all(b"\r\nBRIDGE 115200 PASSIVE\r\n");
        }
    }
    let _ = guard.port.flush();
    thread::sleep(Duration::from_millis(120));

    let listener = TcpListener::bind(validated_listen)
        .with_context(|| format!("Failed to bind TCP listener on '{}'", validated_listen))?;

    log_println!(
        "[Bridge] Listening for GDB/Ghidra connections on {}...",
        validated_listen
    );
    log_println!(
        "[Bridge] In Ghidra/GDB console run: target remote {}",
        validated_listen
    );
    log_println!("--------------------------------------------------------------------");

    for stream_res in listener.incoming() {
        match stream_res {
            Ok(mut tcp_stream) => {
                let _ = tcp_stream.set_read_timeout(Some(Duration::from_millis(100)));
                let peer_addr = tcp_stream
                    .peer_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                log_info!("[Bridge] GDB client connected from: {}", peer_addr);

                if mode == GdbBridgeTargetMode::Panamon1 {
                    if let Err(err) = serve_panamon1_tcp_client(tcp_stream, &mut *guard.port) {
                        log_warn!("[Bridge] PANAMON1 async session ended: {err}");
                    }
                    continue;
                }

                let running = Arc::new(AtomicBool::new(true));

                // Thread 1: Serial -> TCP
                let serial_reader = guard
                    .port
                    .try_clone()
                    .context("Failed to clone serial port handle")?;
                let tcp_writer = tcp_stream
                    .try_clone()
                    .context("Failed to clone TCP stream handle")?;
                let tcp_err_writer = tcp_stream
                    .try_clone()
                    .context("Failed to clone TCP error-reply handle")?;
                let running_clone = running.clone();

                let s2t_handle = thread::spawn(move || {
                    pump_stream(serial_reader, tcp_writer, running_clone, None);
                });

                // Thread 2: Filtered TCP -> Serial (enforcing 0xC000100C & SA0..SA10 barriers)
                pump_filtered_tcp_to_serial(
                    &mut tcp_stream,
                    tcp_err_writer,
                    &mut guard.port,
                    running,
                );

                let _ = s2t_handle.join();
                log_info!("[Bridge] Session finished. Waiting for next GDB connection...");
            }
            Err(e) => {
                log_warn!("[Bridge] TCP connection failed: {}", e);
            }
        }
    }

    Ok(())
}
