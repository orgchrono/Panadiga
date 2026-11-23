"""Anti-trap SSOT geometry, sector-table, BGO 4 MiB vs 8 MiB, and binary pre-flight verifier."""

from __future__ import annotations

import os
import sys

if __package__ in (None, ""):
    _here_trap = os.path.dirname(os.path.abspath(__file__))
    sys.path = [os.path.dirname(_here_trap), *[entry for entry in sys.path if os.path.abspath(entry or ".") != _here_trap]]

import re
import json
from pathlib import Path
import argparse
from typing import Final, Sequence

from scripts.panadiga_cli_bridge import JsonDict, JsonValue
from scripts.panadiga_ops import ROOT
from scripts.pico_heartbeat_ops import emit_json_payload

STALE_SA_RE: Final[re.Pattern[str]] = re.compile(
    r"\b(?:SA0\.\.SA3|SA4\.\.SA133|SA38\.\.SA45|SA70\.\.SA77|SA254\.\.SA269)\b"
)
_MIB_MAP: Final[dict[str, int]] = {
    "4MB": 0x0040_0000,
    "8MB": 0x0080_0000,
    "16MB": 0x0100_0000,
}


def parse_hex_int(raw: object) -> int:
    """Parse an integer or `0x...` hex string (allowing underscores)."""
    if isinstance(raw, int) and not isinstance(raw, bool):
        return raw
    if isinstance(raw, str):
        cleaned = raw.strip().replace("_", "")
        return int(cleaned, 16) if cleaned.lower().startswith("0x") else int(cleaned, 10)
    raise ValueError(f"Unsupported integer representation: {raw!r}")


def parse_declared_geometry_tag(tag: str) -> tuple[int | None, int | None]:
    """Extract declared `(capacity_bytes, base_va)` from tags like `BGO_35SEC_4MB_0X84200000`."""
    cap: int | None = None
    base_va: int | None = None
    for part in tag.strip().split("_"):
        upper = part.upper()
        if upper in _MIB_MAP:
            cap = _MIB_MAP[upper]
        elif upper.startswith("0X") and len(upper) == 10:
            try:
                base_va = int(upper, 16)
            except ValueError:
                pass
    return cap, base_va


def _derive_sector_boundaries(flash: dict[str, object]) -> tuple[int, set[int], set[int], list[str]]:
    errors: list[str] = []
    total_bytes = parse_hex_int(flash["total_bytes"])
    banks = flash.get("sector_banks")
    if not isinstance(banks, list) or not banks:
        return total_bytes, set(), set(), [f"flash '{flash.get('id')}': missing sector_banks"]
    starts: set[int] = set()
    ends: set[int] = set()
    cursor = 0
    for bank in banks:
        if not isinstance(bank, dict):
            continue
        count = int(bank["count"])
        sz = parse_hex_int(bank["sector_size"])
        for _ in range(count):
            starts.add(cursor)
            cursor += sz
            ends.add(cursor)
    if cursor != total_bytes:
        errors.append(
            f"flash '{flash.get('id')}': sector_banks sum {cursor:#x} != total_bytes {total_bytes:#x}"
        )
    return total_bytes, starts, ends, errors


def _load_collection(root: Path, subdir: str) -> dict[str, dict[str, object]]:
    out: dict[str, dict[str, object]] = {}
    target = root / "profiles" / subdir
    if not target.is_dir():
        return out
    for path in sorted(target.glob("*.json")):
        raw = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(raw, dict) and isinstance(raw.get("id"), str):
            out[str(raw["id"])] = raw
    return out


