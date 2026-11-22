"""Scan Panasonic `.FRM` packages (Legacy/SD/Blu-ray/AES) and hydrate T2 metadata in `profiles/`."""

from __future__ import annotations

import shutil
import tempfile
from pathlib import Path
from typing import Final

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, PanadigaCliError, as_list
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import (
    append_evidence_source,
    build_common_discovery_parser,
    build_model_lookup_from_profiles,
    can_mutate_at_tier,
    detect_loader_banner,
    emit_report,
    evidence_tier,
    infer_model_id_from_path,
    is_panasonic_frm_path,
    iter_profile_entities,
    save_json_dict,
    sha256_file,
)
from scripts.re.fleet_decryptor import (
    normalize_va_hex,
    probe_bench_runtime_vas,
    repo_relative_str,
    unpack_frm_via_cli,
)

DEFAULT_FRM_ROOT: Final[Path] = ROOT / "firmware_and_images"


def _format_ssot_va(hex_va: str | None) -> str | None:
    if not hex_va or len(hex_va) != 10 or not hex_va.startswith("0x"):
        return None
    return f"0x{hex_va[2:6]}_{hex_va[6:10]}"


def _extract_va_and_banner(frm_path: Path, *, key_file: Path | None = None) -> JsonDict:
    """Delegate `.FRM` module unpacking to `panadiga frm unpack-all` inside a temp sandbox."""
    extracted_dir = Path(tempfile.mkdtemp(prefix="panadiga_frm_harvest_"))
    info: JsonDict = {
        "prog_load_va": None,
        "main_load_va": None,
        "drv_load_va": None,
        "loader_date": None,
        "loader_version": None,
        "container_format": None,
        "extracted": False,
        "locked": False,
    }
    try:
        rep = unpack_frm_via_cli(frm_path, extracted_dir / "unpack", key_file=key_file)
        bench_vas = probe_bench_runtime_vas(frm_path, extracted_dir / "bench", key_file=key_file)
        info["extracted"] = True
        info["container_format"] = rep.get("format")
        modules = as_list(rep.get("modules", []))
        info["module_count"] = len(modules)
        for item in modules:
            if not isinstance(item, dict):
                continue
            name = str(item.get("name", "")).upper()
            stage_kind = str(item.get("stage_kind", "")).upper()
            unpacked_path = item.get("out_path")
            raw_bytes: bytes | None = None
            if isinstance(unpacked_path, str) and Path(unpacked_path).is_file():
                raw_bytes = Path(unpacked_path).read_bytes()
            sha = None
            if raw_bytes is not None:
                from scripts.re._discovery_common import sha256_bytes

                sha = sha256_bytes(raw_bytes)
                _banner, ver, dt, _off = detect_loader_banner(raw_bytes, 0x4_0000)
                if ver and info["loader_version"] is None:
                    info["loader_version"] = ver
                if dt and info["loader_date"] is None:
                    info["loader_date"] = dt

            dst_norm = normalize_va_hex(item.get("dst_addr_hex")) or (bench_vas.get(sha) if sha else None)
            ssot_va = _format_ssot_va(dst_norm)
            if ssot_va is not None:
                if (name.startswith("PROG") or stage_kind == "PROG") and info["prog_load_va"] is None:
                    info["prog_load_va"] = ssot_va
                elif (name.startswith("MAIN") or stage_kind == "MAIN") and info["main_load_va"] is None:
                    info["main_load_va"] = ssot_va
                elif (name.startswith("DRV") or stage_kind == "DRV") and info["drv_load_va"] is None:
                    info["drv_load_va"] = ssot_va
    except PanadigaCliError as exc:
        info["extracted"] = False
        info["locked"] = True
        info["error"] = str(exc)
    finally:
        shutil.rmtree(extracted_dir, ignore_errors=True)
    return info


