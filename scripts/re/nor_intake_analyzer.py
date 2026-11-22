"""Analyze physical NOR dumps (`2..16 MiB`) with integrity fault detectors (R6) and emit draft SSOT JSONs."""

from __future__ import annotations

import struct
from pathlib import Path
from typing import Final

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, run_panadiga_json
from scripts.re._discovery_common import (
    build_common_discovery_parser,
    detect_loader_banner,
    emit_report,
    save_json_dict,
    sha256_bytes,
    slugify_identifier,
)

_MIN_NOR_SIZE: Final[int] = 0x20_0000
_MIN_MIRROR_BLOCK: Final[int] = 0x1_0000


def detect_16bit_byteswap(data: bytes) -> bool:
    """Detect byte-swapped 16-bit NOR dumps (`EXTRHEAD` -> `XERTDAEH` or `Loader ver` -> `oLdarev er`)."""
    has_normal = b"EXTRHEAD" in data or b"Loader ver" in data
    if has_normal:
        return False
    return b"XERTDAEH" in data or b"oLdarev er" in data


def detect_address_pin_mirroring(data: bytes) -> int | None:
    """Detect power-of-2 floating-address-pin aliasing across non-erased NOR blocks."""
    n = len(data)
    if n < _MIN_MIRROR_BLOCK * 2 or (n & (n - 1)) != 0:
        return None
    if data.count(0xFF) == n:
        return None
    block = _MIN_MIRROR_BLOCK
    while block <= n // 2:
        first = data[:block]
        if all(data[off : off + block] == first for off in range(block, n, block)):
            return block
        block *= 2
    half = n // 2
    if data[:half] == data[half:]:
        return half
    return None


def verify_extrhead_header_checksum(data: bytes, offset: int) -> JsonDict:
    """Validate 64-byte `EXTRHEAD` structure bounds & ASCII metadata at `offset` to filter FTL ghosts."""
    if offset < 0 or offset + 0x40 > len(data) or data[offset : offset + 8] != b"EXTRHEAD":
        return {"offset": offset, "offset_hex": f"0x{offset:06X}", "header_valid": False, "reason": "bad_magic_or_bounds"}

    tag_raw = data[offset + 8 : offset + 12]
    tag_ok = all(0x20 <= b < 0x7F or b == 0 for b in tag_raw)
    tag = tag_raw.decode("ascii", errors="replace").strip()

    dst_addr = struct.unpack_from("<I", data, offset + 0x0C)[0]
    dst_len = struct.unpack_from("<I", data, offset + 0x10)[0]
    src_len = struct.unpack_from("<I", data, offset + 0x14)[0]
    payload_end = offset + 0x40 + src_len

    reasons: list[str] = []
    if not tag_ok or not tag.strip("\x00 "):
        reasons.append("non_ascii_tag")
    if dst_addr == 0 or dst_addr == 0xFFFF_FFFF:
        reasons.append("invalid_dst_addr")
    if dst_len == 0 or dst_len > 0x0200_0000:
        reasons.append("invalid_dst_len")
    if src_len == 0 or payload_end > len(data):
        reasons.append("truncated_or_invalid_src_len")

    return {
        "offset": offset,
        "offset_hex": f"0x{offset:06X}",
        "tag": tag,
        "dst_addr_hex": f"0x{dst_addr:08X}",
        "dst_len": dst_len,
        "src_len": src_len,
        "header_valid": len(reasons) == 0,
        "reason": "ok" if not reasons else ",".join(reasons),
    }


def detect_eemul_generations(data: bytes) -> list[JsonDict]:
    """Enumerate multiple wear-leveling `EEMUL` microsector (`8 KiB`) and sector (`64 KiB`) generations."""
    generations: list[JsonDict] = []
    n = len(data)
    ipl_end = min(n, 0x4_0000)
    for off in range(ipl_end, n - 16, 0x2000):
        marker_kind: str | None = None
        if data[off + 4 : off + 8] == b"1284":
            marker_kind = "eemul_1284"
        elif data[off : off + 4] == b"\x00\x00\xF0\x0F":
            marker_kind = "eemul_f00f"
        elif data[off + 5] == 0xF1 and data[off : off + 5] != b"\xFF\xFF\xFF\xFF\xFF":
            marker_kind = "eemul_f1"
        elif data[off + 5] == 0xF1 and off >= n - 0x1_0000:
            marker_kind = "eemul_f1_top64k"

        if marker_kind is None:
            continue
        seq_word = struct.unpack_from("<I", data, off + 8)[0] if off + 12 <= n else 0
        state_byte = data[off + 5]
        generations.append(
            {
                "offset_hex": f"0x{off:06X}",
                "sector_kb": 64 if off % 0x1_0000 == 0 else 8,
                "marker": marker_kind,
                "state_byte_hex": f"0x{state_byte:02X}",
                "sequence_hex": f"0x{seq_word:08X}",
                "active": state_byte in (0xF1, 0xFE, 0xFF) or marker_kind == "eemul_1284",
            }
        )
    return generations


