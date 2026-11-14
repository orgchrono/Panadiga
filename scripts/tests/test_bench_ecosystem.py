"""Offline CI unit tests for the non-destructive P7501 bench ecosystem."""

from __future__ import annotations

import hashlib
from pathlib import Path

import pytest

from scripts.bench.decoders import (
    parse_am34_uart_log,
    parse_busdiag_output,
    parse_panarsp_output,
    parse_piologic_output,
    parse_pincheck_output,
    parse_status_output,
    read_captured_log,
)
from scripts.bench.protocol import (
    BenchSafetyError,
    BenchSession,
    MockBenchTransport,
    validate_safe_bench_command,
)
from scripts.bench.rsp_dump import (
    MockAm34RspStream,
    ReadOnlyRspDumper,
    build_rsp_frame,
)
from scripts.bench.runner import main as bench_main, run_smoke_suite


def test_bench_session_enforces_safety_and_isolate() -> None:
    """Reject mutating commands and guarantee ISOLATE runs when leaving BenchSession."""
    mock = MockBenchTransport()
    with BenchSession(mock) as session:
        assert "PONG" in session.send("PING")
        with pytest.raises(BenchSafetyError):
            session.send("FLASH_ERASE 0x807F0000")
        with pytest.raises(BenchSafetyError):
            session.send("STATUS\nERASE")
    assert mock.history[-1] == "ISOLATE"


def test_gp26_sticky_low_detection_and_uart_decoders() -> None:
    """Detect sticky-LOW GP26 bug cleanly and parse COM10 electrical/boot telemetry."""
    smoke = run_smoke_suite()
    assert smoke["smoke_ok"] is True
    assert smoke["gp26_bug_detected"] is True
    assert smoke["gp26_recovered"] is True

    mock = MockBenchTransport()
    with BenchSession(mock) as session:
        pins = parse_pincheck_output(session.send("PINCHECK"))
        bus = parse_busdiag_output(session.send("BUSDIAG"))
        st = parse_status_output(session.send("STATUS"))
        jig = parse_am34_uart_log(session.send("JIGBOOT"))

    assert st.vdd_volts == pytest.approx(2.70)
    assert len(pins) == 4
    assert pins[0].hiz_volts == pytest.approx(3.25)
    assert bus.gp2_sole_active_tx is True
    assert bus.xreset_rise_us == 20
    assert jig.ddr_check_ok is True
    assert jig.loader_version == "ver0.61 [06/03/06]"
    assert jig.startup_reason == "70"
    assert -91006 in jig.sdcd_errors
    assert jig.sense_packets[0].vendor_mecha_code == 0x83A7


def test_utf16le_log_decoding_and_cli_smoke(tmp_path: Path) -> None:
    """Verify UTF-16LE capture decoding and CLI smoke-suite exit status."""
    sample = tmp_path / "normal_utf16.log"
    sample.write_bytes("DDR CHECK: OK\r\n***** normal mode *****\r\nBGO driver init failed[1].\r\n".encode("utf-16"))
    decoded = parse_am34_uart_log(read_captured_log(sample))
    assert decoded.ddr_check_ok is True
    assert decoded.boot_mode == "normal mode"
    assert decoded.bgo_failed_code == 1
    assert validate_safe_bench_command("PIOLOGIC 4000 JIG:350") == "PIOLOGIC 4000 JIG:350"
    assert validate_safe_bench_command("PANARSP 6000 JIG:350 PROBESEQ") == "PANARSP 6000 JIG:350 PROBESEQ"
    assert bench_main(["smoke-suite"]) == 0


def test_piologic_panarsp_decoders_and_readonly_rsp_dump() -> None:
    """Validate PIOLOGIC/PANARSP decoders and chunked read-only RSP dump with retries."""
    mock = MockBenchTransport()
    with BenchSession(mock) as session:
        pio_rep = parse_piologic_output(session.send("PIOLOGIC 4000 JIG:350"))
        rsp_rep = parse_panarsp_output(session.send("PANARSP 4000 MEM:515B2E38,4"))

    assert not pio_rep.fifo_stall
    assert pio_rep.edges_count == 28
    assert pio_rep.span_us == 512000
    assert pio_rep.nsw5v_transitions == 0
    assert pio_rep.reset_to_first_rxd_us == 482
    assert pio_rep.min_tx_to_rx_us == 120
    assert rsp_rep.rsp_status == "VALID"
    assert rsp_rep.payload_hex == "DEADBEEF"
    assert rsp_rep.checksum_ok is True
    assert not rsp_rep.fifo_overflow

    sample_image = bytes((i * 37) & 0xFF for i in range(600))
    sim = MockAm34RspStream(0x90200000, sample_image, corrupt_chunk_index=2)
    dumper = ReadOnlyRspDumper(sim, reply_timeout_s=0.5)
    result = dumper.read_memory(0x90200000, len(sample_image), chunk_size=200, use_magic=True)

    assert result.data == sample_image
    assert result.sha256 == hashlib.sha256(sample_image).hexdigest()
    assert result.chunks_ok == 3
    assert result.retries == 1
    assert result.exit_ok is True
    with pytest.raises(ValueError):
        build_rsp_frame("M80000000,4:deadbeef")
