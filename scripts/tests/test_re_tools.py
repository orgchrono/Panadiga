"""Unit tests for Matsushita MN103S and DRV_decomp static reverse-engineering helpers."""

from __future__ import annotations

import struct
import zlib

from scripts.re.mn10300 import (
    decode_bcc_refs,
    decode_branch_call_refs,
    decode_call_frames,
    decode_setlb_loops,
    find_callers_to,
    find_fc_immediate_loads,
    find_pointer_xrefs,
)
from scripts.re.pana_frm import (
    decompress_drv_payload,
    find_extrhead_containers,
    parse_atapi_cdb_dispatch,
    parse_drv_cli_table,
    parse_drv_error_table,
    parse_sense_lut,
)


def test_mn10300_pc_relative_rule_cur_insn() -> None:
    """Verify `target_va = instruction_va + signed_disp` for 16-bit and 32-bit calls."""
    buf = bytearray(0x200)
    buf[0x169] = 0xDD
    struct.pack_into("<i", buf, 0x16A, -0x159)
    buf[0x100] = 0xCD
    struct.pack_into("<h", buf, 0x101, -0x80)
    buf[0x20:0x2F] = b"Loader ver0.61\x00"
    buf[0x40] = 0xFC
    buf[0x41] = 0xCC
    struct.pack_into("<I", buf, 0x42, 0x44000020)
    struct.pack_into("<I", buf, 0x1C0, 0x44000010)

    raw = bytes(buf)
    refs = decode_branch_call_refs(raw, 0x44000000)
    assert any(r.va == 0x44000169 and r.target_va == 0x44000010 and r.mnemonic == "call32" for r in refs)
    callers = find_callers_to(raw, 0x44000000, [0x44000080])
    assert [(c.va, c.mnemonic) for c in callers] == [(0x44000100, "call16")]

    loads = find_fc_immediate_loads(raw, 0x44000000, 0x44000040, 0x44000050)
    assert len(loads) == 1
    assert loads[0].c_string == "Loader ver0.61"
    assert find_pointer_xrefs(raw, 0x44000000, 0x44000010) == [0x440001C0]


def test_pana_frm_tables_and_cp932_mecha_dict() -> None:
    """Verify EXTRHEAD, DRV zlib decompression, CLI table, ATAPI dispatch, Sense LUT, and 0x83A7."""
    compressed = b"\x00" * 0x40 + zlib.compress(b"DRV_DECOMP_OK")
    assert decompress_drv_payload(compressed) == b"DRV_DECOMP_OK"

    img = bytearray(0x400)
    img[0x10:0x18] = b"EXTRHEAD"
    struct.pack_into("<8sIII", img, 0x40, b"SLOGON\x00\x00", 0x40010000, 0, 1)
    struct.pack_into("<8sIII", img, 0x54, b"CDRW\x00\x00\x00\x00", 0x40020000, 1, 2)
    struct.pack_into("<IIII", img, 0x80, 0x40014B00, 0x40014559, 0x40014559, 0x40014C00)
    img[0xA0:0xA6] = bytes([0x02, 0x04, 0x01, 0x04, 0x09, 0x04])
    err_txt = "0.5mW発光ができない(HFｵﾝ密調整)".encode("cp932")
    struct.pack_into("<H", img, 0x100, 0x83A7)
    img[0x102 : 0x102 + len(err_txt)] = err_txt

    raw = bytes(img)
    assert [m.offset for m in find_extrhead_containers(raw, 0x44000000)] == [0x10]
    cli = parse_drv_cli_table(raw, 0x40, count=2)
    assert [(c.name, c.handler_va) for c in cli] == [("SLOGON", 0x40010000), ("CDRW", 0x40020000)]

    atapi = parse_atapi_cdb_dispatch(raw, 0x80, count=4)
    assert [e.is_active for e in atapi] == [True, False, False, True]

    senses = parse_sense_lut(raw, 0xA0, count=2)
    assert (senses[1].sense_key, senses[1].asc, senses[1].ascq) == (0x04, 0x09, 0x04)

    mecha = parse_drv_error_table(raw, 0x100, 0x168)
    assert len(mecha) == 1
    assert mecha[0].code == 0x83A7
    assert "0.5mW発光ができない" in mecha[0].message_jp


