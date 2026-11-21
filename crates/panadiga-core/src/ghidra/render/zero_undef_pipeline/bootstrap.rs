//! Bootstrap, JSON SSOT loader, generic binary importer, and measurement helpers
//! for the generated `ghidra_zero_undef_pipeline.py` script.

pub(super) const PY_BOOTSTRAP: &str = r#"r"""Panadiga 4-pass zero-``??`` Ghidra recovery pipeline (CPython 3 + PyGhidra).

Operates ONLY on isolated/cloned Ghidra projects (default ``Panadiga_MN103S_ZeroUndef``)
and the patched Ghidra install clone (``C:\ghidra\staging\ghidra_12.1.4_PATCHED``);
the original ``Panadiga_MN103S`` project and stock ``GhidraMn10300`` language are never touched.

Usage (from repo root)::

    C:\ghidra\.venv\Scripts\python.exe tools\ghidra\ghidra_zero_undef_pipeline.py <stage> [--only NAME]
    C:\ghidra\.venv\Scripts\python.exe tools\ghidra\ghidra_zero_undef_pipeline.py all --binary path\to\fw.bin --model DMR-EH55

Stages: ``import-loader``, ``baseline``, ``pass2``, ``pass3``, ``pass4``, ``verify``, ``all``.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import re
import struct
import sys
import time

REPO = pathlib.Path(__file__).resolve().parents[2]
TOOLS = REPO / "tools" / "ghidra"
# tools/ghidra/java would shadow JPype's top-level `java` package until JPype boots.
sys.path[:] = [p for p in sys.path if os.path.normcase(os.path.abspath(p or os.getcwd())) != os.path.normcase(str(TOOLS))]

GHIDRA_PATCHED = pathlib.Path(os.environ.get("PANADIGA_GHIDRA_PATCHED", r"C:\ghidra\staging\ghidra_12.1.4_PATCHED"))
PROJECT_DIR = os.environ.get("PANADIGA_GHIDRA_PROJECT_DIR", r"C:\ghidra\projects")
PROJECT_NAME = "Panadiga_MN103S_ZeroUndef"
SOURCE_PROJECT_NAME = "Panadiga_MN103S"
REPORT_DIR = pathlib.Path(os.environ.get("PANADIGA_ZERO_UNDEF_REPORTS", str(REPO / "docs" / "audits" / "zero_undef")))
WORK_DIR = pathlib.Path(os.environ.get("PANADIGA_ZERO_UNDEF_WORK", r"C:\ghidra\staging\zero_undef_work"))
SSOT_MAPS_PATH = TOOLS / "ghidra_ssot_maps.json"

DEFAULT_LANGUAGE_ID = "mn10300:LE:32:default"
DEFAULT_COMPILER_ID = "default"
DEFAULT_MODEL = "DMR-EH55"
LOADER_NAME = "eh55_loader_ver061_256k.bin"
LOADER_SRC = REPO / "firmware_and_images" / LOADER_NAME
LOADER_BASE = 0x44000000

