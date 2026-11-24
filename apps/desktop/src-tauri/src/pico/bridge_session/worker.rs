//! Dedicated background worker thread for the SC0 UART bridge.

use std::io::{Read, Write};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::state::format_utc_hms;

use super::codec::bytes_to_hex_spaced;
use super::controller::{BridgeWorkerMsg, Sc0BridgeController, Sc0BridgeDataEvent};

pub(super) fn emit_bridge_event(
    app: &AppHandle,
    stream: &str,
    bytes: &[u8],
    text_override: Option<&str>,
) {
    let text = text_override
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| String::from_utf8_lossy(bytes).into_owned());
    let payload = Sc0BridgeDataEvent {
        stream: stream.to_string(),
        text,
        hex: bytes_to_hex_spaced(bytes),
        bytes: bytes.to_vec(),
        timestamp: format_utc_hms(),
    };
    let _ = app.emit(crate::commands::constants::EVENT_PICO_BRIDGE_DATA, payload);
}

pub(super) fn run_bridge_worker(
    ctrl: Sc0BridgeController,
    app: AppHandle,
    mut port: Box<dyn serialport::SerialPort>,
    rx_chan: Receiver<BridgeWorkerMsg>,
) {
    let mut buf = [0u8; 512];
    let mut stop_requested = false;
    let mut saw_bridge_res = false;
    let mut saw_dropout = false;

    loop {
        while let Ok(msg) = rx_chan.try_recv() {
            match msg {
                BridgeWorkerMsg::Data(bytes) => {
                    if let Err(err) = port.write_all(&bytes) {
                        ctrl.update_status(|st| {
                            st.last_event = format!("Erro TX Bridge: {}", err);
                        });
                        stop_requested = true;
                        break;
                    }
                    let _ = port.flush();
                    let sent_len = bytes.len() as u64;
                    ctrl.update_status(|st| {
                        st.bytes_tx += sent_len;
                        st.last_event = format!("TX {} bytes para SC0", sent_len);
                    });
                    emit_bridge_event(&app, "tx", &bytes, None);
                    if bytes == b"~~~" {
                        stop_requested = true;
                    }
                }
                BridgeWorkerMsg::StopAndExit => {
                    let _ = port.write_all(b"~~~");
                    let _ = port.flush();
                    emit_bridge_event(&app, "tx", b"~~~", Some("~~~ (exit BRIDGE)"));
                    stop_requested = true;
                }
            }
        }

        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                let slice = &buf[..n];
                let text = String::from_utf8_lossy(slice);
                ctrl.update_status(|st| {
                    st.bytes_rx += n as u64;
                });
                emit_bridge_event(&app, "rx", slice, None);
                if text.contains("EDROPOUT") || text.contains("NSW5V_DROPOUT") {
                    saw_dropout = true;
                }
                if text.contains("BRIDGE: OK")
                    || text.contains("BRIDGE: ERROR")
                    || text.contains("BRIDGE: ERR")
                    || text.contains("@P7501_RES cmd=BRIDGE")
                {
                    saw_bridge_res = true;
                    break;
                }
            }
            Ok(_) => {
                if stop_requested {
                    break;
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(ref e) if crate::pico::is_transient_io_error(e) => {
                if stop_requested {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if !saw_bridge_res {
        let _ = port.write_all(b"~~~");
        let _ = port.flush();
        thread::sleep(Duration::from_millis(60));
    }
    let _ = port.write_all(b"\r\nISOLATE\r\n");
    let _ = port.flush();
    drop(port);

    let end_msg = if saw_dropout {
        "Ponte SC0 abortada por queda NSW5V (EDROPOUT); P7501 isolado em Hi-Z."
    } else {
        "Ponte SC0 encerrada e P7501 isolado em Hi-Z (~~~ + ISOLATE)."
    };
    ctrl.set_active(false);
    ctrl.update_status(|st| {
        st.running = false;
        st.last_event = end_msg.to_string();
    });
    emit_bridge_event(&app, "sys", b"", Some(end_msg));
}
