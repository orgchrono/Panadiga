//! TCP stream session handlers for the GDB RSP proxy (Local Stub vs USB CDC).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use panadiga_core::pico::rsp_codec::{
    NorWritePolicy, build_nor_lock_frame, filter_host_rsp_tcp_stream,
};

use super::super::rsp_protocol::compute_mock_rsp_response_with_policy;
use super::controller::GdbBridgeController;

enum InboundTcpChunk {
    Data(String, usize),
    WouldBlock,
    Closed,
}

fn prepare_gdb_tcp_stream(tcp_stream: &mut TcpStream) {
    let _ = tcp_stream.set_nonblocking(false);
    let _ = tcp_stream.set_read_timeout(Some(Duration::from_millis(50)));
}

fn read_inbound_tcp_chunk(
    ctrl: &GdbBridgeController,
    tcp_stream: &mut TcpStream,
    buf: &mut [u8],
) -> InboundTcpChunk {
    match tcp_stream.read(buf) {
        Ok(0) => InboundTcpChunk::Closed,
        Ok(n) => {
            let raw = String::from_utf8_lossy(&buf[..n]).into_owned();
            ctrl.record_telemetry("INBOUND", &raw);
            InboundTcpChunk::Data(raw, n)
        }
        Err(ref e) if crate::pico::is_transient_io_error(e) => InboundTcpChunk::WouldBlock,
        Err(_) => InboundTcpChunk::Closed,
    }
}

fn is_session_open(ctrl: &GdbBridgeController) -> bool {
    ctrl.is_active()
        && ctrl
            .client_connected_flag()
            .load(std::sync::atomic::Ordering::Relaxed)
}

pub(super) fn handle_local_stub_session(ctrl: &GdbBridgeController, mut tcp_stream: TcpStream) {
    prepare_gdb_tcp_stream(&mut tcp_stream);
    let mut t_buf = [0u8; 1024];
    let mut nor_policy = NorWritePolicy::NvramOnly;

    while is_session_open(ctrl) {
        match read_inbound_tcp_chunk(ctrl, &mut tcp_stream, &mut t_buf) {
            InboundTcpChunk::Closed => break,
            InboundTcpChunk::WouldBlock => continue,
            InboundTcpChunk::Data(raw, _) => {
                for segment in raw.split('$').skip(1) {
                    let cmd = segment.split('#').next().unwrap_or("");
                    if cmd.is_empty() {
                        continue;
                    }
                    if cmd.starts_with('G') {
                        ctrl.sync_registers(cmd);
                    }
                    let regs_snapshot = ctrl.registers_snapshot();
                    let reply_bytes =
                        compute_mock_rsp_response_with_policy(cmd, &regs_snapshot, &mut nor_policy);
                    let reply_str = String::from_utf8_lossy(&reply_bytes).into_owned();
                    ctrl.record_telemetry("OUTBOUND", &reply_str);
                    if tcp_stream.write_all(&reply_bytes).is_err() {
                        return;
                    }
                }
                let _ = tcp_stream.flush();
            }
        }
    }
}

pub(super) fn handle_serial_session(
    ctrl: &GdbBridgeController,
    mut tcp_stream: TcpStream,
    serial_port: &mut Box<dyn serialport::SerialPort>,
) {
    prepare_gdb_tcp_stream(&mut tcp_stream);

    let mut s_buf = [0u8; 1024];
    let mut t_buf = [0u8; 1024];
    let mut tcp_acc: Vec<u8> = Vec::new();
    let mut nor_policy = NorWritePolicy::NvramOnly;
    let mut last_inbound_was_g = false;
    let mut g_accum_buf = String::with_capacity(128);

    while is_session_open(ctrl) {
        match read_inbound_tcp_chunk(ctrl, &mut tcp_stream, &mut t_buf) {
            InboundTcpChunk::Closed => break,
            InboundTcpChunk::WouldBlock => {}
            InboundTcpChunk::Data(_packet, n) => {
                let res = filter_host_rsp_tcp_stream(&mut tcp_acc, &t_buf[..n], &mut nor_policy);
                for raw_cmd in &res.observed_cmds {
                    let cmd = String::from_utf8_lossy(raw_cmd);
                    if cmd == "g" {
                        last_inbound_was_g = true;
                        g_accum_buf.clear();
                    } else if cmd.starts_with('G') {
                        ctrl.sync_registers(&cmd);
                    }
                }
                if !res.forward_to_serial.is_empty() {
                    let _ = serial_port.write_all(&res.forward_to_serial);
                }
                if !res.blocked_host_replies.is_empty() {
                    let reply_str = String::from_utf8_lossy(&res.blocked_host_replies).into_owned();
                    ctrl.record_telemetry("OUTBOUND", &reply_str);
                    if tcp_stream.write_all(&res.blocked_host_replies).is_err() {
                        break;
                    }
                    let _ = tcp_stream.flush();
                }
            }
        }

        match serial_port.read(&mut s_buf) {
            Ok(0) => {}
            Ok(n) => {
                let packet = String::from_utf8_lossy(&s_buf[..n]);
                ctrl.record_telemetry("OUTBOUND", &packet);
                if last_inbound_was_g {
                    g_accum_buf.push_str(&packet);
                    if g_accum_buf.contains('#') {
                        ctrl.sync_registers(&g_accum_buf);
                        last_inbound_was_g = false;
                        g_accum_buf.clear();
                    }
                }
                if tcp_stream.write_all(&s_buf[..n]).is_err() {
                    break;
                }
            }
            Err(ref e) if crate::pico::is_transient_io_error(e) => {}
            Err(_) => break,
        }
    }

    if nor_policy == NorWritePolicy::UpperNorAppUnlocked {
        let lock_frame = build_nor_lock_frame();
        let _ = serial_port.write_all(&lock_frame);
        let _ = serial_port.flush();
    }
}
