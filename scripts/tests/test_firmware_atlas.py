"""Tests for auto-updating 100%-disassembled Ghidra Forensic Atlas & drift detection."""

from __future__ import annotations

from pathlib import Path

from scripts.generate_firmware_atlas import (
    classify_hex_constant,
    collect_forensic_atlas,
    render_atlas_markdown,
    sync_or_check_atlas,
)
from scripts.panadiga_cli_bridge import as_dict, as_list
from scripts.panadiga_ops import ROOT


def test_classify_hex_constant_maps_am34_buses() -> None:
    assert classify_hex_constant("0x84130000") == "FLASH_CS1_UNCACHED"
    assert classify_hex_constant("0x807F0000") == "FLASH_CS0"
    assert classify_hex_constant("0x50409DF2") == "SDRAM"
    assert classify_hex_constant("0x89000538") == "MMIO_DAVIS"
    assert classify_hex_constant("0xD400200C") == "MMIO_SC0"
    assert classify_hex_constant("0xC000100C") == "MMIO_SYS"


def test_collect_forensic_atlas_and_drift_detection(tmp_path: Path) -> None:
    atlas = collect_forensic_atlas(ROOT)
    summary = as_dict(atlas["summary"])
    assert int(str(summary["mn103s_zero_undef_programs"])) == 53
    assert int(str(summary["doc_functions_indexed"])) > 40
    assert int(str(summary["hex_constants_indexed"])) > 20
    assert int(str(summary["bitwise_patterns_indexed"])) > 10
    assert int(str(summary["headers_and_footers_indexed"])) >= 10

    md_first = render_atlas_markdown(atlas)
    md_second = render_atlas_markdown(collect_forensic_atlas(ROOT))
    assert md_first == md_second

    hdr_names = {str(as_dict(h)["artifact"]) for h in as_list(atlas["headers_and_footers"])}
    assert {"EXTRHEAD", "EXTRFOOT", "MainListHeader"}.issubset(hdr_names)

    json_out = tmp_path / "atlas.json"
    md_out = tmp_path / "ATLAS.md"
    rc_w, rep_w = sync_or_check_atlas(root=ROOT, json_out=json_out, md_out=md_out, write=True)
    assert rc_w == 0
    assert rep_w["status"] == "UPDATED"

    rc_ok, rep_ok = sync_or_check_atlas(root=ROOT, json_out=json_out, md_out=md_out, write=False)
    assert rc_ok == 0
    assert rep_ok["status"] == "IN_SYNC"

    md_out.write_text("# drifted\n", encoding="utf-8")
    rc_drift, rep_drift = sync_or_check_atlas(root=ROOT, json_out=json_out, md_out=md_out, write=False)
    assert rc_drift == 1
    assert rep_drift["status"] == "DRIFT_DETECTED"


def test_live_workspace_forensic_atlas_is_in_sync() -> None:
    rc, rep = sync_or_check_atlas(root=ROOT, write=False)
    assert rc == 0, f"Forensic atlas drift detected: {rep}"
    assert rep["status"] == "IN_SYNC"