def analyze_nor_bytes(data: bytes, *, model_id: str, source_ref: str = "<memory>") -> JsonDict:
    """Inspect a raw NOR image buffer deterministically and build draft SSOT JSON payloads."""
    size_bytes = len(data)
    digest = sha256_bytes(data)
    banner, version, loader_date, banner_off = detect_loader_banner(data, 0x4_0000)

    is_all_ff = size_bytes > 0 and data.count(0xFF) == size_bytes
    byte_swapped_16bit = detect_16bit_byteswap(data)
    mirrored_block_size = detect_address_pin_mirroring(data)

    integrity_warnings: list[str] = []
    if size_bytes < _MIN_NOR_SIZE or (size_bytes & (size_bytes - 1)) != 0:
        integrity_warnings.append(f"invalid_nor_size:{size_bytes}")
    if is_all_ff:
        integrity_warnings.append("all_ff_blank_dump")
    if byte_swapped_16bit:
        integrity_warnings.append("16bit_byteswap_detected")
    if mirrored_block_size is not None:
        integrity_warnings.append(f"address_pin_mirror_0x{mirrored_block_size:X}")

    extr_entries: list[JsonValue] = []
    valid_extr_count = 0
    stale_extr_count = 0
    pos = 0
    while True:
        hit = data.find(b"EXTRHEAD", pos)
        if hit < 0:
            break
        checked = verify_extrhead_header_checksum(data, hit)
        if checked["header_valid"]:
            valid_extr_count += 1
        else:
            stale_extr_count += 1
        extr_entries.append(checked)
        pos = hit + 8

    bgo_sectors: list[int] = []
    eemul_1284: list[JsonValue] = []
    eemul_f1: list[JsonValue] = []
    ipl_end = min(size_bytes, 0x4_0000)
    for off in range(0, max(0, size_bytes - 8), 0x1_0000):
        tag_u16 = struct.unpack_from("<H", data, off)[0]
        if tag_u16 == 0xF101 or (0xF201 <= tag_u16 <= 0xF2FF):
            bgo_sectors.append(off)
        if data[off + 4 : off + 8] == b"1284" or data[off : off + 4] == b"\x00\x00\xF0\x0F":
            eemul_1284.append(f"0x{off:06X}")
        if off >= ipl_end and data[off + 5] == 0xF1:
            eemul_f1.append(f"0x{off:06X}")

    eemul_generations = detect_eemul_generations(data)
    intake_blocked = bool(integrity_warnings)

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
        "tier": "community_unverified" if intake_blocked else "physical_dump_verified",
        "sources": [{"kind": "physical_dump", "reference": source_ref, "sha256": digest}],
    }
    if integrity_warnings:
        dump_ev["notes"] = f"INTAKE BLOCKED: {', '.join(integrity_warnings)}"

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
    integrity_summary: JsonDict = {
        "byte_swapped_16bit": byte_swapped_16bit,
        "mirrored_block_size": mirrored_block_size,
        "is_all_ff": is_all_ff,
        "valid_extrhead_count": valid_extr_count,
        "stale_extrhead_count": stale_extr_count,
        "eemul_generations": [g for g in eemul_generations],
        "warnings": [w for w in integrity_warnings],
    }
    return {
        "model_id": model_id,
        "size_bytes": size_bytes,
        "sha256": digest,
        "loader_banner": banner,
        "loader_version": version,
        "loader_date": loader_date,
        "loader_offset": banner_off,
        "intake_blocked": intake_blocked,
        "integrity_checks": integrity_summary,
        "extrhead_entries": extr_entries,
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