_BUILTIN_STAGES = {
    "BOOT.BIN": ("/BOOT.BIN", pathlib.Path(r"C:\ghidra\binaries\BOOT.BIN")),
    LOADER_NAME: ("/" + LOADER_NAME, LOADER_SRC),
    "eh55_frm_PROG061_BOOT.bin": ("/eh55_frm_PROG061_BOOT.bin", pathlib.Path(r"C:\ghidra\binaries\unpacked\eh55_frm_PROG061_BOOT.bin")),
    "DRV_decomp.bin": ("/DRV_decomp.bin", pathlib.Path(r"C:\ghidra\binaries\DRV_decomp.bin")),
    "EH55_DRV_S211.bin": ("/EH55_DRV_S211.bin", pathlib.Path(r"C:\ghidra\binaries\unpacked\EH55_DRV_S211.bin")),
    "EH55_MAIN_3410PQ.bin": ("/EH55_MAIN_3410PQ.bin", pathlib.Path(r"C:\ghidra\binaries\unpacked\EH55_MAIN_3410PQ.bin")),
}
_BUILTIN_STAGE_BASES = {
    "BOOT.BIN": 0x44000000,
    LOADER_NAME: 0x44000000,
    "eh55_frm_PROG061_BOOT.bin": 0x50400000,
    "DRV_decomp.bin": 0x40000000,
    "EH55_DRV_S211.bin": 0x40000000,
    "EH55_MAIN_3410PQ.bin": 0x57000000,
}
_BUILTIN_ENTRY_SEEDS = {
    "BOOT.BIN": (0x44000000, 0x4400C696, 0x4400D32F, 0x440236ED),
    LOADER_NAME: (0x44000000, 0x44000008, 0x44000014, 0x4402A2AE, 0x4400159A, 0x440242B7),
    "eh55_frm_PROG061_BOOT.bin": (
        0x50400000, 0x50400008, 0x50400014, 0x5042D236, 0x50401F3C,
        0x5040A294, 0x5040EA20, 0x50423540, 0x5042358B, 0x504313E6, 0x504009E4,
    ),
    "DRV_decomp.bin": (0x40000000, 0x40000014, 0x40030080),
    "EH55_DRV_S211.bin": (0x40000000, 0x40000014, 0x40001000, 0x40030080, 0x40130EFA),
    "EH55_MAIN_3410PQ.bin": (0x57000000, 0x57000010, 0x57000044, 0x574B5A99, 0x57000D2C, 0x574DD208, 0x5771ABFB),
}
_BUILTIN_MMIO_WINDOWS = (
    ("PANADIGA_MMIO_10", 0x10000000, 0x100FFFFF),
    ("PANADIGA_NOR_CS1_CFI", 0x84000000, 0x847FFFFF),
    ("PANADIGA_AIOF_88", 0x88000000, 0x88FFFFFF),
    ("PANADIGA_ASIC_C0", 0xC0000000, 0xC0FFFFFF),
    ("PANADIGA_ATA_SC0_D4", 0xD4000000, 0xD4FFFFFF),
)
_BUILTIN_TAINTED = (0x50423540, 0x5042358B, 0x5771ABFB, 0x4400C696, 0x4400D32F, 0x440236ED)
_BUILTIN_SPOT_CHECKS = {
    LOADER_NAME: (0x440242B7, 0x4400159A),
    "eh55_frm_PROG061_BOOT.bin": (0x504313E6, 0x504009E4, 0x5040EA2F),
    "EH55_MAIN_3410PQ.bin": (0x57000D2C,),
}
_BUILTIN_GENERIC_RULES = {
    "filename_base_hints": [
        {"pattern": "MAIN", "base_va": "0x57000000"},
        {"pattern": "PROG", "base_va": "0x50400000"},
        {"pattern": "BOOT", "base_va": "0x44000000"},
        {"pattern": "LOADER", "base_va": "0x44000000"},
        {"pattern": "DRV", "base_va": "0x40000000"},
        {"pattern": "NOR", "base_va": "0x80000000"},
        {"pattern": "FLASH", "base_va": "0x80000000"},
    ],
    "default_fallback_base_va": "0x40000000",
    "header_probe_offsets": [0, 4, 8, 12, 16, 20, 24, 28, 32, 48, 64],
}


def _load_ssot_maps(path: pathlib.Path = SSOT_MAPS_PATH) -> dict:
    if not path.is_file():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        return data if isinstance(data, dict) else {}
    except Exception:
        return {}


_SSOT = _load_ssot_maps()
LANGUAGE_ID = _SSOT.get("language_id", DEFAULT_LANGUAGE_ID)
COMPILER_ID = _SSOT.get("compiler_id", DEFAULT_COMPILER_ID)
MODEL_MEMORY_MAPS = _SSOT.get("model_memory_maps", {})
GENERIC_STAGE_RULES = _SSOT.get("generic_stage_rules", _BUILTIN_GENERIC_RULES)
if _SSOT.get("stages"):
    STAGES = {
        s["key"]: (
            s["program_path"],
            (REPO / s["repo_relative_binary"]) if s.get("repo_relative_binary") else pathlib.Path(s["default_binary"]),
        )
        for s in _SSOT["stages"]
    }
    STAGE_BASES = {s["key"]: int(s["base_va"], 0) for s in _SSOT["stages"]}
    STAGE_ENTRY_SEEDS = {s["key"]: tuple(int(v, 0) for v in s.get("entry_seeds", ())) for s in _SSOT["stages"]}
    SPOT_CHECK_OPCODES = {
        s["key"]: tuple(int(v, 0) for v in s["spot_check_opcodes"])
        for s in _SSOT["stages"]
        if s.get("spot_check_opcodes")
    }
