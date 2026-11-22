"""Analyze physical NOR dumps (`2..16 MiB`) and emit draft `nor_layout` + `model_manifest` JSONs."""

from __future__ import annotations

import struct
from pathlib import Path

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, run_panadiga_json
from scripts.re._discovery_common import (
    build_common_discovery_parser,
    detect_loader_banner,
    emit_report,
    save_json_dict,
    sha256_bytes,
    slugify_identifier,
)


def analyze_nor_bytes(data: bytes, *, model_id: str, source_ref: str = "<memory>") -> JsonDict:
    """Inspect a raw NOR image buffer deterministically and build draft SSOT JSON payloads."""
    size_bytes = len(data)
    digest = sha256_bytes(data)
    banner, version, loader_date, banner_off = detect_loader_banner(data, 0x4_0000)

    extr_offsets: list[JsonValue] = []
    pos = 0
    while True:
        hit = data.find(b"EXTRHEAD", pos)
        if hit < 0:
            break
        tag = data[hit + 8 : hit + 12].decode("ascii", errors="replace") if hit + 12 <= size_bytes else ""
        extr_offsets.append({"offset": hit, "offset_hex": f"0x{hit:06X}", "tag": tag.strip()})
        pos = hit + 8

    bgo_sectors: list[int] = []
    eemul_1284: list[JsonValue] = []
    eemul_f1: list[JsonValue] = []
    ipl_end = min(size_bytes, 0x4_0000)
    for off in range(0, size_bytes - 8, 0x1_0000):
        tag_u16 = struct.unpack_from("<H", data, off)[0]
        if tag_u16 == 0xF101 or (0xF201 <= tag_u16 <= 0xF2FF):
            bgo_sectors.append(off)
        if data[off + 4 : off + 8] == b"1284" or data[off : off + 4] == b"\x00\x00\xF0\x0F":
            eemul_1284.append(f"0x{off:06X}")
        if off >= ipl_end and data[off + 5] == 0xF1:
            eemul_f1.append(f"0x{off:06X}")

    regions: list[JsonValue] = []
    if ipl_end >= 0x1_0000:
        regions.append(
            {
                "kind": "ipl",
                "start_offset": "0x0000_0000",
                "end_offset": f"0x{ipl_end - 1:08X}",
                "policy": "read_only_permanent",
                "tag": banner or "Bootloader IPL",
                "signature": None,
            }
        )
    if bgo_sectors:
        bgo_start = bgo_sectors[0]
        bgo_end = min(size_bytes - 1, bgo_sectors[-1] + 0xFFFF)
        regions.append(
            {
                "kind": "bgo",
                "start_offset": f"0x{bgo_start:08X}",
                "end_offset": f"0x{bgo_end:08X}",
                "policy": "gated_upper_nor",
                "tag": "BGO FTL",
                "signature": {"marker_byte_offset": 0, "marker_value": "0x01"},
            }
        )
    if size_bytes >= 0x1_0000:
        nv_start = size_bytes - 0x1_0000
        regions.append(
            {
                "kind": "eemul",
                "start_offset": f"0x{nv_start:08X}",
                "end_offset": f"0x{size_bytes - 1:08X}",
                "policy": "nvram_sanitize",
                "tag": "EEMUL NVRAM (top 64 KiB)",
                "signature": {"marker_byte_offset": 5, "marker_value": "0xF1"} if eemul_f1 else None,
            }
        )

    slug = slugify_identifier(model_id)
    layout_id = f"{slug}_nor_layout_draft"
    dump_ev: JsonDict = {
        "tier": "physical_dump_verified",
        "sources": [{"kind": "physical_dump", "reference": source_ref, "sha256": digest}],
    }
    pending_ev: JsonDict = {
        "tier": "community_unverified",
        "sources": [{"kind": "audit", "reference": "Pending manual or FRM cross-verification"}],
    }
    nor_layout: JsonDict = {
        "id": layout_id,
        "flash_ref": "unknown_flash",
        "train_ref": "unknown_train",
        "regions": regions,
        "evidence": dump_ev,
    }
    model_draft: JsonDict = {
        "model_id": model_id,
        "family_label": model_id,
        "series": "Discovered DIGA",
        "region_suffix": None,
        "tv_standard": None,
        "hw_code": None,
        "board_ref": None,
        "train_ref": None,
        "nor_layout_ref": layout_id,
        "exact_aliases": [],
        "fixture_ref": f"{slug}.fixture.stub.json",
        "claims": {
            "board": pending_ev,
            "flash_chip": dump_ev,
            "nor_layout": dump_ev,
            "train": pending_ev,
            "eeprom": pending_ev,
        },
    }
    return {
        "model_id": model_id,
        "size_bytes": size_bytes,
        "sha256": digest,
        "loader_banner": banner,
        "loader_version": version,
        "loader_date": loader_date,
        "loader_offset": banner_off,
        "extrhead_entries": extr_offsets,
        "bgo_sector_count_64k": len(bgo_sectors),
        "eemul_1284_offsets_hex": eemul_1284,
        "eemul_f1_offsets_hex": eemul_f1,
        "draft_nor_layout": nor_layout,
        "draft_model_manifest": model_draft,
    }


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint delegating to `panadiga profiles discover-nor` (with pure-Python buffer fallback)."""
    parser = build_common_discovery_parser("Analyze a NOR flash dump and emit draft SSOT layout/model JSONs", include_apply=False)
    parser.add_argument("-i", "--input", type=Path, required=True, help="Input physical NOR dump binary")
    parser.add_argument("--model-id", type=str, default="DMR-UNKNOWN", help="Target DIGA model identifier")
    parser.add_argument("--out-dir", type=Path, default=None, help="Output directory for draft JSON stubs")
    parser.add_argument("--offline-only", action="store_true", help="Analyze strictly in Python without shelling out to `panadiga` CLI")
    args = parser.parse_args(argv)

    if args.offline_only:
        report = analyze_nor_bytes(args.input.read_bytes(), model_id=args.model_id, source_ref=str(args.input))
        if args.out_dir is not None:
            slug = slugify_identifier(args.model_id)
            layout = report["draft_nor_layout"]
            manifest = report["draft_model_manifest"]
            if isinstance(layout, dict):
                save_json_dict(args.out_dir / f"{slug}_nor_layout_draft.json", layout)
            if isinstance(manifest, dict):
                save_json_dict(args.out_dir / f"{slug}.draft.json", manifest)
    else:
        cmd = ["profiles", "discover-nor", "--input", str(args.input), "--model-id", args.model_id]
        if args.out_dir is not None:
            cmd.extend(["--out-dir", str(args.out_dir)])
        cli_val = run_panadiga_json(cmd)
        report = cli_val if isinstance(cli_val, dict) else {"result": cli_val}

    emit_report(report, args.json)
    if args.json is None or str(args.json) != "-":
        print(f"[nor_intake_analyzer] model={args.model_id} size={report.get('size_bytes')} bgo={report.get('bgo_sector_count_64k')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
