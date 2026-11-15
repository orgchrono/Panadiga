"""Unit tests for Matsushita MN103S and DRV_decomp static reverse-engineering helpers."""

from __future__ import annotations

import struct
import zlib

from scripts.re.mn10300 import (
    decode_branch_call_refs,
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
    assert audit.detect_stage_from_metadata(0x57000000, "EH55_MAIN_3410PQ", (), 13003052) == "MAIN3410PQ"
    assert audit.detect_stage_from_metadata(0, "EH55_MAIN_3060NP", (), 13427852) == "MAIN3060NP"

    auto_bgo = audit.classify_prog061_sites(
        audit.PATCH_SUM_BRA,
        audit.PATCH_DEVINFO_BRA,
        audit.PATCH_A2CD_AUTOFORMAT,
    )
    assert auto_bgo["mode"] == "AUTO_FORMAT_BGO"

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
