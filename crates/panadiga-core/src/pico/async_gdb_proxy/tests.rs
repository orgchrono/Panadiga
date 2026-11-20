#![deny(unsafe_code)]

use std::collections::VecDeque;
use std::io::{self, Read, Write};

use super::super::panamon1::parse_panamon1_read_bytes;
use super::{GdbBridgeTargetMode, handle_panamon1_rsp_packet, handle_panamon1_rsp_packet_io};

#[test]
fn parses_target_modes_and_translates_panamon1_rsp_safely() {
    assert_eq!(
        GdbBridgeTargetMode::parse_mode("panamon1").unwrap(),
        GdbBridgeTargetMode::Panamon1
    );
    assert_eq!(
        GdbBridgeTargetMode::parse_mode("panarsp").unwrap(),
        GdbBridgeTargetMode::Panarsp
    );
    let sup = String::from_utf8(handle_panamon1_rsp_packet("qSupported", None)).unwrap();
    assert!(sup.contains("qXfer:memory-map:read+"));

    let g = String::from_utf8(handle_panamon1_rsp_packet("g", None)).unwrap();
    assert!(g.starts_with('$') && g.len() > 220);

    // Critical IPL NOR write must be blocked with E05 even through async Ghidra proxy
    let e05 = String::from_utf8(handle_panamon1_rsp_packet("M80000000,2:1234", None)).unwrap();
    assert!(e05.starts_with("$E05#"));
}

struct MockPanamon1Port {
    tx_log: Vec<String>,
    rx_queue: VecDeque<u8>,
}

impl MockPanamon1Port {
    fn new() -> Self {
        Self {
            tx_log: Vec::new(),
            rx_queue: VecDeque::new(),
        }
    }

    fn respond(&mut self, cmd: &str) {
        let reply = if cmd == "R 847F0010,4" {
            "R 847F0010,4\r\nAA55FFFF\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n"
        } else if cmd.starts_with("W ") {
            "W ...\r\nOK:WRITE\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n"
        } else {
            "ERR:E05_IPL_RO\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n"
        };
        self.rx_queue.extend(reply.as_bytes());
    }
}

impl Read for MockPanamon1Port {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut count = 0;
        while count < buf.len() {
            let Some(b) = self.rx_queue.pop_front() else {
                break;
            };
            buf[count] = b;
            count += 1;
        }
        Ok(count)
    }
}

impl Write for MockPanamon1Port {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf)
            .trim_end_matches('\r')
            .to_string();
        if !s.is_empty() {
            self.respond(&s);
            self.tx_log.push(s);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn parses_real_panamon1_raw_hex_lines_without_endian_swap_and_rejects_echo() {
    let reply = "R 84220224,4\r\nAA55FFFF\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n";
    assert_eq!(
        parse_panamon1_read_bytes(reply, 4).unwrap(),
        vec![0xAA, 0x55, 0xFF, 0xFF]
    );
    assert!(
        parse_panamon1_read_bytes("R 84220224,4\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n", 4).is_none()
    );
    assert!(parse_panamon1_read_bytes("R 80000000,4\r\nERR:E05_IPL_RO\r\n", 4).is_none());
}

#[test]
fn roundtrips_m_read_and_bytewise_nvram_m_write_against_mock_panamon1() {
    let mut port = MockPanamon1Port::new();
    let read_rsp = String::from_utf8(handle_panamon1_rsp_packet_io(
        "m847f0010,4",
        Some(&mut port),
    ))
    .unwrap();
    assert!(read_rsp.starts_with("$aa55ffff#"), "rsp={read_rsp}");

    let write_rsp = String::from_utf8(handle_panamon1_rsp_packet_io(
        "M847f0020,2:a55a",
        Some(&mut port),
    ))
    .unwrap();
    assert!(write_rsp.starts_with("$OK#"), "rsp={write_rsp}");
    assert_eq!(
        port.tx_log,
        vec!["R 847F0010,4", "W 847F0020,A5", "W 847F0021,5A"]
    );
}
