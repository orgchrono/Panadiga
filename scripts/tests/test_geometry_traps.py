"""Tests for SSOT anti-trap validation (`4 MiB` vs `8 MiB` BGO, sector math, and dump size check)."""

from __future__ import annotations

import json
from pathlib import Path
import shutil

from scripts.panadiga_ops import ROOT
from scripts.verify_geometry_traps import (
    parse_declared_geometry_tag,
    validate_traps_and_geometries,
    verify_nor_dump_against_layout,
)


def test_parse_declared_geometry_tag_and_live_workspace_pass() -> None:
    assert parse_declared_geometry_tag("BGO_35SEC_4MB_0X84200000") == (0x0040_0000, 0x8420_0000)
    assert parse_declared_geometry_tag("BGO_45SEC_8MB_0X84130000") == (0x0080_0000, 0x8413_0000)

    rc, rep = validate_traps_and_geometries(ROOT)
    assert rc == 0, f"Unexpected trap check errors: {rep}"
    assert rep["status"] == "PASS"
    assert int(str(rep["traps_catalog_count"])) >= 8
    assert int(str(rep["layouts_checked"])) >= 4


def test_rejects_4mb_vs_8mb_mismatch_in_synthetic_ssot_and_dump(tmp_path: Path) -> None:
    shutil.copytree(ROOT / "profiles" / "traps", tmp_path / "profiles" / "traps")
    shutil.copytree(ROOT / "profiles" / "flash_chips", tmp_path / "profiles" / "flash_chips")
    shutil.copytree(ROOT / "profiles" / "firmware_trains", tmp_path / "profiles" / "firmware_trains")
    shutil.copytree(ROOT / "profiles" / "nor_layouts", tmp_path / "profiles" / "nor_layouts")

    # Ensure evidence_refs referenced by traps exist inside synthetic root
    traps_file = tmp_path / "profiles" / "traps" / "geometry_and_binary_traps.json"
    traps_data = json.loads(traps_file.read_text(encoding="utf-8"))
    for entry in traps_data["traps"]:
        for ref in entry["evidence_refs"]:
            target = tmp_path / ref
            if not target.exists():
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text("stub\n", encoding="utf-8")

    layout_path = tmp_path / "profiles" / "nor_layouts" / "loader_ver063_4mb_top_bgo35.json"
    layout = json.loads(layout_path.read_text(encoding="utf-8"))
    layout["flash_ref"] = "spansion_s29jl064h_8mb"
    layout_path.write_text(json.dumps(layout), encoding="utf-8")

    rc_bad, rep_bad = validate_traps_and_geometries(tmp_path)
    assert rc_bad == 1
    assert rep_bad["status"] == "FAIL"

    # Synthetic 4 MiB dump passed into EH55 8 MiB layout must fail pre-flight size check
    fake_4mb = tmp_path / "wrong_4mb_dump.bin"
    fake_4mb.write_bytes(b"\xFF" * 0x0040_0000)
    rc_dump, rep_dump = verify_nor_dump_against_layout(
        fake_4mb, "eh55_ver061_on_s29jl064h", root=ROOT
    )
    assert rc_dump == 1
    assert rep_dump["status"] == "FAIL"
