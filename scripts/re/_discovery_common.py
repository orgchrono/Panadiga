"""Shared typed helpers for SSOT discovery & profile hydration scripts (`scripts/re/`)."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import shutil
import struct
import subprocess
import zipfile
from pathlib import Path
from typing import Final

from scripts.panadiga_cli_bridge import JsonDict, JsonValue
from scripts.panadiga_ops import ROOT

PROFILES_DIR: Final[Path] = ROOT / "profiles"

TIER_RANK: Final[dict[str, int]] = {
    "community_unverified": 0,
    "service_manual_only": 1,
    "official_frm_verified": 2,
    "physical_dump_verified": 3,
    "bench_verified": 4,
}

_LOADER_RE: Final[re.Pattern[bytes]] = re.compile(rb"Loader ver([^\x00\[\r\n ]+)(?:\s*\[([^\]]+)\])?")


def slugify_identifier(raw: str) -> str:
    """Normalize an identifier (e.g. `DMR-EH55`) to lowercase snake_case (`dmr_eh55`)."""
    cleaned = re.sub(r"[^a-zA-Z0-9]+", "_", raw.strip().lower()).strip("_")
    return cleaned or "unknown"


def sha256_bytes(data: bytes) -> str:
    """Return lowercase SHA-256 hex digest for in-memory bytes."""
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    """Stream lowercase SHA-256 hex digest for a binary file without loading huge images unnecessarily."""
    hasher = hashlib.sha256()
    with path.open("rb") as fh:
        while True:
            chunk = fh.read(1024 * 1024)
            if not chunk:
                break
            hasher.update(chunk)
    return hasher.hexdigest()


def load_json_dict(path: Path) -> JsonDict:
    """Load a UTF-8 JSON file expecting a top-level object."""
    parsed = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(parsed, dict):
        raise ValueError(f"Expected JSON object in {path}")
    return parsed


def save_json_dict(path: Path, payload: JsonDict) -> None:
    """Write a JSON object formatted consistently with `profiles/` manifests."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def iter_profile_entities(profiles_dir: Path, collection: str) -> list[tuple[Path, JsonDict]]:
    """Return `(path, dict)` pairs for every `.json` file in `profiles/<collection>/`."""
    target = profiles_dir / collection
    if not target.is_dir():
        return []
    return [(p, load_json_dict(p)) for p in sorted(target.glob("*.json"))]


def evidence_tier(entity: JsonDict) -> str:
    """Read an entity's `evidence.tier` string (or empty string if absent)."""
    ev = entity.get("evidence")
    if isinstance(ev, dict):
        tier = ev.get("tier")
        if isinstance(tier, str):
            return tier
    return ""


def can_mutate_at_tier(current_tier: str, ceiling_tier: str) -> bool:
    """Enforce monotonic trust policy: never overwrite entities above `ceiling_tier`."""
    return TIER_RANK.get(current_tier, 0) <= TIER_RANK.get(ceiling_tier, 0)


def append_evidence_source(
    evidence: JsonDict,
    *,
    kind: str,
    reference: str,
    sha256: str | None = None,
    notes: str | None = None,
    target_tier: str | None = None,
) -> bool:
    """Add a provenance source to an `evidence` dict without duplicates; monotonically upgrade tier."""
    changed = False
    if target_tier is not None:
        cur = str(evidence.get("tier") or "community_unverified")
        if TIER_RANK.get(target_tier, 0) > TIER_RANK.get(cur, 0):
            evidence["tier"] = target_tier
            changed = True

    raw_sources = evidence.get("sources")
    sources: list[JsonValue] = list(raw_sources) if isinstance(raw_sources, list) else []
    for existing in sources:
        if isinstance(existing, dict) and existing.get("reference") == reference and existing.get("kind") == kind:
            return changed

    entry: JsonDict = {"kind": kind, "reference": reference}
    if sha256 is not None:
        entry["sha256"] = sha256
    if notes is not None:
        entry["notes"] = notes
    sources.append(entry)
    evidence["sources"] = sources
    return True


def detect_loader_banner(data: bytes, scan_limit: int = 0x4_0000) -> tuple[str | None, str | None, str | None, int | None]:
    """Scan bootloader slice for ASCII `Loader verX.YY [YY/MM/DD]` marker."""
    head = data[:scan_limit]
    match = _LOADER_RE.search(head)
    if match is None:
        return None, None, None, None
    version = match.group(1).decode("ascii", errors="replace").strip()
    date = match.group(2).decode("ascii", errors="replace").strip() if match.group(2) else None
    banner = match.group(0).decode("ascii", errors="replace").strip()
    return banner, version, date, match.start()


