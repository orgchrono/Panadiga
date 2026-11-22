"""Scan Panasonic `.FRM` packages and hydrate empty T2 (`official_frm_verified`) metadata in `profiles/`."""

from __future__ import annotations

import shutil
import tempfile
from pathlib import Path
from typing import Final

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, PanadigaCliError, as_dict, as_list, run_panadiga_json
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import (
    append_evidence_source,
    build_common_discovery_parser,
    can_mutate_at_tier,
    detect_loader_banner,
    emit_report,
    evidence_tier,
    iter_profile_entities,
    save_json_dict,
    sha256_file,
)

DEFAULT_FRM_ROOT: Final[Path] = ROOT / "firmware_and_images"


def _repo_rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def _extract_va_and_banner(frm_path: Path) -> JsonDict:
    """Delegate `.FRM` module unpacking to `panadiga unbrick extract-frm` inside a temp sandbox."""
    extracted_dir = Path(tempfile.mkdtemp(prefix="panadiga_frm_harvest_"))
    info: JsonDict = {
        "prog_load_va": None,
        "main_load_va": None,
        "drv_load_va": None,
        "loader_date": None,
        "loader_version": None,
        "extracted": False,
    }
    try:
        rep = as_dict(
            run_panadiga_json(
                ["unbrick", "extract-frm", "--frm", str(frm_path), "--out", str(extracted_dir), "--unpack"]
            )
        )
        info["extracted"] = True
        info["container_format"] = rep.get("container_format")
        modules = as_list(rep.get("modules", []))
        for item in modules:
            if not isinstance(item, dict):
                continue
            name = str(item.get("name", "")).upper()
            dst_va = item.get("dst_addr_hex")
            unpacked_path = item.get("unpacked_path") or item.get("decrypted_path")
            if isinstance(dst_va, str) and dst_va.startswith("0x"):
                normalized_va = f"0x{dst_va[2:].upper():>08}"
                if len(normalized_va) == 10:
                    normalized_va = f"{normalized_va[:6]}_{normalized_va[6:]}"
                if name.startswith("PROG") and info["prog_load_va"] is None:
                    info["prog_load_va"] = normalized_va
                elif name.startswith("MAIN") and info["main_load_va"] is None:
                    info["main_load_va"] = normalized_va
                elif name.startswith("DRV") and info["drv_load_va"] is None:
                    info["drv_load_va"] = normalized_va
            if isinstance(unpacked_path, str) and Path(unpacked_path).is_file():
                _banner, ver, dt, _off = detect_loader_banner(Path(unpacked_path).read_bytes(), 0x4_0000)
                if ver and info["loader_version"] is None:
                    info["loader_version"] = ver
                if dt and info["loader_date"] is None:
                    info["loader_date"] = dt
    except PanadigaCliError:
        info["extracted"] = False
    finally:
        shutil.rmtree(extracted_dir, ignore_errors=True)
    return info


def harvest_frm_trains(
    *,
    frm_root: Path = DEFAULT_FRM_ROOT,
    profiles_dir: Path,
    apply: bool = False,
    unpack_missing: bool = False,
) -> JsonDict:
    """Match `.FRM` binaries against `profiles/firmware_trains/*.json` and fill safe T2 metadata."""
    frm_files = sorted(frm_root.rglob("*.FRM")) if frm_root.is_dir() else []
    by_sha: dict[str, Path] = {}
    scanned_records: list[JsonValue] = []
    for frm_path in frm_files:
        digest = sha256_file(frm_path)
        by_sha.setdefault(digest, frm_path)
        scanned_records.append(
            {
                "path": _repo_rel(frm_path),
                "size_bytes": frm_path.stat().st_size,
                "sha256": digest,
            }
        )

    updates: list[JsonValue] = []
    for train_path, train in iter_profile_entities(profiles_dir, "firmware_trains"):
        cur_tier = evidence_tier(train)
        if not can_mutate_at_tier(cur_tier, "official_frm_verified"):
            continue
        frm_spec = train.get("frm")
        if not isinstance(frm_spec, dict):
            continue
        official_sha = frm_spec.get("official_frm_sha256")
        matched_path = by_sha.get(official_sha) if isinstance(official_sha, str) else None
        if matched_path is None:
            continue

        filled_fields: dict[str, JsonValue] = {}
        needs_va = any(
            train.get(k) is None for k in ("prog_load_va", "main_load_va", "drv_load_va", "loader_date")
        )
        if unpack_missing and needs_va:
            ext = _extract_va_and_banner(matched_path)
            for k in ("prog_load_va", "main_load_va", "drv_load_va", "loader_date"):
                val = ext.get(k)
                if train.get(k) is None and isinstance(val, str):
                    train[k] = val
                    filled_fields[k] = val

        ev = train.get("evidence")
        ev_changed = False
        if isinstance(ev, dict):
            ev_changed = append_evidence_source(
                ev,
                kind="official_frm",
                reference=_repo_rel(matched_path),
                sha256=str(official_sha),
                target_tier="official_frm_verified",
            )

        if filled_fields or ev_changed:
            updates.append(
                {
                    "train_id": train.get("id"),
                    "file": train_path.name,
                    "matched_frm": _repo_rel(matched_path),
                    "filled_fields": filled_fields,
                    "evidence_updated": ev_changed,
                }
            )
            if apply:
                save_json_dict(train_path, train)

    return {
        "frm_root": _repo_rel(frm_root),
        "frm_files_scanned": len(frm_files),
        "apply": apply,
        "matched_count": len(updates),
        "updates": updates,
        "containers": scanned_records,
    }


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint for `frm_train_harvester.py`."""
    parser = build_common_discovery_parser("Harvest Panasonic .FRM container metadata into profiles/firmware_trains/")
    parser.add_argument("--frm-root", type=Path, default=DEFAULT_FRM_ROOT, help="Directory holding .FRM images")
    parser.add_argument("--unpack-missing", action="store_true", help="Invoke `panadiga unbrick extract-frm` to populate missing VAs/dates")
    args = parser.parse_args(argv)

    rep = harvest_frm_trains(
        frm_root=args.frm_root,
        profiles_dir=args.profiles_dir,
        apply=bool(args.apply),
        unpack_missing=bool(args.unpack_missing),
    )
    emit_report(rep, args.json)
    if args.json is None or str(args.json) != "-":
        print(
            f"[frm_train_harvester] scanned={rep['frm_files_scanned']} matched_updates={rep['matched_count']} apply={rep['apply']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
