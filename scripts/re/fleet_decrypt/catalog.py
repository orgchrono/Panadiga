"""SHA-256 deduplication, catalog emission, fleet orchestration, and locked-container retries."""

from __future__ import annotations

import json
import re
import shutil
import tempfile
from pathlib import Path

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, as_dict, as_list
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import (
    PROFILES_DIR,
    build_model_lookup_from_profiles,
    infer_model_id_from_path,
    slugify_identifier,
)
from scripts.re.fleet_decrypt.discovery import (
    DecryptedArtifact,
    _classify_raw_bin,
    discover_inputs,
    repo_relative_str,
    resolve_facade_hook,
)
from scripts.re.fleet_decrypt.unpacker import decrypt_frm_container

def _safe_token(text: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "_", text).strip("_") or "item"


def _persist_partition_copies(part_files: tuple[str, ...], dest_dir: Path) -> list[JsonValue]:
    if not part_files:
        return []
    dest_dir.mkdir(parents=True, exist_ok=True)
    saved: list[JsonValue] = []
    for raw_p in part_files:
        src = Path(raw_p)
        if not src.is_file():
            continue
        target = dest_dir / src.name
        if not target.exists():
            shutil.copy2(src, target)
        saved.append(str(target))
    return saved


def _merge_artifacts_into_dedup(
    all_artifacts: list[DecryptedArtifact],
    dedup: dict[str, JsonDict],
    unique_dir: Path,
    out_dir: Path,
) -> None:
    for item in all_artifacts:
        sha = item.sha256
        alias_entry: JsonDict = {
            "model_id": item.model_id,
            "source_ref": item.source_ref,
            "module_name": item.module_name,
        }
        existing = dedup.get(sha)
        if existing is not None:
            aliases = as_list(existing.get("alias_sources", []))
            if alias_entry not in aliases:
                aliases.append(alias_entry)
                existing["alias_sources"] = aliases
            continue

        safe_model = _safe_token(item.model_id)
        safe_mod = _safe_token(item.module_name)
        unique_name = f"{safe_model}_{safe_mod}_{sha[:8]}.bin"
        unique_path = unique_dir / unique_name
        unique_path.write_bytes(item.raw_bytes)

        persisted_parts = _persist_partition_copies(
            item.partition_files,
            out_dir / "partitions" / f"{safe_model}_{sha[:8]}",
        )
        entry: JsonDict = dict(item.metadata)
        entry["unique_name"] = unique_name
        entry["unique_file"] = str(unique_path)
        entry["partition_files"] = persisted_parts
        entry["alias_sources"] = [alias_entry]
        dedup[sha] = entry