def test_ghidra_auto_unbrick_audit_helpers() -> None:
    """Verify exported Ghidra auto-unbrick Jython script runs cleanly in CPython and classifies sites."""
    import importlib

    audit = importlib.import_module("tools.ghidra.ghidra_auto_unbrick_audit")

    assert audit.detect_stage_from_metadata(0x44000000, "BOOT.BIN", (), 0) == "BOOT.BIN"
    assert audit.detect_stage_from_metadata(0, "firmware", ("BGO driver init",), 262144) == "PROG061"
    assert audit.detect_stage_from_metadata(0x50400000, "unknown.bin", (), 262144) == "PROG061"
    assert audit.detect_stage_from_metadata(0, "eh55_frm_PROG061_BOOT.bin", ("SetBusWidth", "BGO driver init"), 262144) == "PROG061"
    assert audit.detect_stage_from_metadata(0, "BOOT.BIN", ("SetBusWidth", "BGO driver init"), 262144) == "BOOT.BIN"
    assert audit.detect_stage_from_metadata(0x57000000, "EH55_MAIN_3410PQ", (), 13003052) == "MAIN3410PQ"
    assert audit.detect_stage_from_metadata(0, "EH55_MAIN_3410PQ.bin", ("SetBusWidth", "BGO driver init", "_dev_SERIAL"), 13003052) == "MAIN3410PQ"
    assert audit.detect_stage_from_metadata(0, "EH55_MAIN_3060NP", (), 13427852) == "MAIN3060NP"

    auto_bgo = audit.classify_prog061_sites(
        audit.PATCH_SUM_BRA,
        audit.PATCH_DEVINFO_BRA,
        audit.PATCH_A2CD_AUTOFORMAT,
    )
    assert auto_bgo["mode"] == "AUTO_FORMAT_BGO"

    mon_ok = audit.classify_prog061_sites(
        audit.PATCH_SUM_BRA,
        audit.PATCH_DEVINFO_BRA,
        audit.PATCH_A2CD_UART_MONITOR,
        audit.PATCH_A2BC_BRA_MONITOR,
    )
    assert mon_ok["mode"] == "UART_MONITOR_PANAMON1"
    assert mon_ok["sites"]["0x5040A2BC_bgo_init_call"] == "BRA_TO_MONITOR"

    mon_stale = audit.classify_prog061_sites(
        audit.PATCH_SUM_BRA,
        audit.PATCH_DEVINFO_BRA,
        audit.PATCH_A2CD_UART_MONITOR,
        audit.ORIG_A2BC_BGO_CALL,
    )
    assert mon_stale["mode"] == "CUSTOM_OR_PARTIAL"
    assert mon_stale["sites"]["0x5040A2CD_bgo_init_fail_hook"] == "UART_MONITOR_HOOK_CONDITIONAL_STALE"

    factory_main = audit.classify_main3410_sites(
        audit.ORIG_FPC_STATUS_CTRL,
        audit.ORIG_ODD_STATE_BRANCH,
        audit.ORIG_ODD_INIT_STATUS,
    )
    assert factory_main["mode"] == "FACTORY_ORIGINAL"

    patched_main = audit.classify_main3410_sites(
        audit.PATCH_FPC_STATUS_CTRL,
        audit.PATCH_ODD_STATE_BRANCH,
        audit.PATCH_ODD_INIT_STATUS,
    )
    assert patched_main["mode"] == "AV_PASSTHROUGH"

    blob = bytearray(b"\x00" * 16 + b"SetBusWidth -91006" + b"\x00" * 8 + struct.pack("<I", 0x89001850))
    kw_hits = audit.scan_ascii_keywords(blob, 0x44027800)
    assert kw_hits["SetBusWidth"] == ["0x44027810"]
    assert kw_hits["-91006"] == ["0x4402781C"]
    mmio_hits = audit.scan_le32_constants(blob, 0x44027800)
    assert mmio_hits["SDCD_BUS_WIDTH_CFG"] == ["0x4402782A"]