else:
    STAGES = dict(_BUILTIN_STAGES)
    STAGE_BASES = dict(_BUILTIN_STAGE_BASES)
    STAGE_ENTRY_SEEDS = dict(_BUILTIN_ENTRY_SEEDS)
    SPOT_CHECK_OPCODES = dict(_BUILTIN_SPOT_CHECKS)

MMIO_WINDOWS = (
    tuple((w["name"], int(w["start"], 0), int(w["end"], 0)) for w in _SSOT["volatile_mmio_windows"])
    if _SSOT.get("volatile_mmio_windows")
    else _BUILTIN_MMIO_WINDOWS
)
TAINTED_FUNCTIONS = (
    tuple(int(v, 0) for v in _SSOT["tainted_functions"])
    if _SSOT.get("tainted_functions")
    else _BUILTIN_TAINTED
)

DECOMP_BAD_TOKENS = ("mn_todo", "halt_baddata", "Bad instruction", "BAD_INSTRUCTION")
PAD_BYTES = (0x00, 0xFF)
ERROR_BM = "Error"
AUTO_BM_TYPE = "Analysis"
AUTO_DATA_CAT = "Panadiga-AutoData"
RESIDUAL_CAT = "Panadiga-Residual"
TRAP4_CAT = "Trap4-MidInsnOverlap"
EXTERNAL_CAT = "Panadiga-ExternalFlow"
S_BOX_SIGNATURE = bytes.fromhex("0001bb9c3f5ec73e36a4929338")
CLI_ENTRY_SIZE = 20
MECHA_RECORD_SIZE = 0x34
MAX_ARRAY_CHUNK = 0x10000
MIN_PTR_RUN = 6
MIN_PAD_RUN = 16
MAX_TINY_PAD = 31
LARGE_IMAGE_BYTES = 2 * 1024 * 1024

ALLOWED_TEXT_BYTES = set(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 _-[]/:.,()%!+=*?<>'\"\t\r\n")
KNOWN_TEXT_PREFIXES = (b"[nfrom]", b"[RTSC]", b"[DAVIS]", b"[SDDIAG]", b"[SDCD]", b"src/")
STRONG_PROLOGUE_RE = re.compile(rb"\xCF[\x00-\xFF]\x3F?\xF8\xFE[\x80-\xFF]", re.S)
STRING_RE = re.compile(rb"(?<![\x20-\x7e\t\r\n])[\x20-\x7e\t\r\n]{5,}\x00")
SJIS_RE = re.compile(rb"(?<![\x20-\x7e\x81-\x9f\xe0-\xfc])(?:[\x20-\x7e]|[\x81-\x9f\xe0-\xfc][\x40-\x7e\x80-\xfc]){3,}\x00")
NONEXIST_ADDR_RE = re.compile(r"memory at ([0-9a-fA-F]+)")
FLOW_FROM_RE = re.compile(r"flow from ([0-9a-fA-F]+)")


def start_ghidra():
    import pyghidra

    pyghidra.start(install_dir=GHIDRA_PATCHED)
    import java.lang  # noqa: F401
    sys.path.append(str(TOOLS))
    return pyghidra


def guard_project(project_name: str = PROJECT_NAME) -> None:
    if project_name.strip().lower() == SOURCE_PROJECT_NAME.lower():
        raise SystemExit("refusing to operate on the read-only source project %s" % SOURCE_PROJECT_NAME)


def to_u32(value: int) -> int:
    return value & 0xFFFFFFFF


def va_addr(prog, va: int):
    return prog.getAddressFactory().getDefaultAddressSpace().getAddress(va)


class tx:
    """Context manager for a Ghidra program transaction."""

    def __init__(self, prog, message: str):
        self.prog = prog
        self.message = message
        self.id = None

    def __enter__(self):
        self.id = self.prog.startTransaction(self.message)
        return self.prog

    def __exit__(self, exc_type, exc, tb):
        self.prog.endTransaction(self.id, exc_type is None)
        return False
"#;