def harvest_frm_trains(
    *,
    frm_root: Path = DEFAULT_FRM_ROOT,
    frm_file: Path | None = None,
    profiles_dir: Path,
    key_file: Path | None = None,
    apply: bool = False,
    unpack_missing: bool = False,
) -> JsonDict:
    """Match `.FRM` binaries against `profiles/firmware_trains/*.json` and report onboarding candidates."""
    if frm_file is not None:
        frm_files = [frm_file] if frm_file.is_file() else []
    elif frm_root.is_dir():
        frm_files = sorted(p for p in frm_root.rglob("*") if p.is_file() and is_panasonic_frm_path(p))
    else:
        frm_files = []

    by_sha: dict[str, Path] = {}
    scanned_records: list[JsonValue] = []
    for frm_path in frm_files:
        digest = sha256_file(frm_path)
        by_sha.setdefault(digest, frm_path)
        scanned_records.append(
            {
                "path": repo_relative_str(frm_path),
                "size_bytes": frm_path.stat().st_size,
                "sha256": digest,
            }
        )

    known_train_shas: set[str] = set()
    updates: list[JsonValue] = []
    for train_path, train in iter_profile_entities(profiles_dir, "firmware_trains"):
        frm_spec = train.get("frm")
        if not isinstance(frm_spec, dict):
            continue
        official_sha = frm_spec.get("official_frm_sha256")
        if isinstance(official_sha, str) and official_sha:
            known_train_shas.add(official_sha)
        cur_tier = evidence_tier(train)
        if not can_mutate_at_tier(cur_tier, "official_frm_verified"):
            continue
        matched_path = by_sha.get(official_sha) if isinstance(official_sha, str) else None
        if matched_path is None:
            continue

        filled_fields: dict[str, JsonValue] = {}
        needs_va = any(
            train.get(k) is None for k in ("prog_load_va", "main_load_va", "drv_load_va", "loader_date")
        )
        if unpack_missing and needs_va:
            ext = _extract_va_and_banner(matched_path, key_file=key_file)
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
                reference=repo_relative_str(matched_path),
                sha256=str(official_sha),
                target_tier="official_frm_verified",
            )

        if filled_fields or ev_changed:
            updates.append(
                {
                    "train_id": train.get("id"),
                    "file": train_path.name,
                    "matched_frm": repo_relative_str(matched_path),
                    "filled_fields": filled_fields,
                    "evidence_updated": ev_changed,
                }
            )
            if apply:
                save_json_dict(train_path, train)

    model_lookup = build_model_lookup_from_profiles(profiles_dir)
    unmatched_containers: list[JsonValue] = []
    for digest, frm_path in sorted(by_sha.items(), key=lambda kv: str(kv[1])):
        if digest in known_train_shas:
            continue
        entry: JsonDict = {
            "path": repo_relative_str(frm_path),
            "sha256": digest,
            "size_bytes": frm_path.stat().st_size,
            "inferred_model_id": infer_model_id_from_path(frm_path, profiles_dir, lookup=model_lookup),
        }
        if unpack_missing or frm_file is not None:
            entry["probe"] = _extract_va_and_banner(frm_path, key_file=key_file)
        unmatched_containers.append(entry)

    return {
        "frm_root": repo_relative_str(frm_file if frm_file is not None else frm_root),
        "frm_files_scanned": len(frm_files),
        "apply": apply,
        "matched_count": len(updates),
        "updates": updates,
        "unmatched_count": len(unmatched_containers),
        "unmatched_containers": unmatched_containers,
        "containers": scanned_records,
    }


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint for `frm_train_harvester.py`."""
    parser = build_common_discovery_parser("Harvest Panasonic .FRM container metadata into profiles/firmware_trains/")
    parser.add_argument("--frm-root", type=Path, default=DEFAULT_FRM_ROOT, help="Directory holding .FRM images")
    parser.add_argument("--frm", type=Path, default=None, help="Single .FRM file to inspect/harvest instead of scanning --frm-root")
    parser.add_argument("--key-file", type=Path, default=None, help="Optional `keys.ukf` external keyring path")
    parser.add_argument("--unpack-missing", action="store_true", help="Invoke `panadiga frm unpack-all` to populate missing VAs/dates")
    args = parser.parse_args(argv)

    rep = harvest_frm_trains(
        frm_root=args.frm_root,
        frm_file=args.frm,
        profiles_dir=args.profiles_dir,
        key_file=args.key_file,
        apply=bool(args.apply),
        unpack_missing=bool(args.unpack_missing),
    )
    emit_report(rep, args.json)
    if args.json is None or str(args.json) != "-":
        print(
            f"[frm_train_harvester] scanned={rep['frm_files_scanned']} matched_updates={rep['matched_count']} "
            f"unmatched={rep['unmatched_count']} apply={rep['apply']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
