#![deny(unsafe_code)]

use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use super::super::panamon1::{PANAMON1_RETRY_PACING_US, parse_panamon1_read_bytes};
use super::super::rsp_codec::NorWritePolicy;
use super::{
    GdbBridgeTargetMode, handle_panamon1_rsp_bytes_io, handle_panamon1_rsp_packet,
    handle_panamon1_rsp_packet_io,
};

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
    pending_glitches: VecDeque<String>,
}

impl MockPanamon1Port {
    fn new() -> Self {
        Self {
            tx_log: Vec::new(),
            rx_queue: VecDeque::new(),
            pending_glitches: VecDeque::new(),
        }
    }

    fn respond(&mut self, cmd: &str) {
        if let Some(glitch) = self.pending_glitches.pop_front() {
            self.rx_queue.extend(glitch.as_bytes());
            return;
        }
        let reply = if cmd == "R 847F0010,4" {
            "R 847F0010,4\r\nAA55FFFF\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n"
        } else if cmd == "U CONFIRM_UPPER_NOR" {
            "U CONFIRM_UPPER_NOR\r\nOK:NOR_UNLOCK\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n"
        } else if cmd == "U LOCK" {
            "U LOCK\r\nOK:NOR_LOCK\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n"
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

#[test]
fn replays_real_com10_panamon1_boottrace_and_rw_session_transcript() {
    let transcript = concat!(
        "BOOTTRACE: OK\r\n",
        "PICO>\r\n",
        "[BRIDGE] PANAMON1> prompt confirmed active.\r\n",
        "R 90439B00,10\r\n",
        "000180500000000000000000CAFE0001\r\n",
        "PANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n",
    );
    assert_eq!(
        parse_panamon1_read_bytes(transcript, 16).unwrap(),
        vec![
            0x00, 0x01, 0x80, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xCA, 0xFE,
            0x00, 0x01
        ]
    );
    assert!(
        parse_panamon1_read_bytes(
            "W 90439B40,F0\r\nOK:WRITE\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n",
            16
        )
        .is_none()
    );

    let mut port = MockPanamon1Port::new();
    let ipl_ro = String::from_utf8(handle_panamon1_rsp_packet_io(
        "M80000000,1:00",
        Some(&mut port),
    ))
    .unwrap();
    assert!(ipl_ro.starts_with("$E05#"), "rsp={ipl_ro}");
    assert!(port.tx_log.is_empty());

    let err_reply = "R 80000000,4\r\nERR:E05_IPL_RO\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n";
    assert!(parse_panamon1_read_bytes(err_reply, 4).is_none());
}

#[test]
fn retries_after_300us_pacing_on_question_mark_or_missing_prompt() {
    let mut port = MockPanamon1Port::new();
    port.pending_glitches
        .push_back("\r\n?\r\nPANAMON1>[B/R/W/D/F/J/E/U/Q]\r\n".to_string());

    let start = Instant::now();
    let rsp = String::from_utf8(handle_panamon1_rsp_packet_io(
        "m847f0010,4",
        Some(&mut port),
    ))
    .unwrap();
    assert!(start.elapsed() >= Duration::from_micros(PANAMON1_RETRY_PACING_US));
    assert!(rsp.starts_with("$aa55ffff#"), "rsp={rsp}");
    assert_eq!(port.tx_log, vec!["R 847F0010,4", "R 847F0010,4"]);
}

#[test]
fn forwards_nor_unlock_enables_upper_nor_blocks_sa0_sa3_and_relocks() {
    let mut port = MockPanamon1Port::new();
    let mut policy = NorWritePolicy::NvramOnly;

    let locked = String::from_utf8(handle_panamon1_rsp_bytes_io(
        b"M84040000,1:ab",
        Some(&mut port),
        &mut policy,
    ))
    .unwrap();
    assert!(locked.starts_with("$E05#"), "rsp={locked}");
    assert!(port.tx_log.is_empty());

    let bad_unlock = String::from_utf8(handle_panamon1_rsp_bytes_io(
        b"qPanadigaNorUnlock:BAD_TOKEN",
        Some(&mut port),
        &mut policy,
    ))
    .unwrap();
    assert!(bad_unlock.starts_with("$E01#"), "rsp={bad_unlock}");

    let unlock = String::from_utf8(handle_panamon1_rsp_bytes_io(
        b"qPanadigaNorUnlock:CONFIRM_UPPER_NOR",
        Some(&mut port),
        &mut policy,
    ))
    .unwrap();
    assert!(unlock.starts_with("$OK#"), "rsp={unlock}");
    assert_eq!(policy, NorWritePolicy::UpperNorAppUnlocked);

    let upper_ok = String::from_utf8(handle_panamon1_rsp_bytes_io(
        b"M84040000,1:ab",
        Some(&mut port),
        &mut policy,
    ))
    .unwrap();
    assert!(upper_ok.starts_with("$OK#"), "rsp={upper_ok}");

    // Critical SA0..SA3 bootloader remains permanently E05 even when Upper NOR is unlocked.
    let ipl_ro = String::from_utf8(handle_panamon1_rsp_bytes_io(
        b"M84000000,1:cd",
        Some(&mut port),
        &mut policy,
    ))
    .unwrap();
    assert!(ipl_ro.starts_with("$E05#"), "rsp={ipl_ro}");

    let straddle_ipl = String::from_utf8(handle_panamon1_rsp_bytes_io(
        b"X8003ffff,2:ab",
        Some(&mut port),
        &mut policy,
    ))
    .unwrap();
    assert!(straddle_ipl.starts_with("$E05#"), "rsp={straddle_ipl}");

    let lock = String::from_utf8(handle_panamon1_rsp_bytes_io(
        b"qPanadigaNorLock",
        Some(&mut port),
        &mut policy,
    ))
    .unwrap();
    assert!(lock.starts_with("$OK#"), "rsp={lock}");
    assert_eq!(policy, NorWritePolicy::NvramOnly);

    let relocked = String::from_utf8(handle_panamon1_rsp_bytes_io(
        b"M84040000,1:ef",
        Some(&mut port),
        &mut policy,
    ))
    .unwrap();
    assert!(relocked.starts_with("$E05#"), "rsp={relocked}");
    assert_eq!(
        port.tx_log,
        vec!["U CONFIRM_UPPER_NOR", "W 84040000,AB", "U LOCK"]
    );
}

#[test]
fn supports_binary_x_memory_write_with_rsp_unescaping_and_high_bytes() {
    let mut port = MockPanamon1Port::new();
    let mut policy = NorWritePolicy::NvramOnly;

    let probe = String::from_utf8(handle_panamon1_rsp_bytes_io(
        b"X847f0000,0:",
        Some(&mut port),
        &mut policy,
    ))
    .unwrap();
    assert!(probe.starts_with("$OK#"), "probe={probe}");

    let mut pkt = b"X847f0030,6:".to_vec();
    pkt.extend_from_slice(&[0x7D, 0x5D, 0x7D, 0x03, 0x7D, 0x04, 0x7D, 0x0A, 0x80, 0xFF]);
    let rsp = String::from_utf8(handle_panamon1_rsp_bytes_io(
        &pkt,
        Some(&mut port),
        &mut policy,
    ))
    .unwrap();
    assert!(rsp.starts_with("$OK#"), "rsp={rsp}");
    assert_eq!(
        port.tx_log,
        vec![
            "W 847F0030,7D",
            "W 847F0031,23",
            "W 847F0032,24",
            "W 847F0033,2A",
            "W 847F0034,80",
            "W 847F0035,FF",
        ]
    );
}

#[test]
fn blocks_lethal_register_c000100c_for_both_reads_and_writes() {
    let mut port = MockPanamon1Port::new();
    let mut policy = NorWritePolicy::NvramOnly;

    for raw in [
        b"mC000100C,4".as_slice(),
        b"mc0001008,8".as_slice(),
        b"MC000100C,4:00000000".as_slice(),
        b"Xc000100f,1:\x00".as_slice(),
    ] {
        let rsp = String::from_utf8(handle_panamon1_rsp_bytes_io(
            raw,
            Some(&mut port),
            &mut policy,
        ))
        .unwrap();
        assert!(rsp.starts_with("$E05#"), "raw={raw:?} rsp={rsp}");
    }
    assert!(port.tx_log.is_empty());
}