def build_common_discovery_parser(description: str, *, include_apply: bool = True) -> argparse.ArgumentParser:
    """Create CLI argument parser with uniform `--profiles-dir`, `--json`, and optional `--apply`."""
    parser = argparse.ArgumentParser(description=description)
    parser.add_argument("--profiles-dir", type=Path, default=PROFILES_DIR, help="Path to SSOT profiles/ directory")
    parser.add_argument("--json", type=Path, default=None, help="Write structured JSON report (use `-` for stdout)")
    if include_apply:
        parser.add_argument("--apply", action="store_true", help="Write verified empty fields cleanly back to profiles/")
    return parser


def emit_report(report: JsonDict, out_json: Path | None) -> None:
    """Write report JSON cleanly to `-` (stdout) or an explicit file path."""
    text = json.dumps(report, indent=2, ensure_ascii=False)
    if out_json is None:
        return
    if str(out_json) == "-":
        print(text)
    else:
        out_json.parent.mkdir(parents=True, exist_ok=True)
        out_json.write_text(text + "\n", encoding="utf-8")


PANASONIC_FRM_NAMES: Final[tuple[str, ...]] = (
    "PANA_DVD.FRM",
    "PANA__SD.FRM",
    "PANA_ESD.FRM",
    "PANA_USB.FRM",
    "PANAEUSB.FRM",
    "DIGADIAG.FRM",
)


def is_panasonic_frm_path(path: Path) -> bool:
    """Return True if `path` has a `.FRM` suffix (case-insensitive) or known Panasonic container filename."""
    up = path.name.upper()
    return up.endswith(".FRM") or up in PANASONIC_FRM_NAMES


def shannon_entropy(data: bytes, sample_limit: int = 1_048_576) -> float:
    """Compute Shannon entropy (0.0..8.0 bits/byte) over up to `sample_limit` leading bytes."""
    if not data:
        return 0.0
    sample = data[: min(len(data), sample_limit)]
    counts = [0] * 256
    for byte in sample:
        counts[byte] += 1
    total = float(len(sample))
    return -sum((cnt / total) * math.log2(cnt / total) for cnt in counts if cnt > 0)


def scan_embedded_filesystems(data: bytes) -> JsonDict:
    """Locate Linux `CramFS`, `RomFS`, and `SquashFS` partitions inside a `MAIN` image."""
    hits: list[JsonDict] = []
    pos = 0
    while True:
        idx = data.find(b"\x45\x3D\xCD\x28", pos)
        if idx < 0:
            break
        if idx % 16 == 0 and idx + 64 <= len(data):
            sz, flags = struct.unpack_from("<II", data, idx + 4)
            if data[idx + 16 : idx + 32] == b"Compressed ROMFS" and 4096 <= sz <= len(data) - idx:
                hits.append(
                    {
                        "kind": "CramFS",
                        "offset": idx,
                        "offset_hex": f"0x{idx:08X}",
                        "size_bytes": sz,
                        "flags_hex": f"0x{flags:08X}",
                    }
                )
        pos = idx + 16

    pos = 0
    while True:
        idx = data.find(b"-rom1fs-", pos)
        if idx < 0:
            break
        if idx % 16 == 0 and idx + 32 <= len(data):
            sz = struct.unpack_from(">I", data, idx + 8)[0]
            if 256 <= sz <= len(data) - idx:
                hits.append({"kind": "RomFS", "offset": idx, "offset_hex": f"0x{idx:08X}", "size_bytes": sz})
        pos = idx + 16

    for magic, kind, endian in ((b"hsqs", "SquashFS-LE", "<"), (b"sqsh", "SquashFS-BE", ">")):
        pos = 0
        while True:
            idx = data.find(magic, pos)
            if idx < 0:
                break
            if idx % 4 == 0 and idx + 32 <= len(data):
                s_major = struct.unpack_from(f"{endian}H", data, idx + 28)[0]
                if s_major in (3, 4):
                    hits.append({"kind": kind, "offset": idx, "offset_hex": f"0x{idx:08X}", "version": s_major})
            pos = idx + 4

    cands = [int(str(h["offset"])) for h in hits if int(str(h["offset"])) >= 0x20_0000]
    k_end = 0x42_0000
    if cands:
        first_fs = min(cands)
        k_end = (first_fs & ~0xFFF) if (first_fs & 0xFFF) else first_fs
    fs_records: list[JsonValue] = [dict(h) for h in sorted(hits, key=lambda item: int(str(item["offset"])))]
    return {"kernel_slice_end": k_end, "filesystems": fs_records}


def _register_model_tokens(lookup: dict[str, str], raw_name: str, model_id: str) -> None:
    low = raw_name.strip().lower()
    if not low:
        return
    compact = re.sub(r"[^a-z0-9]+", "", low)
    if len(compact) >= 3:
        lookup.setdefault(compact, model_id)
    for prefix in ("dmr-", "dmp-", "dp-", "dmr_", "dmp_", "dp_", "dmr", "dmp", "dp"):
        if low.startswith(prefix):
            short = re.sub(r"[^a-z0-9]+", "", low[len(prefix) :])
            if len(short) >= 4:
                lookup.setdefault(short, model_id)
            break


