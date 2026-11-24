"""Offline CI unit tests for the non-destructive P7501 bench ecosystem."""

from __future__ import annotations

import hashlib
import struct
from pathlib import Path

import pytest

from scripts.bench.decoders import (
    describe_boot_fat_error_code,
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
from scripts.bench.nor_dumper import (
    NOR_MAGIC_FOOTER,
    NOR_MAGIC_HEADER,
    build_readonly_nor_dumper_stub,
    decode_nor_dump_stream,
    validate_nor_dump_range,
)
from scripts.bench.rsp_dump import (
    MockAm34RspStream,
    ReadOnlyRspDumper,
    build_rsp_frame,
    decode_mn103s_g_registers,
)
from scripts.bench.runner import main as bench_main, run_smoke_suite
from scripts.re.pana_frm import (
    DRV_UNLOCK_COMMAND,
    classify_drv_cli_command,
    parse_drv_cli_table,
)


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


def test_binary_x_dump_and_g_register_decoding() -> None:
    """Validate $x binary reads with #9A/NUL bytes and 112-byte MN103S $g decoding."""
    tricky_payload = (
        b"\x00\x239A$\x15+OK\xff\x00#00#ff"
        + bytes((i * 53) & 0xFF for i in range(1200))
    )
    sim = MockAm34RspStream(0x84000000, tricky_payload, corrupt_chunk_index=1)
    dumper = ReadOnlyRspDumper(sim, reply_timeout_s=0.5)
    result = dumper.read_memory(
        0x84000000, len(tricky_payload), chunk_size=1088, use_binary_x=True
    )

    assert result.data == tricky_payload
    assert result.chunks_ok == 2
    assert result.retries == 1

    with pytest.raises(ValueError):
        dumper.read_memory(0x84000000, 1090, chunk_size=1089, use_binary_x=True)

    assert result.exit_ok is True
    dumper.enter_stub()
    regs_from_mock = dumper.read_registers()
    assert len(regs_from_mock) == 28
    assert regs_from_mock["w00"] == 0

    custom_words = [0x1000 + i for i in range(28)]
    custom_words[25] = 0xAA000064
    custom_words[26] = 0xBB000068
    custom_words[27] = 0xCC00006C
    packed_g = struct.pack("<28I", *custom_words).hex().encode("ascii")
    decoded_g = decode_mn103s_g_registers(packed_g)
    assert decoded_g["w25"] == 0xAA000064
    assert decoded_g["w26"] == 0xBB000068
    assert decoded_g["w27"] == 0xCC00006C
    with pytest.raises(ValueError):
        decode_mn103s_g_registers(b"00" * 16)


def test_drv_cli_113_table_classification_and_telemetry_notes() -> None:
    """Validate 113-entry DRV CLI parser defaults, classifier, and AM34 fault notes."""
    base = 0x400DF5C0
    image = bytearray(0x2000)
    entries = 113
    table_off = 0x100
    sample_names = ("VER", "WB", "PASS", "SERVOTEST", "READ")
    for idx in range(entries):
        name = sample_names[idx] if idx < len(sample_names) else f"CMD{idx}"
        flags = 0x38 if name == "WB" else 0x00
        raw_name = name.encode("ascii")[:8].ljust(8, b"\x00")
        struct.pack_into(
            "<8sIII",
            image,
            table_off + idx * 20,
            raw_name,
            base + 0x40 + idx * 4,
            0,
            flags,
        )

    parsed = parse_drv_cli_table(bytes(image), table_offset=table_off, image_base=base)
    assert len(parsed) == 113
    assert parsed[0].name == "VER"
    assert DRV_UNLOCK_COMMAND == "PASS 0x4d415453 0x48495441"
    assert classify_drv_cli_command("WB", 0x00) == "WRITE_BLOCKED"
    assert classify_drv_cli_command("UNKNOWN", 0x38) == "WRITE_BLOCKED"
    assert classify_drv_cli_command("PASS", 0x00) == "UNLOCK_AUTH"
    assert classify_drv_cli_command("VER", 0x00) == "READ_ONLY"
    assert classify_drv_cli_command("SERVOTEST", 0x00) == "DIAGNOSTIC"

    log_info = parse_am34_uart_log(
        "***** normal mode *****\r\nBGO driver init failed[1].\r\n"
        "Drive error 0x83A7 (Laser 0.5mW failed)\r\n"
    )
    joined_notes = " ".join(log_info.notes)
    assert "FUN_4400836f" in joined_notes
    assert "boot_panic_or_halt_7812" in joined_notes
    assert "FUN_400db60c" in joined_notes
    assert "Ram88000194" in joined_notes


def test_readonly_nor_dumper_stub_and_stream_verification() -> None:
    """Validate strictly read-only MN103S NOR dumper generation and stream decoding."""
    stub = build_readonly_nor_dumper_stub(
        0x84000000, 8192, block_size=4096, load_addr=0x51800000
    )
    assert NOR_MAGIC_HEADER + struct.pack("<III", 0x84000000, 8192, 4096) in stub
    assert NOR_MAGIC_FOOTER in stub
    assert bytes.fromhex("fc82082000d4") in stub
    assert bytes.fromhex("ca00") in stub

    blk0 = bytes((i & 0xFF) for i in range(128))
    blk1 = bytes(((255 - i) & 0xFF) for i in range(128))
    raw_stream = bytearray(b"DIGADIAG READY\r\n")
    raw_stream.extend(NOR_MAGIC_HEADER)
    raw_stream.extend(struct.pack("<III", 0x80000000, 256, 128))
    for blk in (blk0, blk1):
        raw_stream.extend(blk)
        raw_stream.extend(struct.pack("<I", sum(blk) & 0xFFFFFFFF))
    raw_stream.extend(NOR_MAGIC_FOOTER)

    dump = decode_nor_dump_stream(bytes(raw_stream))
    assert dump.start_addr == 0x80000000
    assert dump.data == blk0 + blk1
    assert dump.sha256 == hashlib.sha256(blk0 + blk1).hexdigest()
    assert len(dump.block_checksums) == 2

    # Corrupt checksum byte -> rejected
    bad_csum = bytearray(raw_stream)
    bad_csum[-len(NOR_MAGIC_FOOTER) - 1] ^= 0xFF
    with pytest.raises(ValueError):
        decode_nor_dump_stream(bytes(bad_csum))

    # Safety guards: RAM, MMIO, out-of-range end, bad alignment/block_size
    for bad_args in (
        (0x50000000, 4096, 4096),
        (0xD4002000, 4096, 4096),
        (0x847FF000, 8192, 4096),
        (0x84000000, 4096, 0),
        (0x84000000, 4100, 4096),
    ):
        with pytest.raises(ValueError):
            validate_nor_dump_range(*bad_args)
        with pytest.raises(ValueError):
            build_readonly_nor_dumper_stub(*bad_args)


def test_digadiag_50400000_and_guarded_rsp_writes_and_pass_unlock() -> None:
    """Verify DIGADIAG 0x50400000 default, PASS unlock classification, and IPL-guarded $M/$X writes."""
    from scripts.bench.nor_dumper import DIGADIAG_STUB_LOAD_ADDR, resolve_nor_stub_load_addr
    from scripts.bench.rsp_frames import (
        build_rsp_frame,
        build_rsp_write_frame,
        is_ipl_readonly_address,
        parse_rsp_reply,
    )

    assert resolve_nor_stub_load_addr(None, wrap_digadiag=True) == DIGADIAG_STUB_LOAD_ADDR
    assert DIGADIAG_STUB_LOAD_ADDR == 0x50400000
    assert resolve_nor_stub_load_addr(None, wrap_digadiag=False) == 0x51800000

    assert classify_drv_cli_command("WB", 0x38, pass_unlocked=False) == "WRITE_BLOCKED"
    assert classify_drv_cli_command("WB", 0x38, pass_unlocked=True) == "UNLOCKED_WRITE"

    assert is_ipl_readonly_address(0x84000000, 4)
    assert not is_ipl_readonly_address(0x847F0000, 4)

    with pytest.raises(ValueError):
        build_rsp_frame("M51800000,4:01020304")
    with pytest.raises(ValueError):
        build_rsp_write_frame(0x80000000, b"\x00\x01")
    with pytest.raises(ValueError):
        build_rsp_write_frame(0x847EFFF8, b"\x00" * 16)

    m_frame = build_rsp_write_frame(0x51800000, b"\xaa\xbb\xcc\xdd")
    parsed_m = parse_rsp_reply(m_frame)
    assert parsed_m is not None
    assert parsed_m.payload == b"M51800000,4:aabbccdd"

    x_frame = build_rsp_write_frame(0x847F0000, b"}#$*", use_binary_x=True)
    parsed_x = parse_rsp_reply(x_frame)
    assert parsed_x is not None
    assert parsed_x.payload.startswith(b"X847f0000,4:")
    assert b"}]" in parsed_x.payload and b"}\x03" in parsed_x.payload


def test_parse_am34_uart_log_sd_stage_funnel_not_masked_by_bgo() -> None:
    """Ensure Stage-1 SD diag load failure overrides secondary post-reset BGO cascade."""
    sample = (
        "DDR CHECK: OK\r\n"
        "***** Loader ver0.61 [06/03/06] *****\r\n"
        "***** serial boot mode *****\r\n"
        "startup reason = [5]\r\n"
        "SD init start...\r\n"
        "\tcapacity:3bb000[LBA](2002780160Byte)\t transfer speed:25000[Kbps]\r\n"
        "\tRCA:2 \t Bus Width:4(current) 4(support)\r\n"
        "SD = 0x3BB000[LBA]\r\n"
        "... SD init end\r\n"
        "SD diag load failed[0].\r\n"
        "***** normal mode *****\r\n"
        "BGO driver init failed[1].\r\n"
        "** can not close tray **\r\n"
    )
    rep = parse_am34_uart_log(sample)
    assert rep.sd_capacity_lba == 0x3BB000
    assert rep.sd_capacity_bytes == 2002780160
    assert rep.sd_bus_width == 4
    assert rep.sd_transfer_kbps == 25000
    assert rep.sd_hw_init_ok is True
    assert rep.sd_diag_load_failed == 0
    assert rep.odd_tray_failed is True
    assert rep.bgo_failed_code == 1
    assert "SD_HW_OK" in rep.boot_stages and "SD_DIAG_LOAD_FAIL" in rep.boot_stages
    assert "SD diag load failed[0]" in rep.diagnosis
    assert any("FUN_4400a97c" in n for n in rep.notes)

    interleaved = (
        "DDR CHECK: OK\r\n"
        "***** Loader ver0.61 [06/03/06] *****\r\n"
        "***** serial boot mode\r\n"
        "[DIAGWATCH +26ms] serial boot mode -> SW_UART released Hi-Z (Q7514 TX ready)\r\n"
        " *****\r\n"
        "SD = 0x3BB000[LBA]\r\n"
        "... SD init end\r\n"
        "[DIAGWATCH +1453ms] STAGE: SD hardware 4-bit init OK (... SD init end)\r\n"
        "\r\n"
        "SD diag load failed\r\n"
        "[DIAGWATCH +1504ms] ALERT: SD diag load failed[0] (FUN_4400a97c)\r\n"
        "[0].\r\n"
        "***** normal mode *****\r\n"
        "BGO driver init failed\r\n"
        "[DIAGWATCH +3599ms] ALERT: BGO driver init failure detected\r\n"
        "[1].\r\n"
        "** can not close tray\r\n"
        "[DIAGWATCH +6786ms] ALERT: ODD tray close error detected\r\n"
        " **\r\n"
    )
    rep_i = parse_am34_uart_log(interleaved)
    assert rep_i.boot_mode == "normal mode"
    assert rep_i.sd_hw_init_ok is True
    assert rep_i.sd_diag_load_failed == 0
    assert rep_i.bgo_failed_code == 1
    assert rep_i.odd_tray_failed is True
    assert "SD diag load failed[0]" in rep_i.diagnosis
    assert "fat_read 0x20000 failed" in rep_i.diagnosis

    assert "FAT_ERR_FILE_NOT_FOUND" in describe_boot_fat_error_code(-10010)
    assert "FAT_ERR_INVALID_FS_OR_IO" in describe_boot_fat_error_code(-10025)
    assert "FAT_ERR_CORRUPT_CLUSTER" in describe_boot_fat_error_code(-10028)
    mount_rep = parse_am34_uart_log("SD = 0x3BB000[LBA]\r\n... SD init end\r\nFAT Mount failed[-10025].\r\nSD diag load failed[0].\r\n")
    assert mount_rep.fat_mount_failed == -10025
    assert "FAT_ERR_INVALID_FS_OR_IO" in mount_rep.diagnosis


def test_golden_wire_contract_cross_stack_python_parity() -> None:
    """Verify Python check_command_safety, MockBenchTransport, and Rust CLI share the C11 wire contract."""
    import json
    from scripts.pico_heartbeat_ops import check_command_safety, grade_heartbeat

    contract_path = (
        Path(__file__).resolve().parents[2]
        / "firmware"
        / "pico-p7501"
        / "tests"
        / "golden"
        / "p7501_wire_contract.json"
    )
    doc = json.loads(contract_path.read_text(encoding="utf-8"))

    for safe_cmd in doc["safe_commands"]:
        ok, msg = check_command_safety(safe_cmd)
        assert ok, f"Python rejected safe command {safe_cmd!r}: {msg}"
        assert validate_safe_bench_command(safe_cmd)

    for bad_cmd in doc["blocked_commands"]:
        ok, _ = check_command_safety(bad_cmd)
        assert not ok, f"Python failed to block unsafe command {bad_cmd!r}"
        with pytest.raises(BenchSafetyError):
            validate_safe_bench_command(bad_cmd)

    standby_lines = "\n".join(doc["status_samples"][0]["lines"])
    health_standby, _ = grade_heartbeat(
        ping_ok=True,
        vdd_volts=0.0,
        dcsr_hex=None,
        ping_rc=0,
        status_rc=0,
        combined_text=standby_lines,
    )
    assert health_standby == "TARGET_OFF_OR_STANDBY"

    fault_lines = "\n".join(doc["status_samples"][1]["lines"])
    health_fault, _ = grade_heartbeat(
        ping_ok=True,
        vdd_volts=3.24,
        dcsr_hex=None,
        ping_rc=0,
        status_rc=0,
        combined_text=fault_lines,
    )
    assert health_fault == "TARGET_OFF_OR_STANDBY"

    mock = MockBenchTransport(vdd_volts=3.24, nsw5v_high=True, fault_latch=True, dropouts=2)
    with BenchSession(mock) as session:
        st_out = session.send("STATUS")
        assert "fault_latch=1" in st_out and "@P7501_RES cmd=STATUS status=OK" in st_out
        ack_out = session.send("FAULTACK")
        assert "@P7501_RES cmd=FAULTACK status=OK" in ack_out
        iso_out = session.send("ISOLATE")
        assert "@P7501_RES cmd=ISOLATE status=OK" in iso_out
        assert mock.fault_latch is False

    from scripts.profiles_types import P7501_RES_CODES, P7501_WIRE_TAGS

    assert doc["wire_tags"] == P7501_WIRE_TAGS
    assert doc["res_codes"] == P7501_RES_CODES
