"""Shared typed helpers for SSOT discovery & profile hydration scripts (`scripts/re/`)."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
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