def _write_fleet_catalog(
    out_dir: Path,
    *,
    input_label: str,
    sources_scanned: int,
    decryptable_sources: int,
    dedup: dict[str, JsonDict],
    locked_containers: list[JsonValue],
) -> JsonDict:
    unique_bins: list[JsonValue] = list(dedup.values())
    catalog: JsonDict = {
        "schema_version": "1.0.0",
        "input": input_label,
        "sources_scanned": sources_scanned,
        "decryptable_source_count": decryptable_sources,
        "unique_binary_count": len(unique_bins),
        "unique_binaries": unique_bins,
        "locked_containers": locked_containers,
        "excluded_non_decryptable": locked_containers,
    }
    (out_dir / "fleet_decrypted_catalog.json").write_text(
        json.dumps(catalog, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    return catalog


def decrypt_fleet(
    input_path: Path,
    out_dir: Path,
    *,
    profiles_dir: Path = PROFILES_DIR,
    key_file: Path | None = None,
    split_main: bool = False,
    extract_rootfs_elfs: bool = False,
    model_hint: str | None = None,
) -> JsonDict:
    """Decrypt containers/NOR dumps under `input_path`, deduplicate by SHA-256, and write catalog."""
    out_dir.mkdir(parents=True, exist_ok=True)
    unique_dir = out_dir / "unique"
    unique_dir.mkdir(parents=True, exist_ok=True)
    model_lookup = build_model_lookup_from_profiles(profiles_dir)

    work_root = Path(tempfile.mkdtemp(prefix="panadiga_fleet_dec_"))
    all_artifacts: list[DecryptedArtifact] = []
    locked_containers: list[JsonValue] = []
    sources_scanned = 0
    decryptable_sources = 0

    try:
        frm_items, raw_items = discover_inputs(input_path, work_root / "archives")
        for idx, (frm_path, display_ref) in enumerate(frm_items):
            sources_scanned += 1
            inferred = model_hint or infer_model_id_from_path(Path(display_ref), profiles_dir, lookup=model_lookup) or "UNKNOWN"
            arts, locked = resolve_facade_hook("decrypt_frm_container", decrypt_frm_container)(
                frm_path,
                display_ref,
                work_root / f"frm_{idx:03d}",
                model_id=inferred,
                key_file=key_file,
                split_main=split_main,
                extract_rootfs_elfs=extract_rootfs_elfs,
            )
            if locked is not None:
                locked_containers.append(locked)
            else:
                decryptable_sources += 1
                all_artifacts.extend(arts)

        for raw_path, display_ref in raw_items:
            sources_scanned += 1
            inferred = model_hint or infer_model_id_from_path(Path(display_ref), profiles_dir, lookup=model_lookup) or "UNKNOWN"
            rec = resolve_facade_hook("_classify_raw_bin", _classify_raw_bin)(raw_path, display_ref, inferred)
            if rec is not None:
                decryptable_sources += 1
                all_artifacts.append(rec)

        dedup: dict[str, JsonDict] = {}
        _merge_artifacts_into_dedup(all_artifacts, dedup, unique_dir, out_dir)
    finally:
        shutil.rmtree(work_root, ignore_errors=True)

    return _write_fleet_catalog(
        out_dir,
        input_label=repo_relative_str(input_path),
        sources_scanned=sources_scanned,
        decryptable_sources=decryptable_sources,
        dedup=dedup,
        locked_containers=locked_containers,
    )


def retry_locked_containers(
    catalog_path: Path,
    out_dir: Path,
    *,
    profiles_dir: Path = PROFILES_DIR,
    key_file: Path | None = None,
    split_main: bool = False,
    extract_rootfs_elfs: bool = False,
) -> JsonDict:
    """Re-attempt decryption ONLY on entries recorded in `locked_containers` (e.g. after adding a new key)."""
    out_dir.mkdir(parents=True, exist_ok=True)
    unique_dir = out_dir / "unique"
    unique_dir.mkdir(parents=True, exist_ok=True)

    prev = as_dict(json.loads(catalog_path.read_text(encoding="utf-8")))
    dedup: dict[str, JsonDict] = {}
    for entry in as_list(prev.get("unique_binaries", [])):
        if isinstance(entry, dict) and isinstance(entry.get("sha256"), str):
            dedup[str(entry["sha256"])] = dict(entry)

    work_root = Path(tempfile.mkdtemp(prefix="panadiga_retry_locked_"))
    still_locked: list[JsonValue] = []
    unlocked_artifacts: list[DecryptedArtifact] = []
    newly_unlocked = 0

    try:
        for idx, item in enumerate(as_list(prev.get("locked_containers", []))):
            if not isinstance(item, dict):
                continue
            raw_ref = str(item.get("path", "") or item.get("origin", ""))
            model_id = str(item.get("model_id", "UNKNOWN"))
            cand = Path(raw_ref)
            if not cand.is_file():
                cand = ROOT / raw_ref
            if not cand.is_file():
                still_locked.append(item)
                continue
            arts, locked = resolve_facade_hook("decrypt_frm_container", decrypt_frm_container)(
                cand,
                raw_ref,
                work_root / f"retry_{idx:03d}",
                model_id=model_id,
                key_file=key_file,
                split_main=split_main,
                extract_rootfs_elfs=extract_rootfs_elfs,
            )
            if locked is not None:
                still_locked.append(locked)
            else:
                newly_unlocked += 1
                unlocked_artifacts.extend(arts)

        _merge_artifacts_into_dedup(unlocked_artifacts, dedup, unique_dir, out_dir)
    finally:
        shutil.rmtree(work_root, ignore_errors=True)

    sources_scanned = int(str(prev.get("sources_scanned", 0)))
    decryptable_sources = int(str(prev.get("decryptable_source_count", 0))) + newly_unlocked
    return _write_fleet_catalog(
        out_dir,
        input_label=str(prev.get("input", repo_relative_str(catalog_path))),
        sources_scanned=sources_scanned,
        decryptable_sources=decryptable_sources,
        dedup=dedup,
        locked_containers=still_locked,
    )