def build_model_lookup_from_profiles(profiles_dir: Path) -> dict[str, str]:
    """Build normalized token -> `model_id` lookup dynamically from `profiles/models/*.json`."""
    lookup: dict[str, str] = {}
    for model_path, entity in iter_profile_entities(profiles_dir, "models"):
        model_id = entity.get("model_id")
        if not isinstance(model_id, str) or not model_id.strip():
            continue
        _register_model_tokens(lookup, model_id, model_id)
        _register_model_tokens(lookup, model_path.stem, model_id)
        aliases = entity.get("exact_aliases")
        if isinstance(aliases, list):
            for alias in aliases:
                if isinstance(alias, str):
                    _register_model_tokens(lookup, alias, model_id)
    return dict(sorted(lookup.items(), key=lambda kv: (-len(kv[0]), kv[0])))


def _tail_tokens(path: Path, depth: int = 4) -> tuple[set[str], str]:
    tail_parts = path.parts[-depth:]
    tail_text = "/".join(tail_parts).lower()
    raw_tokens = [tok for tok in re.split(r"[^a-z0-9]+", tail_text) if tok]
    delimited: set[str] = set(raw_tokens)
    for idx in range(len(raw_tokens) - 1):
        delimited.add(raw_tokens[idx] + raw_tokens[idx + 1])
    compact_tail = "".join(raw_tokens)
    return delimited, compact_tail


def infer_model_id_from_path(
    path: Path,
    profiles_dir: Path = PROFILES_DIR,
    *,
    lookup: dict[str, str] | None = None,
) -> str | None:
    """Infer SSOT `model_id` from trailing path tokens using `profiles/models/*.json`."""
    resolved_lookup = lookup if lookup is not None else build_model_lookup_from_profiles(profiles_dir)
    delimited, compact_tail = _tail_tokens(path, depth=4)
    for token, model_id in resolved_lookup.items():
        if token in delimited:
            return model_id
    for token, model_id in resolved_lookup.items():
        if token in compact_tail:
            return model_id
    return None


def _unique_dest(work_dir: Path, base: str) -> Path:
    candidate = work_dir / base
    if not candidate.exists():
        return candidate
    stem = Path(base).stem
    suffix = Path(base).suffix
    idx = 1
    while True:
        candidate = work_dir / f"{stem}_{idx}{suffix}"
        if not candidate.exists():
            return candidate
        idx += 1


def locate_seven_zip(explicit: Path | None = None) -> Path | None:
    """Find `7z`/`7za` executable cleanly across Windows/Linux/macOS."""
    if explicit is not None and explicit.is_file():
        return explicit
    for cand in ("7z", "7za", r"C:\Program Files\7-Zip\7z.exe"):
        found = shutil.which(cand) if not cand.endswith(".exe") else (cand if Path(cand).is_file() else None)
        if found is not None:
            return Path(found)
    return None


def extract_frm_from_archive_or_sfx(
    archive_path: Path,
    work_dir: Path,
    *,
    seven_z: Path | None = None,
) -> list[tuple[Path, str]]:
    """Extract `.FRM`/`.BIN` firmware payloads cleanly from `.zip`, `.7z`, `.rar`, or `.exe` SFX archives."""
    work_dir.mkdir(parents=True, exist_ok=True)
    extracted: list[tuple[Path, str]] = []
    suffix = archive_path.suffix.lower()

    if zipfile.is_zipfile(archive_path):
        with zipfile.ZipFile(archive_path) as zf:
            for member in zf.infolist():
                if member.is_dir():
                    continue
                base = Path(member.filename).name
                if is_panasonic_frm_path(Path(base)) or base.upper().endswith(".BIN"):
                    dest = _unique_dest(work_dir, base)
                    dest.write_bytes(zf.read(member.filename))
                    extracted.append((dest, f"{archive_path.name}!{member.filename}"))
        return extracted

    tool = locate_seven_zip(seven_z)
    if tool is not None and suffix in {".7z", ".rar", ".exe"}:
        proc = subprocess.run(
            [str(tool), "x", "-y", f"-o{work_dir}", str(archive_path)],
            capture_output=True,
            check=False,
        )
        if proc.returncode == 0:
            for item in sorted(work_dir.rglob("*")):
                if item.is_file() and (is_panasonic_frm_path(item) or item.suffix.upper() == ".BIN"):
                    rel = item.relative_to(work_dir).as_posix()
                    extracted.append((item, f"{archive_path.name}!{rel}"))
            return extracted

    if suffix == ".exe" and not extracted:
        raw = archive_path.read_bytes()
        pk_off = raw.find(b"PK\x03\x04")
        if pk_off >= 0:
            tmp_zip = work_dir / f"{archive_path.stem}_embedded.zip"
            tmp_zip.write_bytes(raw[pk_off:])
            return extract_frm_from_archive_or_sfx(tmp_zip, work_dir / "_zip_inner", seven_z=tool)

    return extracted