def validate_traps_and_geometries(root: Path = ROOT) -> tuple[int, JsonDict]:
    """Validate `profiles/traps/` and verify all NOR layouts against flash & firmware trains."""
    errors: list[str] = []
    traps_path = root / "profiles" / "traps" / "geometry_and_binary_traps.json"
    traps_count = 0
    if not traps_path.is_file():
        errors.append("Missing profiles/traps/geometry_and_binary_traps.json")
    else:
        traps_raw_text = traps_path.read_text(encoding="utf-8")
        if STALE_SA_RE.search(traps_raw_text):
            errors.append("Stale SA token found in geometry_and_binary_traps.json")
        traps_obj = json.loads(traps_raw_text)
        traps_list = traps_obj.get("traps") if isinstance(traps_obj, dict) else None
        if not isinstance(traps_list, list) or not traps_list:
            errors.append("geometry_and_binary_traps.json must define a non-empty traps list")
        else:
            traps_count = len(traps_list)
            for item in traps_list:
                if not isinstance(item, dict) or not item.get("id"):
                    errors.append("Malformed trap entry missing 'id'")
                    continue
                refs = item.get("evidence_refs", [])
                if isinstance(refs, list):
                    for ref in refs:
                        if isinstance(ref, str) and not (root / ref).exists():
                            errors.append(f"{item['id']}: missing evidence_ref '{ref}'")

    flashes = _load_collection(root, "flash_chips")
    trains = _load_collection(root, "firmware_trains")
    layouts = _load_collection(root, "nor_layouts")

    for layout_id, layout in sorted(layouts.items()):
        layout_text = json.dumps(layout)
        if STALE_SA_RE.search(layout_text):
            errors.append(f"nor_layout '{layout_id}': contains stale SA label")
        flash = flashes.get(str(layout.get("flash_ref", "")))
        train = trains.get(str(layout.get("train_ref", "")))
        if flash is None:
            errors.append(f"nor_layout '{layout_id}': unknown flash_ref")
            continue
        if train is None:
            errors.append(f"nor_layout '{layout_id}': unknown train_ref")
            continue

        total_bytes, starts, ends, sec_errs = _derive_sector_boundaries(flash)
        errors.extend(sec_errs)
        regions_raw = layout.get("regions")
        regions = regions_raw if isinstance(regions_raw, list) else []
        bgo_span: tuple[int, int] | None = None
        prev_end = -1
        sorted_regions = sorted(
            (r for r in regions if isinstance(r, dict)),
            key=lambda r: parse_hex_int(r["start_offset"]),
        )
        for reg in sorted_regions:
            s_off = parse_hex_int(reg["start_offset"])
            e_off = parse_hex_int(reg["end_offset"])
            if s_off <= prev_end:
                errors.append(f"nor_layout '{layout_id}': overlapping region at {s_off:#x}")
            prev_end = e_off
            if s_off not in starts or (e_off + 1) not in ends or e_off >= total_bytes:
                errors.append(
                    f"nor_layout '{layout_id}': misaligned/OOB region {reg.get('kind')} [{s_off:#x}..={e_off:#x}]"
                )
            if reg.get("kind") == "bgo":
                bgo_span = (s_off, e_off)

        diag = train.get("digadiag_spec")
        if isinstance(diag, dict):
            fmt = str(diag.get("format", ""))
            exact_bytes = parse_hex_int(diag.get("exact_bytes", 0))
            if fmt == "raw_256k_prog061" and exact_bytes != 262_144:
                errors.append(f"train '{train['id']}': raw_256k_prog061 requires exact_bytes=262144")
            elif fmt == "extrhead_128k" and exact_bytes != 131_072:
                errors.append(f"train '{train['id']}': extrhead_128k requires exact_bytes=131072")
            cap, base_va = parse_declared_geometry_tag(str(diag.get("bgo_geometry_target", "")))
            if cap is not None and cap != total_bytes:
                errors.append(
                    f"nor_layout '{layout_id}': flash_bytes {total_bytes:#x} != declared target capacity {cap:#x}"
                )
            if base_va is not None and (bgo_span is None or bgo_span[0] != (base_va & 0x00FF_FFFF)):
                errors.append(
                    f"nor_layout '{layout_id}': BGO start mismatches declared base {base_va & 0x00FF_FFFF:#x}"
                )

        bgo_spec = train.get("bgo_retarget_spec")
        if isinstance(bgo_spec, dict):
            words = bgo_spec.get("bgo_table_words", [])
            if isinstance(words, list) and len(words) == 64:
                w12 = parse_hex_int(words[12])
                w13 = parse_hex_int(words[13])
                w15 = parse_hex_int(words[15])
                w31 = parse_hex_int(words[31])
                s_off = w12 & 0x00FF_FFFF
                e_excl = w13 & 0x00FF_FFFF
                if w15 == 0xFFFF_FFFF and (e_excl - s_off) != w31 * 0x1_0000:
                    errors.append(
                        f"train '{train['id']}': BGO span {e_excl - s_off:#x} != word[31]*64KiB ({w31 * 0x10000:#x})"
                    )
                if bgo_span != (s_off, e_excl - 1):
                    errors.append(
                        f"nor_layout '{layout_id}': BGO span {bgo_span} != retarget spec [{s_off:#x}..={e_excl - 1:#x}]"
                    )
            cap, _ = parse_declared_geometry_tag(str(bgo_spec.get("target_geometry_id", "")))
            if cap is not None and cap != total_bytes:
                errors.append(
                    f"nor_layout '{layout_id}': retarget target capacity {cap:#x} != flash_bytes {total_bytes:#x}"
                )

    report: JsonDict = {
        "status": "PASS" if not errors else "FAIL",
        "traps_catalog_count": traps_count,
        "layouts_checked": len(layouts),
        "trains_checked": len(trains),
        "errors": [str(e) for e in errors],
    }
    return (0 if not errors else 1), report


