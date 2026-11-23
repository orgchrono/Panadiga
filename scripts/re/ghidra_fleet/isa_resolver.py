"""SSOT/ELF ISA resolution, shard selection, and Ghidra headless discovery."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import importlib
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import xml.etree.ElementTree as ET
from pathlib import Path
from types import SimpleNamespace
from typing import Any, Final

from scripts.panadiga_cli_bridge import JsonDict, JsonValue, as_dict, as_list
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import PROFILES_DIR, emit_report, iter_profile_entities

TOOLS_GHIDRA: Final[Path] = ROOT / "tools" / "ghidra"
JAVA_DIR: Final[Path] = TOOLS_GHIDRA / "java"
DEFAULT_SMOKE_FIXTURE: Final[Path] = ROOT / "scripts" / "tests" / "fixtures" / "mn103s_smoke_traps.bin"
EXECUTABLE_KINDS: Final[frozenset[str]] = frozenset(
    {"IPL", "PROG", "DRV", "MAIN", "MAIN_LINUX_KERNEL", "VXWORKS_BAL", "NOR_FULL", "USER_ELF"}
)
VALID_SHARDS: Final[tuple[str, ...]] = ("all", "small", "main_a", "main_b", "main_c")
SUBSYSTEM_TOKENS: Final[tuple[str, ...]] = (
    "bgo",
    "nfrom",
    "ifodev",
    "eemul",
    "sdcd",
    "fat",
    "fmu",
    "digadiag",
    "extrhead",
    "rtsd",
    "davis",
    "lba48",
    "_dev_serial",
    "slogon",
    "pass",
    "atapi",
    "aiof",
    "avdd",
    "tbc",
    "pal-m",
    "devinfo",
    "swup",
    "nand",
    "linux version",
    "start_kernel",
    "fpc",
    "testmode",
    "400gb",
    "m7920",
    "mats",
    "hita",
)


def default_ghidra_home() -> Path | None:
    """Resolve Ghidra installation path from environment variables without hardcoding user paths."""
    for env_key in ("PANADIGA_GHIDRA_PATCHED", "GHIDRA_INSTALL_DIR", "GHIDRA_HOME"):
        val = os.environ.get(env_key, "").strip()
        if val:
            cand = Path(val)
            if cand.is_dir():
                return cand
    return None


def default_project_dir() -> Path:
    """Resolve default Ghidra project directory cleanly via environment or repo scratch dir."""
    env_dir = os.environ.get("PANADIGA_GHIDRA_PROJECT_DIR", "").strip()
    return Path(env_dir) if env_dir else (ROOT / ".ghidra_projects")


def find_analyze_headless(ghidra_home: Path | None) -> Path:
    """Locate `analyzeHeadless` (.bat on Windows) inside `ghidra_home/support/`."""
    resolved = ghidra_home or default_ghidra_home()
    if resolved is None:
        raise FileNotFoundError(
            "Ghidra home not configured: pass --ghidra-home or set PANADIGA_GHIDRA_PATCHED / GHIDRA_INSTALL_DIR"
        )
    exe_name = "analyzeHeadless.bat" if os.name == "nt" else "analyzeHeadless"
    candidate = resolved / "support" / exe_name
    if not candidate.is_file():
        raise FileNotFoundError(f"Ghidra headless launcher missing at {candidate}")
    return candidate


@dataclass(frozen=True)
class IsaResolution:
    """Resolved Ghidra SLEIGH language target from SSOT `profiles/soc_cores/` or ELF header."""

    language_id: str | None
    compiler_id: str
    sleigh_provider: str | None
    default_base_address: str | None
    source: str


def discover_installed_ghidra_languages(ghidra_home: Path) -> dict[str, set[str]]:
    """Parse `*.ldefs` XML files under `ghidra_home` without booting JVM (< 50 ms)."""
    installed: dict[str, set[str]] = {}
    if not ghidra_home.is_dir():
        return installed
    for ldef in sorted(ghidra_home.rglob("*.ldefs")):
        try:
            tree = ET.parse(ldef)
        except ET.ParseError:
            continue
        for lang in tree.findall(".//language"):
            lid = lang.attrib.get("id", "").strip()
            if not lid:
                continue
            compilers = {
                comp.attrib.get("id", "").strip()
                for comp in lang.findall("./compiler")
                if comp.attrib.get("id", "").strip()
            }
            bucket = installed.setdefault(lid, set())
            bucket.update(compilers or {"default"})
    return installed


def _language_from_isa_string(isa: str, endianness: str = "little") -> tuple[str, str]:
    low = isa.lower()
    if "mn103" in low:
        return "mn10300:LE:32:default", "default"
    if "sparc" in low:
        return ("sparc:LE:32:default" if endianness == "little" else "sparc:BE:32:default"), "default"
    if "arm64" in low or "aarch64" in low or "cortex-a53" in low:
        return "AARCH64:LE:64:v8A", "default"
    if "arm" in low or "cortex-a" in low:
        return "ARM:LE:32:v7", "default"
    if "mips" in low:
        return ("MIPS:BE:32:default" if endianness == "big" else "MIPS:LE:32:default"), "default"
    return "mn10300:LE:32:default", "default"


def _isa_resolution_from_core(core: JsonDict, source: str) -> IsaResolution:
    explicit_lang = core.get("ghidra_language_id")
    explicit_comp = core.get("ghidra_compiler_id")
    if isinstance(explicit_lang, str) and explicit_lang.strip():
        lang_id = explicit_lang.strip()
        comp_id = explicit_comp.strip() if isinstance(explicit_comp, str) and explicit_comp.strip() else "default"
    else:
        lang_id, comp_id = _language_from_isa_string(
            str(core.get("isa", "")), str(core.get("endianness", "little"))
        )
    provider = core.get("ghidra_sleigh_provider")
    default_base = core.get("default_base_address")
    return IsaResolution(
        language_id=lang_id,
        compiler_id=comp_id,
        sleigh_provider=str(provider) if isinstance(provider, str) else None,
        default_base_address=str(default_base) if isinstance(default_base, str) else None,
        source=source,
    )


def build_ssot_isa_map(profiles_dir: Path = PROFILES_DIR) -> dict[str, IsaResolution]:
    """Build normalized `model_id` -> `IsaResolution` map strictly from SSOT board/soc/train chains."""
    cores: dict[str, JsonDict] = {
        str(ent.get("id", "")): ent for _p, ent in iter_profile_entities(profiles_dir, "soc_cores") if ent.get("id")
    }
    socs: dict[str, JsonDict] = {
        str(ent.get("id", "")): ent for _p, ent in iter_profile_entities(profiles_dir, "socs") if ent.get("id")
    }
    boards: dict[str, JsonDict] = {
        str(ent.get("id", "")): ent for _p, ent in iter_profile_entities(profiles_dir, "boards") if ent.get("id")
    }
    trains: dict[str, JsonDict] = {
        str(ent.get("id", "")): ent for _p, ent in iter_profile_entities(profiles_dir, "firmware_trains") if ent.get("id")
    }

    mapping: dict[str, IsaResolution] = {}
    for _mpath, model in iter_profile_entities(profiles_dir, "models"):
        model_id = str(model.get("model_id", "")).strip()
        if not model_id:
            continue

        core: JsonDict | None = None
        source = "ssot"
        board = boards.get(str(model.get("board_ref") or "").strip())
        soc = socs.get(str(board.get("soc_ref") or "").strip()) if board is not None else None
        if soc is not None:
            core = cores.get(str(soc.get("core_ref") or "").strip())

        if core is None:
            train = trains.get(str(model.get("train_ref") or "").strip())
            if train is not None:
                core = cores.get(str(train.get("soc_core_ref") or "").strip())
                source = "ssot_train"

        if core is None:
            continue

        res = _isa_resolution_from_core(core, source)
        mapping[model_id.upper()] = res
        aliases = model.get("exact_aliases")
        if isinstance(aliases, list):
            for alias in aliases:
                if isinstance(alias, str) and alias.strip():
                    mapping[alias.strip().upper()] = res
    return mapping


def _language_from_elf_header(bin_path: Path) -> tuple[str, str] | None:
    if not bin_path.is_file():
        return None
    with bin_path.open("rb") as fh:
        hdr = fh.read(20)
    if len(hdr) < 20 or hdr[:4] != b"\x7fELF":
        return None
    is_le = hdr[5] == 1
    endian = "<H" if is_le else ">H"
    e_machine = struct.unpack_from(endian, hdr, 18)[0]
    if e_machine == 0x28:
        return "ARM:LE:32:v7", "default"
    if e_machine == 0xB7:
        return "AARCH64:LE:64:v8A", "default"
    if e_machine in (0x02, 0x12, 0x2B):
        return "sparc:BE:32:default", "default"
    if e_machine in (0x59, 0x5A):
        return "mn10300:LE:32:default", "default"
    if e_machine in (0x08, 0x0A):
        return ("MIPS:LE:32:default" if is_le else "MIPS:BE:32:default"), "default"
    return None


def resolve_isa_metadata(
    rec: JsonDict,
    profiles_dir: Path = PROFILES_DIR,
    *,
    isa_map: dict[str, IsaResolution] | None = None,
    language_override: str | None = None,
) -> IsaResolution:
    """Resolve Ghidra language strictly from override, SSOT `profiles/`, or ELF header (never guess raw ISAs)."""
    if language_override and language_override.strip():
        return IsaResolution(
            language_id=language_override.strip(),
            compiler_id="default",
            sleigh_provider="override",
            default_base_address=None,
            source="override",
        )

    model_id = str(rec.get("model_id", "")).strip().upper()
    resolved_map = isa_map if isa_map is not None else build_ssot_isa_map(profiles_dir)
    if model_id in resolved_map:
        return resolved_map[model_id]

    unique_file_raw = rec.get("unique_file")
    if isinstance(unique_file_raw, str):
        elf_lang = _language_from_elf_header(Path(unique_file_raw))
        if elf_lang is not None:
            return IsaResolution(
                language_id=elf_lang[0],
                compiler_id=elf_lang[1],
                sleigh_provider="elf_header",
                default_base_address=None,
                source="elf",
            )

    return IsaResolution(
        language_id=None,
        compiler_id="default",
        sleigh_provider=None,
        default_base_address=None,
        source="UNKNOWN_ISA",
    )


def resolve_language_for_record(
    rec: JsonDict,
    profiles_dir: Path = PROFILES_DIR,
    *,
    isa_map: dict[str, IsaResolution] | None = None,
    language_override: str | None = None,
) -> tuple[str | None, str]:
    """Return `(ghidra_language_id, compiler_id)` or `(None, 'default')` when ISA is unmapped."""
    meta = resolve_isa_metadata(rec, profiles_dir, isa_map=isa_map, language_override=language_override)
    return meta.language_id, meta.compiler_id


def is_executable_target(rec: JsonDict, profiles_dir: Path = PROFILES_DIR) -> bool:
    """Filter decrypted catalog records to executable binaries eligible for Ghidra import."""
    kind = str(rec.get("stage_kind", ""))
    name = str(rec.get("unique_name", ""))
    if kind not in EXECUTABLE_KINDS:
        return False
    if "DEV.BIN" in name or "DRV_J147" in name or "DRV1J315" in name:
        return False
    if kind == "NOR_FULL":
        lang_id, _comp = resolve_language_for_record(rec, profiles_dir)
        return lang_id is not None and lang_id.startswith("sparc")
    return True


def select_targets(
    catalog: JsonDict,
    *,
    shard: str = "all",
    filter_pattern: str | None = None,
    profiles_dir: Path = PROFILES_DIR,
    core_shas: set[str] | None = None,
) -> list[JsonDict]:
    """Select ordered executable targets for a given shard and optional regex filter."""
    if shard not in VALID_SHARDS:
        raise ValueError(f"Unsupported shard {shard!r}; expected one of {VALID_SHARDS}")
    excluded_shas = core_shas or set()
    candidates = [
        as_dict(r)
        for r in as_list(catalog.get("unique_binaries", []))
        if isinstance(r, dict)
        and is_executable_target(as_dict(r), profiles_dir)
        and str(r.get("sha256", "")) not in excluded_shas
    ]
    if filter_pattern:
        rx = re.compile(filter_pattern, re.IGNORECASE)
        candidates = [
            r
            for r in candidates
            if rx.search(f"{r.get('unique_name', '')} {r.get('model_id', '')} {r.get('stage_kind', '')}")
        ]

    if shard == "all":
        return candidates
    if shard == "small":
        return [r for r in candidates if str(r.get("stage_kind", "")) != "MAIN"]

    mains = [r for r in candidates if str(r.get("stage_kind", "")) == "MAIN"]
    shard_idx = {"main_a": 0, "main_b": 1, "main_c": 2}[shard]
    return mains[shard_idx::3]

