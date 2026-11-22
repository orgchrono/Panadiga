#![deny(unsafe_code)]

//! Synchronous TCP GDB/Ghidra RSP client loop bridged over Pico `BRIDGE 115200 PASSIVE`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serialport::SerialPort;

use super::super::rsp_codec::{NorWritePolicy, build_rsp_frame};
use super::{handle_panamon1_rsp_bytes_io, query_panamon1_with_retry};

/// Serves one synchronous TCP GDB/Ghidra client by translating RSP packets asynchronously
/// into `PANAMON1>` UART transactions on Pico `BRIDGE`.
pub fn serve_panamon1_tcp_client(
    mut tcp_stream: TcpStream,
    serial_port: &mut dyn SerialPort,
) -> std::io::Result<()> {
    let _ = tcp_stream.set_read_timeout(Some(Duration::from_millis(80)));
    let mut buf = [0u8; 2048];
    let mut rx_acc: Vec<u8> = Vec::new();
    let mut nor_policy = NorWritePolicy::NvramOnly;

    let result = (|| -> std::io::Result<()> {
        loop {
            match tcp_stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    rx_acc.extend_from_slice(&buf[..n]);
                    loop {
                        let first_dollar = rx_acc.iter().position(|&b| b == b'$');
                        let ctrl_c_limit = first_dollar.unwrap_or(rx_acc.len());
                        if let Some(ctrl_pos) =
                            rx_acc[..ctrl_c_limit].iter().position(|&b| b == 0x03)
                        {
                            rx_acc.remove(ctrl_pos);
                            let _ = tcp_stream
                                .write_all(&build_rsp_frame(b"S05", false).unwrap_or_default());
                            let _ = tcp_stream.flush();
                            continue;
                        }
                        let Some(dollar) = first_dollar else {
                            rx_acc.clear();
                            break;
                        };
                        let after_dollar = &rx_acc[dollar + 1..];
                        let Some(hash) = after_dollar.iter().position(|&b| b == b'#') else {
                            if dollar > 0 {
                                rx_acc.drain(..dollar);
                            }
                            break;
                        };
                        if after_dollar.len() < hash + 3 {
                            if dollar > 0 {
                                rx_acc.drain(..dollar);
                            }
                            break;
                        }
                        let cmd_bytes = after_dollar[..hash].to_vec();
                        rx_acc.drain(..dollar + 1 + hash + 3);
                        let _ = tcp_stream.write_all(b"+");
                        let reply = handle_panamon1_rsp_bytes_io(
                            &cmd_bytes,
                            Some(serial_port),
                            &mut nor_policy,
                        );
                        tcp_stream.write_all(&reply)?;
                        tcp_stream.flush()?;
                        if cmd_bytes == b"k" {
                            return Ok(());
                        }
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::TimedOut
                        || e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    })();

    if nor_policy == NorWritePolicy::UpperNorAppUnlocked {
        let _ = query_panamon1_with_retry(serial_port, "U LOCK", Duration::from_millis(1000));
    }
    result
}