def verify_nor_dump_against_layout(
    dump_path: Path,
    layout_id: str,
    *,
    root: Path = ROOT,
) -> tuple[int, JsonDict]:
    """Read-only pre-flight check verifying NOR binary byte size and markers match SSOT layout."""
    flashes = _load_collection(root, "flash_chips")
    trains = _load_collection(root, "firmware_trains")
    layouts = _load_collection(root, "nor_layouts")
    layout = layouts.get(layout_id)
    if layout is None:
        return 2, {"status": "FAIL", "error": f"Unknown nor_layout '{layout_id}'"}
    flash = flashes.get(str(layout.get("flash_ref", "")))
    train = trains.get(str(layout.get("train_ref", "")))
    if flash is None or train is None:
        return 2, {"status": "FAIL", "error": f"Layout '{layout_id}' has broken foreign keys"}
    if not dump_path.is_file():
        return 2, {"status": "FAIL", "error": f"Dump file not found: {dump_path}"}

    actual_size = dump_path.stat().st_size
    expected_size = parse_hex_int(flash["total_bytes"])
    errors: list[str] = []
    if actual_size != expected_size:
        errors.append(
            f"NOR size mismatch for '{layout_id}': file is {actual_size} B ({actual_size // 1048576} MiB), expected {expected_size} B ({expected_size // 1048576} MiB)"
        )
    data = dump_path.read_bytes()
    fp = train.get("loader_fingerprint")
    if isinstance(fp, dict) and isinstance(fp.get("ascii_marker"), str):
        s_off = parse_hex_int(fp["offset_start"])
        e_off = parse_hex_int(fp["offset_end"])
        marker = str(fp["ascii_marker"]).encode("ascii", errors="ignore")
        if marker and marker not in data[s_off:e_off]:
            errors.append(f"Expected loader_fingerprint {fp['ascii_marker']!r} absent in [{s_off:#x}..{e_off:#x}]")

    return (0 if not errors else 1), {
        "status": "PASS" if not errors else "FAIL",
        "layout_id": layout_id,
        "dump_bytes": actual_size,
        "expected_bytes": expected_size,
        "errors": [str(e) for e in errors],
    }


def main(argv: Sequence[str] | None = None) -> int:
    """CLI entrypoint for SSOT geometry anti-trap checks."""
    parser = argparse.ArgumentParser(description="Verify Panadiga BGO 4MB/8MB geometry & binary traps.")
    parser.add_argument("--check", action="store_true", help="Validate traps catalog and all NOR layouts")
    parser.add_argument("--nor-dump", default=None, help="Optional binary NOR dump for read-only size/marker check")
    parser.add_argument("--layout", default=None, help="Target nor_layout id for --nor-dump")
    parser.add_argument("--json", default="-", help="JSON output path or '-' for stdout")
    args = parser.parse_args(argv)

    if args.nor_dump:
        if not args.layout:
            emit_json_payload({"status": "FAIL", "error": "--layout is required with --nor-dump"}, args.json)
            return 2
        code, payload = verify_nor_dump_against_layout(Path(args.nor_dump), args.layout)
    else:
        code, payload = validate_traps_and_geometries(ROOT)
    emit_json_payload(payload, args.json)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