def test_mn10300_frames_loops_and_ghidra_fix_decompilation() -> None:
    """Verify MN103S call frames, Bcc, SETLB loops, and Ghidra MN103 fix helpers."""
    import importlib

    buf = bytearray([0xCB] * 0x80)
    buf[0x40] = 0xCD
    struct.pack_into("<hBB", buf, 0x41, -0x30, 0x98, 0x24)
    buf[0x50] = 0xC8
    struct.pack_into("<b", buf, 0x51, 0x08)
    buf[0x60] = 0xDB
    buf[0x65] = 0xD9

    raw = bytes(buf)
    frames = decode_call_frames(raw, 0x57000000)
    assert len(frames) == 1
    assert frames[0].target_va == 0x57000010
    assert frames[0].frame_bytes == 0x24
    assert frames[0].saved_groups == ("d2", "a3", "other")

    bccs = decode_bcc_refs(raw, 0x57000000, 0x5700004E, 0x57000056)
    assert [(b.mnemonic, b.target_va) for b in bccs] == [("beq", 0x57000058)]

    loops = decode_setlb_loops(raw, 0x57000000)
    assert len(loops) == 1
    assert loops[0].mnemonic == "lne"
    assert loops[0].loop_body_va == 0x57000061

    fix_mod = importlib.import_module("tools.ghidra.ghidra_mn103_fix_decompilation")
    assert fix_mod.decode_mn103_regmask(0x98) == ("d2", "a3", "other")
    assert fix_mod.extract_mn103_call_frame([0xCD, 0xD0, 0xFF, 0x98, 0x24]) == (0x98, 0x24)
    assert fix_mod.extract_mn103_call_frame([0xDD, 0x01, 0x00, 0x00, 0x00, 0xA0, 0x10]) == (0xA0, 0x10)
    assert fix_mod.classify_stack_purge(-0x80000000) == "UNKNOWN"
    assert fix_mod.classify_stack_purge(0) == "OK"
    assert hasattr(fix_mod, "resolve_all_seven_traps")
    assert len(fix_mod.KNOWN_DRV_ATAPI_SLOTS) >= 5


def test_mn103_traps_1_to_7_helpers() -> None:
    """Verify static Python Trap 1/4/5/6/7 helpers in scripts.re.mn103_traps."""
    from scripts.re.mn103_traps import (
        InstructionSpan,
        build_instruction_spans,
        detect_mid_instruction_overlaps,
        filter_mid_instruction_constant_hits,
        identify_drv_revision,
        is_cfi_command_address,
        scan_code_pointer_tables,
    )

    buf = bytearray(0x60)
    # 0x50400010: JMP32 disp=-0x0F -> target = 0x50400001 (lands inside insn @ 0x50400000..0x50400006)
    buf[0x10] = 0xDC
    struct.pack_into("<i", buf, 0x11, -0x0F)
    # 0x50400020: SETLB .. LNE loop
    buf[0x20] = 0xDB
    buf[0x25] = 0xD9
    spans = build_instruction_spans(bytes(buf), 0x50400000)
    assert any(sp.start_va == 0x50400010 and sp.end_va == 0x50400015 for sp in spans)
    assert any(sp.start_va == 0x50400025 and sp.mnemonic == "lne" for sp in spans)

    overlaps = detect_mid_instruction_overlaps(
        bytes(buf),
        0x50400000,
        extra_spans=(InstructionSpan(0x50400000, 0x50400006, "mov"),),
    )
    assert len(overlaps) == 1
    assert overlaps[0].target_va == 0x50400001

    genuine, false_hits = filter_mid_instruction_constant_hits(
        (0x50400001, 0x50400030),
        (InstructionSpan(0x50400000, 0x50400006, "mov"),),
    )
    assert genuine == (0x50400030,)
    assert false_hits == (0x50400001,)

    assert is_cfi_command_address(0x84200554)
    assert is_cfi_command_address(0x84200AAA)
    assert not is_cfi_command_address(0x847F0000)

    tbl = bytearray(32)
    for idx, va in enumerate((0x50401000, 0x50401100, 0x50401200, 0x40014559, 0x50401300)):
        struct.pack_into("<I", tbl, idx * 4, va)
    ptrs = scan_code_pointer_tables(bytes(tbl), 0x5040EA30, 0x50400000, 0x50440000, min_run=3)
    assert len(ptrs) == 3

    fp = identify_drv_revision(b"\x00" * 16 + b"SLOGON\x00\x00" + b"\x00" * 16)
    assert fp.revision == "DRV_decomp"
    assert fp.pass_handler_va == 0x4013420A
