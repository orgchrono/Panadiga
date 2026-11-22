r"""Panadiga 4-pass zero-``??`` Ghidra recovery pipeline (CPython 3 + PyGhidra).

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

class Image:
    """Raw firmware bytes validated against the program's single initialized block."""

    def __init__(self, prog, path):
        blocks = [b for b in prog.getMemory().getBlocks() if b.isInitialized()]
        if len(blocks) != 1:
            raise SystemExit("%s: expected 1 initialized block, got %d" % (prog.getName(), len(blocks)))
        blk = blocks[0]
        self.base = to_u32(blk.getStart().getOffset())
        self.size = int(blk.getSize())
        self.raw = pathlib.Path(path).read_bytes()
        if len(self.raw) != self.size:
            raise SystemExit("%s: file size %d != block size %d" % (prog.getName(), len(self.raw), self.size))
        mem = prog.getMemory()
        for off in (0, self.size // 2, self.size - 1):
            if (mem.getByte(va_addr(prog, self.base + off)) & 0xFF) != self.raw[off]:
                raise SystemExit("%s: memory mismatch at +0x%X" % (prog.getName(), off))
        self.code_hi = self._compute_code_hi()

    def _compute_code_hi(self) -> int:
        if self.base == 0x40000000:
            return self.end
        last = None
        for m in STRONG_PROLOGUE_RE.finditer(self.raw):
            last = m.start()
        return min(self.end, self.base + last + 0x2000) if last is not None else self.end

    @property
    def end(self) -> int:
        return self.base + self.size

    def contains(self, va: int) -> bool:
        return self.base <= va < self.end

    def slice(self, va_lo: int, va_hi: int) -> bytes:
        return self.raw[va_lo - self.base : va_hi - self.base]

    def u16(self, va: int) -> int:
        return struct.unpack_from("<H", self.raw, va - self.base)[0]

    def u32(self, va: int) -> int:
        return struct.unpack_from("<I", self.raw, va - self.base)[0]


def initialized_set(prog):
    from ghidra.program.model.address import AddressSet

    result = AddressSet()
    for blk in prog.getMemory().getBlocks():
        if blk.isInitialized():
            result.add(blk.getStart(), blk.getEnd())
    return result


def undefined_ranges(prog):
    """Return list of (start, end) inclusive integer ranges of undefined initialized bytes."""
    from ghidra.util.task import TaskMonitor

    undef = prog.getListing().getUndefinedRanges(initialized_set(prog), False, TaskMonitor.DUMMY)
    return [(to_u32(r.getMinAddress().getOffset()), to_u32(r.getMaxAddress().getOffset())) for r in undef.getAddressRanges()]


def clip_to_undefined(prog, lo: int, hi: int):
    """Return undefined sub-ranges inside [lo, hi]."""
    from ghidra.program.model.address import AddressSet
    from ghidra.util.task import TaskMonitor

    want = AddressSet(va_addr(prog, lo), va_addr(prog, hi))
    undef = prog.getListing().getUndefinedRanges(want, False, TaskMonitor.DUMMY)
    return [(to_u32(r.getMinAddress().getOffset()), to_u32(r.getMaxAddress().getOffset())) for r in undef.getAddressRanges()]


def count_bytes(ranges) -> int:
    return sum(end - start + 1 for start, end in ranges)


def instruction_bytes(prog) -> int:
    total = 0
    it = prog.getListing().getInstructions(True)
    while it.hasNext():
        total += it.next().getLength()
    return total


def scan_mn_todo(prog):
    """Return (total_count, sample_hits, in_function_count) for instructions emitting ``mn_todo``."""
    cache = {}
    hits = []
    count = 0
    in_func = 0
    fm = prog.getFunctionManager()
    it = prog.getListing().getInstructions(True)
    while it.hasNext():
        ins = it.next()
        raw = ins.getBytes()
        key = (len(raw), raw[0] & 0xFF, (raw[1] & 0xFF) if len(raw) > 1 else 0, ins.getMnemonicString())
        bad = cache.get(key)
        if bad is None:
            bad = False
            for op in ins.getPcode():
                if op.getOpcode() == 72:
                    if prog.getLanguage().getUserDefinedOpName(int(op.getInput(0).getOffset())) == "mn_todo":
                        bad = True
                        break
            cache[key] = bad
        if bad:
            count += 1
            fn = fm.getFunctionContaining(ins.getAddress())
            if fn is not None:
                in_func += 1
            if len(hits) < 24:
                hits.append({
                    "va": "0x%08X" % to_u32(ins.getAddress().getOffset()),
                    "mnemonic": "%s %s" % (ins.getMnemonicString(), ",".join(ins.getDefaultOperandRepresentation(i) for i in range(ins.getNumOperands()))),
                    "function": fn.getName() if fn is not None else None,
                })
    return count, hits, in_func


def count_error_bookmarks(prog) -> int:
    total = 0
    it = prog.getBookmarkManager().getBookmarksIterator(ERROR_BM)
    while it.hasNext():
        it.next()
        total += 1
    return total


def measure(prog, with_todo: bool = True) -> dict:
    inits = initialized_set(prog)
    undef = undefined_ranges(prog)
    listing = prog.getListing()
    result = {
        "program": prog.getName(),
        "initialized_bytes": int(inits.getNumAddresses()),
        "undefined_code_bytes": count_bytes(undef),
        "undefined_ranges": len(undef),
        "largest_undefined_ranges": sorted(((e - s + 1, "0x%08X" % s) for s, e in undef), reverse=True)[:12],
        "instructions": int(listing.getNumInstructions()),
        "defined_data_units": int(listing.getNumDefinedData()),
        "functions": int(prog.getFunctionManager().getFunctionCount()),
        "bad_instruction_bookmarks": count_error_bookmarks(prog),
    }
    if with_todo:
        count, hits, in_func = scan_mn_todo(prog)
        result["mn_todo_calls"] = count
        result["mn_todo_in_functions"] = in_func
        result["mn_todo_sample"] = hits
    return result


def write_json(directory: pathlib.Path, name: str, payload) -> pathlib.Path:
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / name
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return path


def _create_uninit_block_if_free(mem, space, name: str, start: int, size: int, r: bool, w: bool, x: bool, vol: bool) -> bool:
    if size <= 0 or mem.getBlock(name) is not None:
        return False
    end = to_u32(start + size - 1)
    if any(to_u32(b.getStart().getOffset()) <= end and start <= to_u32(b.getEnd().getOffset()) for b in mem.getBlocks()):
        return False
    blk = mem.createUninitializedBlock(name, space.getAddress(start), size, False)
    blk.setRead(r)
    blk.setWrite(w)
    blk.setExecute(x)
    blk.setVolatile(vol)
    return True


def infer_base_va(bin_path: pathlib.Path, model_id: str, explicit_base: str | None) -> int:
    if explicit_base is not None:
        return to_u32(int(explicit_base, 0))
    if bin_path.name in STAGE_BASES:
        return STAGE_BASES[bin_path.name]
    upper = bin_path.name.upper()
    if bin_path.is_file():
        with bin_path.open("rb") as fh:
            head = fh.read(0x400)
        if head.startswith(b"\n root=/dev/fma"):
            return 0x90100000
        if "PROG" in upper or "DIGADIAG" in upper:
            for idx in range(max(0, len(head) - 6)):
                if head[idx] == 0xFC and 0xDC <= head[idx + 1] <= 0xDF:
                    imm = struct.unpack_from("<I", head, idx + 2)[0]
                    if 0x50800000 <= imm < 0x50840000:
                        return 0x50800000
                    if 0x50030400 <= imm < 0x50060000:
                        return 0x50030400
    if "DIGADIAG" in upper:
        return 0x50400000
    model_info = MODEL_MEMORY_MAPS.get(model_id, {})
    for pat, field in (("PROG", "prog_load_va"), ("MAIN", "main_load_va"), ("DRV", "drv_load_va")):
        if pat in upper and model_info.get(field):
            va = to_u32(int(model_info[field], 0))
            if va == 0x90400000:
                return 0x50400000
            if va == 0x90202000:
                return 0x40000000
            return va
    for hint in GENERIC_STAGE_RULES.get("filename_base_hints", ()):
        if hint["pattern"].upper() in upper:
            return to_u32(int(hint["base_va"], 0))
    return to_u32(int(GENERIC_STAGE_RULES.get("default_fallback_base_va", "0x40000000"), 0))


def import_binary(
    pyghidra,
    project,
    prog_path: str,
    bin_path: pathlib.Path,
    base_va: int,
    language_id: str = LANGUAGE_ID,
    compiler_id: str = COMPILER_ID,
) -> bool:
    from generic.stl import Pair
    from ghidra.app.util.importer import ProgramLoader
    from ghidra.app.util.opinion import BinaryLoader
    from ghidra.util.task import TaskMonitor
    from java.io import File
    from java.util import ArrayList

    if project.getProjectData().getFile(prog_path) is not None:
        print("[pass1] %s already present in %s" % (prog_path, project.getName()))
        return False
    folder_path = prog_path.rsplit("/", 1)[0] or "/"
    prog_name = prog_path.rsplit("/", 1)[-1]
    args = ArrayList()
    args.add(Pair("Base Address", "0x%X" % base_va))
    loaded = (
        ProgramLoader.builder()
        .source(File(str(bin_path)))
        .project(project)
        .projectFolderPath(folder_path)
        .name(prog_name)
        .language(language_id)
        .compiler(compiler_id)
        .loaders(BinaryLoader)
        .loaderArgs(args)
        .monitor(TaskMonitor.DUMMY)
        .load()
    )
    try:
        loaded.save(TaskMonitor.DUMMY)
        print("[pass1] imported %s -> %s at 0x%X" % (bin_path, prog_path, base_va))
        return True
    finally:
        loaded.close()


def apply_model_memory_layout(project, pyghidra, prog_path: str, base_va: int, model_id: str = DEFAULT_MODEL) -> dict:
    import ghidra_am34_memory_map as memmap
    from ghidra.util.task import TaskMonitor

    templates = []
    model_regions = MODEL_MEMORY_MAPS.get(model_id, {}).get("regions", [])
    if model_regions:
        templates = [
            (r["name"], int(r["start"], 0), int(r["size"], 0), bool(r["read"]), bool(r["write"]), bool(r["execute"]), bool(r["is_volatile"]))
            for r in model_regions
        ]
    elif project.getProjectData().getFile("/BOOT.BIN") is not None:
        with pyghidra.program_context(project, "/BOOT.BIN") as ref:
            templates = [
                (b.getName(), to_u32(b.getStart().getOffset()), int(b.getSize()), b.isRead(), b.isWrite(), b.isExecute(), b.isVolatile())
                for b in ref.getMemory().getBlocks()
                if not b.isInitialized() and b.getName() != "mmio"
            ]

    created = 0
    with pyghidra.program_context(project, prog_path) as prog:
        mem = prog.getMemory()
        space = prog.getAddressFactory().getDefaultAddressSpace()
        with tx(prog, "Panadiga SSOT memory layout (%s)" % model_id):
            ram = mem.getBlock("ram")
            if ram is not None and to_u32(ram.getStart().getOffset()) != base_va:
                mem.moveBlock(ram, space.getAddress(base_va), TaskMonitor.DUMMY)
            for bname, start, size, r, w, x, vol in templates:
                if _create_uninit_block_if_free(mem, space, bname, start, size, r, w, x, vol):
                    created += 1
            if str(prog.getLanguageID()).lower().startswith("mn10300"):
                memmap.setup_memory_map(prog)
        prog.save("Panadiga layout (%s)" % model_id, TaskMonitor.DUMMY)
    return {"blocks_created": created, "model": model_id, "base_va": "0x%08X" % base_va}


def import_loader(pyghidra, project) -> None:
    import_binary(pyghidra, project, "/" + LOADER_NAME, LOADER_SRC, LOADER_BASE, LANGUAGE_ID, COMPILER_ID)


def layout_loader(project, pyghidra) -> None:
    apply_model_memory_layout(project, pyghidra, "/" + LOADER_NAME, LOADER_BASE, DEFAULT_MODEL)

# ---------------------------------------------------------------------------
# Universal Panasonic Decrypt & Dynamic Bench Runtime (`--frm` / `decrypt-runtime`)
# ---------------------------------------------------------------------------

PANA_FEISTEL_SBOX = tuple(bytearray.fromhex(
    "0001bb9c3f5ec73e36a49293389b8d1a3c84f72acb3d70ad30a2c1950339a50e"
    "e6fb091eb9c424575fc35dfa6056fded81f34aff85dbcda0d09d8b264c500c1d"
    "7d77de7a0a0b73e15476e8c228059899294bee87425965b81fdf235863691c79"
    "64c56f133bce08a7b212207fe289212b41497c82bef2438e86029ed8678fa3e7"
    "bcf866224de5bad2daea6125b0c07ba64896dddc7190555b4f4e6dd11134aa1b"
    "f41780b7cc8cd9378a2c94167240ef9ac847f6fc2eb32fbdcfa9151018ec6a6e"
    "c9915378a83aaf0de0973251b1f5272d358814e3fe06d7d6aba1d5e445449f0f"
    "5a83ebca0704b4b652aed4333175d36c62bf6bf9ac46f05c197e68b5f174c6e9"
))

PANA_DVD_KEY_CATALOG = (
    ("old_df72", "df7215c7427fd010"),
    ("le2011_08e0", "08e03d859af9f3ee"),
    ("y2011_af4d", "af4d5b16c298c16d"),
    ("y2012_9407", "9407311b7561f97e"),
    ("y2012_5d88", "5d88719745a4efff"),
    ("y2013_38bb", "38bb0c17d362701f"),
    ("y2013_ec23", "ec238cd791456646"),
    ("hualu_0a1e", "0a1ee6c5bdc4933f"),
    ("y2014_8f8c", "8f8ca88482e43d9b"),
    ("y2014_21fc", "21fca17361e66b4e"),
    ("y2016_fdfe", "fdfe0bee4c90fa2c"),
    ("y2018_06c9", "06c943f3b997f7e0"),
)

_TOC_MAGIC_PREFIXES = (b"PROG", b"DRV ", b"MAIN", b"STB ")


def _ror8(val: int, bits: int) -> int:
    return ((val >> bits) | (val << (8 - bits))) & 0xFF


def _rol8(val: int, bits: int) -> int:
    return ((val << bits) | (val >> (8 - bits))) & 0xFF


def _feistel_f_round(b0: int, b1: int, b2: int, b3: int, k0: int, k1: int, k2: int, k3: int):
    v3, v4 = k1 ^ b1, k0 ^ b0
    x3, x2 = k3 ^ b3, k2 ^ b2
    v1, v5 = x3 >> 2, _ror8(x3, 2)
    v2, v6 = x2 >> 5, _ror8(x2, 5)
    o3 = ((v6 >> 1) | (v1 << 7)) & 0xFF
    o2 = ((v3 & 0x7F) | (v2 << 7)) & 0xFF
    o1 = ((v4 >> 1) | (v3 & 0x80)) & 0xFF
    o0 = ((v5 >> 1) | (v4 << 7)) & 0xFF
    s = PANA_FEISTEL_SBOX[v4 ^ _rol8(v3, 1) ^ v6 ^ v5]
    return (
        (o0 + _rol8(s, 3)) & 0xFF,
        (o1 + _rol8(s, 2)) & 0xFF,
        (o2 + _rol8(s, 1)) & 0xFF,
        (o3 + s) & 0xFF,
    )


def derive_round_key_8b(key_8b: bytes) -> tuple:
    rk = bytearray(32)
    rk[:7] = key_8b[:7]
    rk[1] ^= key_8b[7]
    rk[4] ^= key_8b[7]
    state = (0x35, 0x7D, 0xFB, 0x9A)
    for rnd in range(16):
        off = (rnd & 7) * 4
        state = _feistel_f_round(rk[off], rk[off + 1], rk[off + 2], rk[off + 3], *state)
        rk[off:off + 4] = state
    return tuple(rk)


_PRECOMPUTED_ROUND_KEYS = tuple((name, derive_round_key_8b(bytes.fromhex(hx))) for name, hx in PANA_DVD_KEY_CATALOG)


def _feistel_block(blk: bytes, rk: tuple, decrypt: bool = True) -> bytes:
    l0, l1, l2, l3, r0, r1, r2, r3 = blk[0], blk[1], blk[2], blk[3], blk[4], blk[5], blk[6], blk[7]
    for step in range(16):
        rnd = (15 - step) if decrypt else step
        off = (rnd & 7) * 4
        f0, f1, f2, f3 = _feistel_f_round(l0, l1, l2, l3, rk[off], rk[off + 1], rk[off + 2], rk[off + 3])
        l0, l1, l2, l3, r0, r1, r2, r3 = f0 ^ r0, f1 ^ r1, f2 ^ r2, f3 ^ r3, l0, l1, l2, l3
    return bytes((r0, r1, r2, r3, l0, l1, l2, l3))


def feistel_decrypt_buf(raw: bytes, rk: tuple) -> bytes:
    aligned = len(raw) & ~7
    out = bytearray(raw)
    for off in range(0, aligned, 8):
        out[off:off + 8] = _feistel_block(out[off:off + 8], rk, True)
    return bytes(out)


def feistel_crypt_edges(raw: bytes, span: int, rk: tuple, decrypt: bool = True) -> bytes:
    aligned_span = span & ~7
    aligned_len = len(raw) & ~7
    if aligned_span == 0 or aligned_len == 0:
        return bytes(raw)
    out = bytearray(raw)
    ranges = ((0, aligned_len),) if aligned_len <= 2 * aligned_span else ((0, aligned_span), (aligned_len - aligned_span, aligned_len))
    for lo, hi in ranges:
        for off in range(lo, hi, 8):
            out[off:off + 8] = _feistel_block(out[off:off + 8], rk, decrypt)
    return bytes(out)


def strip_panasonic_outer_header(raw: bytes) -> bytes:
    if len(raw) >= 48 and raw[:12] == b"PANASONIC\x00\x00\x00" and raw[0x2C:0x30] == b"\x00\x00\x00\x00":
        return raw[48:]
    return raw


def adler32_panasonic_strided(data: bytes, step: int = 16) -> int:
    stride = max(1, step)
    s1, s2 = 1, 0
    for b in data[::stride]:
        s1 = (s1 + b) % 65521
        s2 = (s2 + s1) % 65521
    return s1 | (s2 << 16)


def network_decipher(data: bytes) -> bytes:
    out = bytearray(data)
    counter = 0
    for idx, b in enumerate(out):
        if (idx & 0xFF) == 0:
            counter = 904
        out[idx] = b ^ ((counter >> 8) & 0xFF)
        counter = (counter + b + 38563) & 0xFFFFFFFF
    return bytes(out)


def lzss_decompress_pana(data: bytes, expected_size: int) -> bytes:
    win = bytearray(0x1000)
    dst = bytearray()
    src_i, win_pos, flags, end = 0, 0x0FEE, 0, len(data)
    while src_i < end:
        flags >>= 1
        if (flags & 0x100) == 0:
            if src_i >= end:
                break
            flags = data[src_i] | 0xFF00
            src_i += 1
        if (flags & 1) == 0:
            if src_i + 1 >= end:
                break
            b1, b2 = data[src_i], data[src_i + 1]
            src_i += 2
            offset = b1 | ((b2 & 0xF0) << 4)
            for _ in range((b2 & 0x0F) + 3):
                c = win[offset]
                dst.append(c)
                win[win_pos] = c
                win_pos = (win_pos + 1) & 0x0FFF
                offset = (offset + 1) & 0x0FFF
                if expected_size > 0 and len(dst) >= expected_size:
                    return bytes(dst)
        else:
            c = data[src_i]
            src_i += 1
            dst.append(c)
            win[win_pos] = c
            win_pos = (win_pos + 1) & 0x0FFF
            if expected_size > 0 and len(dst) >= expected_size:
                return bytes(dst)
    return bytes(dst)


def detect_feistel_key(raw: bytes):
    toc_bytes = strip_panasonic_outer_header(raw)
    scan_limit = min(len(toc_bytes), 0x2000) & ~7
    for name, rk in _PRECOMPUTED_ROUND_KEYS:
        for base in range(0, min(scan_limit, 0x80), 0x40):
            for off in (base, base + 48, base + 64):
                if off + 8 <= len(toc_bytes):
                    dec = _feistel_block(toc_bytes[off:off + 8], rk, True)
                    if dec.startswith(_TOC_MAGIC_PREFIXES):
                        return name, rk
    return None, None


def _decompress_payload(payload: bytes, algo: int, expected_size: int) -> bytes:
    import gzip
    import lzma
    import zlib

    if algo == 0:
        return zlib.decompress(payload)
    if algo == 1:
        return gzip.decompress(payload)
    if algo == 2:
        return lzss_decompress_pana(payload, expected_size)
    if algo == 3:
        for fmt in (lzma.FORMAT_AUTO, lzma.FORMAT_ALONE, lzma.FORMAT_RAW):
            try:
                return lzma.decompress(payload, format=fmt)
            except Exception:
                continue
        raise ValueError("LZMA decompression failed")
    raise ValueError("Unsupported EXTRHEAD compression algo %d" % algo)


def unpack_extrhead(raw: bytes, preferred_rk: tuple | None = None, depth: int = 0):
    if len(raw) < 0x40:
        return None
    is_plain = raw[:8] == b"EXTRHEAD"
    rk, double_crypt = (preferred_rk, False) if is_plain else (None, False)
    if not is_plain:
        candidates = [(preferred_rk, "preferred")] if preferred_rk is not None else [(k, n) for n, k in _PRECOMPUTED_ROUND_KEYS]
        for cand_rk, _ in candidates:
            d1 = _feistel_block(raw[:8], cand_rk, True)
            if d1 == b"EXTRHEAD":
                rk, double_crypt = cand_rk, False
                break
            d2 = _feistel_block(d1, cand_rk, True)
            if d2 == b"EXTRHEAD":
                rk, double_crypt = cand_rk, True
                break
        if rk is None:
            return None
    hdr = raw[:0x40] if is_plain else feistel_decrypt_buf(raw[:0x40], rk)
    if double_crypt and rk is not None:
        hdr = feistel_decrypt_buf(hdr, rk)
    if hdr[:8] != b"EXTRHEAD":
        return None
    tag = hdr[0x08:0x0C].decode("ascii", errors="replace").strip()
    flag, algo = struct.unpack_from("<HH", hdr, 0x0C)
    dst_len, dst_addr, src_len = struct.unpack_from("<III", hdr, 0x10)
    footer_off = struct.unpack_from("<I", hdr, 0x20)[0]
    hdr_chk, adler_step = struct.unpack_from("<II", hdr, 0x28)
    build_name = hdr[0x30:0x40].split(b"\x00", 1)[0].decode("ascii", errors="replace").strip()
    span = (footer_off + 8) if footer_off >= 0x40 else (0x40 + src_len + 8)
    aligned_end = min(len(raw) & ~7, (span + 7) & ~7)
    work = raw[:aligned_end] if aligned_end >= 0x40 else raw
    if double_crypt and rk is not None:
        work = feistel_decrypt_buf(work, rk)
    if not is_plain and rk is not None:
        if tag == "DRV":
            drv_span = min(0x2800, len(work) & ~7)
            work = feistel_decrypt_buf(work[:drv_span], rk) + work[drv_span:]
        elif tag == "MAIN":
            work = feistel_crypt_edges(work, 0x19000, rk, True)
    comp = work[0x40:0x40 + src_len]
    decomp = _decompress_payload(comp, algo, dst_len)
    chk_ok = adler32_panasonic_strided(decomp, adler_step or 16) == hdr_chk
    if depth < 4 and len(decomp) >= 0x40 and decomp[:8] == b"EXTRHEAD":
        nested = unpack_extrhead(decomp, None, depth + 1)
        if nested is not None:
            nested_dec, nested_meta = nested
            nested_meta["checksum_valid"] = chk_ok and nested_meta["checksum_valid"]
            return nested_dec, nested_meta
    meta = {
        "tag": tag, "flag": flag, "algo": algo, "dst_len": dst_len,
        "dst_addr": "0x%08X" % dst_addr, "build_name": build_name, "checksum_valid": chk_ok,
    }
    return decomp, meta


def _parse_toc_entries(dec: bytes, stride: int, raw_len: int) -> list:
    entries = []
    sum_off = stride - 2
    for idx in range(len(dec) // stride):
        slot = dec[idx * stride:(idx + 1) * stride]
        name = slot[:0x0A].split(b"\x00", 1)[0].decode("ascii", errors="replace").strip()
        if not name:
            continue
        mod_off = struct.unpack_from("<I", slot, 0x0C)[0]
        mod_sz = struct.unpack_from("<I", slot, 0x20)[0]
        if mod_off >= raw_len and stride == 48 and idx == 0:
            alt_off = struct.unpack_from("<I", slot, 0x08)[0]
            if alt_off == 0x1800 and alt_off + mod_sz <= raw_len:
                mod_off = alt_off
        if mod_sz == 0 or mod_off + mod_sz > raw_len:
            continue
        chk_u16 = struct.unpack_from("<H", slot, sum_off)[0]
        sum_ok = (sum(slot[:sum_off]) & 0xFFFF) == chk_u16
        if not sum_ok and stride == 48:
            chk_u32 = struct.unpack_from("<I", slot, 0x2C)[0]
            sum_ok = adler32_panasonic_strided(slot[:0x2C], 1) == chk_u32
        if sum_ok:
            entries.append({"name": name, "offset": mod_off, "size": mod_sz})
    return entries


def _try_main_list(raw: bytes, rk: tuple | None):
    if len(raw) < 20:
        return None
    chk, _unk, lsz, segsz, _ctype = struct.unpack_from("<5I", raw, 0)
    if lsz < 20 or lsz > len(raw) or (lsz - 20) % 8 != 0:
        return None
    hdr_tail = raw[4:lsz]
    if adler32_panasonic_strided(hdr_tail, 1) != chk and (sum(hdr_tail) & 0xFFFFFFFF) != chk:
        return None
    crypt_span = 0x2800 if segsz == 0x2000000 else 0x1400
    cursor, chunks, last_meta = lsz, [], None
    for i in range((lsz - 20) // 8):
        ssz, _ssum = struct.unpack_from("<2I", raw, 20 + i * 8)
        seg = raw[cursor:cursor + ssz]
        if rk is not None and len(seg) >= crypt_span:
            seg = feistel_crypt_edges(seg, crypt_span, rk, True)
        unpacked = unpack_extrhead(seg, rk)
        if unpacked is None:
            return None
        chunks.append(unpacked[0])
        last_meta = unpacked[1]
        cursor += ssz
    return b"".join(chunks), last_meta


def _try_stb_zlib(buf: bytes):
    import zlib

    if len(buf) <= 264 or not buf.startswith(b"CRC:"):
        return None
    hdr_txt = buf[:256].decode("ascii", errors="replace")
    model = None
    for part in hdr_txt.split("\x00"):
        if part.startswith("Model:"):
            model = part[6:].strip()
            break
    payload = buf[256:]
    if "zlib:" in hdr_txt and payload[:1] == b"\x78":
        try:
            dec = zlib.decompress(payload)
        except Exception:
            dec = b""
        if dec:
            return dec, {
                "tag": "STB", "flag": 0, "algo": 0, "dst_len": len(dec),
                "dst_addr": "0x80000400", "build_name": model or "", "checksum_valid": True,
            }
    return payload, {
        "tag": "STB", "flag": 0, "algo": None, "dst_len": len(payload),
        "dst_addr": "0x80000400", "build_name": model or "", "checksum_valid": True,
    }


def parse_frm_toc(raw: bytes, total_len: int | None = None):
    raw_len = len(raw) if total_len is None else total_len
    kname, rk = detect_feistel_key(raw)
    if rk is not None:
        toc_bytes = strip_panasonic_outer_header(raw)
        dec = feistel_decrypt_buf(toc_bytes[:min(len(toc_bytes), 0x2000) & ~7], rk)
        for stride, label in ((64, "Dvd64B"), (48, "Dvd48B")):
            entries = _parse_toc_entries(dec, stride, raw_len)
            if entries:
                return "FRM_FEISTEL_%s" % label, kname, rk, entries
    for stride, label in ((64, "Sd64B"), (48, "Sd48B")):
        entries = _parse_toc_entries(raw[:min(len(raw), 0x2000)], stride, raw_len)
        if entries:
            return "FRM_SD_%s" % label, None, None, entries
    return "RAW_OR_STANDALONE", None, None, []


def scan_dynamic_bench_anchors(raw: bytes, base_va: int, stage_name: str, model_id: str, region_code: str | None) -> dict:
    bgo_tbl, bgo_base = None, None
    for sig_val in (0x84130000, 0x84200000):
        pat = struct.pack("<II", sig_val, 0x84400000)
        pos = raw.find(pat)
        if pos >= 44 and pos % 4 == 0 and pos + 236 <= len(raw):
            if struct.unpack_from("<I", raw, pos - 12)[0] == 0x10000 and struct.unpack_from("<I", raw, pos + 8)[0] == 0x10000:
                bgo_tbl = "0x%08X" % (base_va + pos - 44)
                bgo_base = "0x%08X" % sig_val
                break
    cli_va = None
    cli_pos = raw.find(b"SLOGON\x00\x00")
    if cli_pos >= 0:
        cli_va = "0x%08X" % (base_va + cli_pos)
    sbox_vas = []
    pos = raw.find(S_BOX_SIGNATURE)
    while pos != -1:
        if pos + 256 <= len(raw):
            sbox_vas.append("0x%08X" % (base_va + pos))
        pos = raw.find(S_BOX_SIGNATURE, pos + 1)
    reg_up = (region_code or "").strip().upper()
    tv_std = "PAL-M" if reg_up in ("LB", "LBS") else ("PAL/SECAM" if reg_up == "EE" else ("NTSC" if reg_up in ("P", "VP", "PC", "VPC") else "PAL"))
    vmodesel = "0x0002" if tv_std == "PAL-M" else ("0x0000" if tv_std.startswith("NTSC") else "0x0001")
    return {
        "stage_key": stage_name,
        "base_va": "0x%08X" % base_va,
        "size_bytes": len(raw),
        "bgo_layout_table_va": bgo_tbl,
        "bgo_partition_base": bgo_base,
        "cli_table_va": cli_va,
        "feistel_sbox_vas": sbox_vas,
        "region_interpolation": {"model_id": model_id, "region_code": reg_up or "AUTO", "tv_standard": tv_std, "aiof_vmodesel": vmodesel},
    }


def decrypt_and_prepare_runtime_stages(input_path: pathlib.Path, out_dir: pathlib.Path, model_id: str, region_code: str | None = None):
    raw = input_path.read_bytes()
    out_dir.mkdir(parents=True, exist_ok=True)
    fmt, kname, rk, entries = parse_frm_toc(raw)
    manifest_stages = []
    ghidra_targets = []
    items = []
    for e in entries:
        off, sz = e["offset"], e["size"]
        pad_end = min(len(raw), off + ((sz + 7) & ~7)) if rk is not None else (off + sz)
        items.append((e["name"], raw[off:pad_end], sz, rk is not None))
    if not items:
        up = unpack_extrhead(raw, None)
        items = [(input_path.name, up[0], len(up[0]), False)] if up else [(input_path.name, raw, len(raw), False)]
    for idx, (slot_name, slot_bytes, orig_size, outer_feistel) in enumerate(items):
        unpacked_bytes, meta = slot_bytes[:orig_size], None
        ml = _try_main_list(unpacked_bytes, rk)
        if ml is not None:
            unpacked_bytes, meta = ml
        else:
            dec_slot = feistel_decrypt_buf(slot_bytes, rk)[:orig_size] if (outer_feistel and rk is not None) else unpacked_bytes
            ml2 = _try_main_list(dec_slot, rk)
            if ml2 is not None:
                unpacked_bytes, meta = ml2
            else:
                ext = unpack_extrhead(dec_slot, rk) if (not slot_name.upper().startswith("PROG") or dec_slot[:8] == b"EXTRHEAD") else None
                if ext is not None:
                    unpacked_bytes, meta = ext
                else:
                    stb = _try_stb_zlib(dec_slot)
                    if stb is not None:
                        unpacked_bytes, meta = stb
                    else:
                        unpacked_bytes = dec_slot
        slug = re.sub(r"[^A-Za-z0-9]+", "_", slot_name.strip()).strip("_") or ("stage_%d" % idx)
        dst_file = out_dir / ("%02d_%s.bin" % (idx, slug))
        dst_file.write_bytes(unpacked_bytes)
        raw_dst = meta["dst_addr"] if meta else None
        base_override = raw_dst if (raw_dst and int(raw_dst, 0) not in (0, 0x90400000, 0x90202000)) else None
        base_va = infer_base_va(dst_file, model_id, base_override)
        anchors = scan_dynamic_bench_anchors(unpacked_bytes, base_va, slot_name, model_id, region_code)
        anchors["unpacked_path"] = str(dst_file)
        if meta:
            anchors["extrhead"] = meta
        manifest_stages.append(anchors)
        ghidra_targets.append((slug, "/" + dst_file.name, dst_file, base_va))
    manifest = {"input": str(input_path), "container_format": fmt, "feistel_key": kname, "stages": manifest_stages}
    write_json(out_dir, "bench_runtime_manifest.json", manifest)
    return manifest, ghidra_targets

# ---------------------------------------------------------------------------
# Pass 2 - typed data, literal pools, padding
# ---------------------------------------------------------------------------

def mark_auto(prog, va: int, kind: str, length: int, category: str = AUTO_DATA_CAT) -> None:
    prog.getBookmarkManager().setBookmark(va_addr(prog, va), AUTO_BM_TYPE, category, "%s len=%d" % (kind, length))


def set_eol(prog, va: int, text: str) -> None:
    from ghidra.program.model.listing import CodeUnit

    prog.getListing().setComment(va_addr(prog, va), CodeUnit.EOL_COMMENT, text)


def create_byte_array(prog, va: int, count: int) -> bool:
    from ghidra.program.model.data import ArrayDataType, ByteDataType

    listing = prog.getListing()
    done = 0
    while done < count:
        chunk = min(MAX_ARRAY_CHUNK, count - done)
        dt = ByteDataType.dataType if chunk == 1 else ArrayDataType(ByteDataType.dataType, chunk, 1)
        try:
            listing.createData(va_addr(prog, va + done), dt)
        except Exception:
            return False
        done += chunk
    return True


def _type_undefined_bytes(prog, va_lo: int, va_hi: int, kind: str, stats_key: str, stats: dict) -> int:
    total = 0
    for lo, hi in clip_to_undefined(prog, va_lo, va_hi):
        n = hi - lo + 1
        if create_byte_array(prog, lo, n):
            total += n
            stats[stats_key] += n
            mark_auto(prog, lo, kind, n)
    return total


def _type_undefined_array(prog, va: int, count: int, unit_dt, unit_size: int, kind: str, stats_key: str, stats: dict) -> int:
    from ghidra.program.model.data import ArrayDataType

    listing = prog.getListing()
    total = 0
    run_start = None
    run_len = 0

    def flush() -> None:
        nonlocal total, run_start, run_len
        if run_start is None or run_len <= 0:
            return
        dt = unit_dt if run_len == 1 else ArrayDataType(unit_dt, run_len, unit_size)
        try:
            listing.createData(va_addr(prog, run_start), dt)
            n = run_len * unit_size
            total += n
            stats[stats_key] += n
            mark_auto(prog, run_start, kind, n)
        except Exception:
            pass
        run_start = None
        run_len = 0

    for idx in range(count):
        slot = va + idx * unit_size
        if clip_to_undefined(prog, slot, slot + unit_size - 1) == [(slot, slot + unit_size - 1)]:
            if run_start is None:
                run_start = slot
            run_len += 1
        else:
            flush()
            _type_undefined_bytes(prog, slot, slot + unit_size - 1, kind + "_part", stats_key, stats)
    flush()
    return total


def _type_cli_table(prog, img: Image, stats: dict, seeds: set) -> None:
    from ghidra.program.model.data import ArrayDataType, ByteDataType, Pointer32DataType, StructureDataType, UnsignedIntegerDataType

    pos = img.raw.find(b"SLOGON\x00\x00")
    if pos < 0:
        return
    tbl_va = img.base + pos
    count = 0
    for idx in range(160):
        off = tbl_va + idx * CLI_ENTRY_SIZE
        if not img.contains(off + CLI_ENTRY_SIZE - 1):
            break
        name = img.slice(off, off + 8)
        nul = name.find(b"\x00")
        if nul <= 0 or any(c < 0x20 or c > 0x7E for c in name[:nul]):
            break
        count += 1
    if count < 8:
        return

    entry = StructureDataType("panadiga_drv_cli_entry", 0)
    entry.add(ArrayDataType(ByteDataType.dataType, 8, 1), 8, "name", "NUL-padded ASCII command name")
    entry.add(UnsignedIntegerDataType.dataType, 4, "policy_w0", "<8sIII> policy/arg descriptor")
    entry.add(UnsignedIntegerDataType.dataType, 4, "flags", "<8sIII> flags")
    ptr_dt = Pointer32DataType()
    entry.add(ptr_dt, 4, "handler", "<8sIII> command handler pointer")

    listing = prog.getListing()
    for idx in range(count):
        off = tbl_va + idx * CLI_ENTRY_SIZE
        handler = img.u32(off + 16)
        if img.contains(handler):
            seeds.add(handler)
        if clip_to_undefined(prog, off, off + CLI_ENTRY_SIZE - 1) == [(off, off + CLI_ENTRY_SIZE - 1)]:
            try:
                listing.createData(va_addr(prog, off), entry)
                stats["structured_table_bytes"] += CLI_ENTRY_SIZE
                continue
            except Exception:
                pass
        for sub_off, dt in ((8, UnsignedIntegerDataType.dataType), (12, UnsignedIntegerDataType.dataType), (16, ptr_dt)):
            if clip_to_undefined(prog, off + sub_off, off + sub_off + 3) == [(off + sub_off, off + sub_off + 3)]:
                try:
                    listing.createData(va_addr(prog, off + sub_off), dt)
                    stats["structured_table_bytes"] += 4
                except Exception:
                    pass
        _type_undefined_bytes(prog, off, off + 7, "drv_cli_name_tail", "structured_table_bytes", stats)
    mark_auto(prog, tbl_va, "drv_cli_table<8sIII>", count * CLI_ENTRY_SIZE)


def _type_atapi_tables(prog, img: Image, stats: dict, seeds: set) -> None:
    from ghidra.program.model.data import Pointer32DataType

    if img.base != 0x40000000:
        return
    raw = img.raw
    lut_off = None
    for off in range(0xC0000, min(0xF5000, img.size - 256)):
        if raw[off] == 0x1A and raw[off + 3] == 0x01 and raw[off + 0x12] == 0 and raw[off + 0x1B] == 0x0C and raw[off + 0x1E] == 0x0A:
            lut_off = off
            break
    if lut_off is None:
        return
    lut_va = img.base + lut_off
    max_idx = max(raw[lut_off : lut_off + 256])
    ptr_off = lut_off - 4 * (max_idx + 1)
    while ptr_off >= 4 and img.contains(img.u32(img.base + ptr_off - 4)):
        ptr_off -= 4
    ptr_va = img.base + ptr_off
    cnt = (lut_off - ptr_off) // 4
    for idx in range(cnt):
        tgt = img.u32(ptr_va + 4 * idx)
        if img.contains(tgt):
            seeds.add(tgt)
    _type_undefined_array(prog, ptr_va, cnt, Pointer32DataType(), 4, "atapi_handler_table", "pointer_table_bytes", stats)
    _type_undefined_bytes(prog, lut_va, lut_va + 255, "atapi_cdb_lut_256", "structured_table_bytes", stats)
    if lut_va == 0x400DEA64:
        _type_undefined_bytes(prog, 0x400DE35C, 0x400DE4B1, "atapi_sense_lut", "structured_table_bytes", stats)
        _type_undefined_bytes(prog, 0x400DE4B2, 0x400DE695, "atapi_attr3_table", "structured_table_bytes", stats)
        _type_undefined_bytes(prog, 0x400DE696, 0x400DE7DF, "atapi_attr2_table", "structured_table_bytes", stats)


def _valid_mecha_rec(rec: bytes) -> bool:
    code = struct.unpack_from("<H", rec, 0)[0]
    if code == 0xFFFF:
        return False
    body = rec[2:].split(b"\x00", 1)[0]
    if len(body) < 2 or any(b < 0x20 and b not in (9, 10, 13) for b in body):
        return False
    try:
        body.decode("cp932")
    except UnicodeDecodeError:
        return False
    return True


def _type_mecha_table(prog, img: Image, stats: dict) -> None:
    pos = img.raw.find(b"\xa7\x830.5mW")
    if pos < 0:
        return
    start, end = pos, pos + MECHA_RECORD_SIZE
    while start >= MECHA_RECORD_SIZE and _valid_mecha_rec(img.raw[start - MECHA_RECORD_SIZE : start]):
        start -= MECHA_RECORD_SIZE
    while end + MECHA_RECORD_SIZE <= img.size and _valid_mecha_rec(img.raw[end : end + MECHA_RECORD_SIZE]):
        end += MECHA_RECORD_SIZE
    count = (end - start) // MECHA_RECORD_SIZE
    for idx in range(count):
        rec_va = img.base + start + idx * MECHA_RECORD_SIZE
        _type_undefined_bytes(prog, rec_va, rec_va + MECHA_RECORD_SIZE - 1, "drv_mecha_error_rec", "structured_table_bytes", stats)
    set_eol(prog, img.base + start, "[Pass2] CP932 mecha error table (%d x 0x34-byte records)" % count)


def _type_signature_blocks(
    prog, img: Image, sig: bytes, length: int, kind: str, label: str, stats: dict
) -> None:
    pos = img.raw.find(sig)
    while pos != -1:
        va = img.base + pos
        if pos + length <= img.size:
            if _type_undefined_bytes(prog, va, va + length - 1, kind, "structured_table_bytes", stats):
                set_eol(prog, va, label)
        pos = img.raw.find(sig, pos + 1)


def _type_bgo_and_jump_tables(prog, img: Image, stats: dict, seeds: set) -> None:
    from ghidra.program.model.data import Pointer32DataType, UnsignedIntegerDataType

    if img.base in (0x44000000, 0x80000000):
        bgo_tbl_va = img.base + 0x337A4
        bgo_probe_va = img.base + 0x337D0
        if img.contains(bgo_probe_va) and img.u32(bgo_probe_va) in (0x84130000, 0x84200000):
            _type_undefined_array(prog, bgo_tbl_va, 70, UnsignedIntegerDataType.dataType, 4, "bgo_layout_table_u32", "structured_table_bytes", stats)
    for bgo_sig in (b"\x00\x00\x13\x84\x00\x00\x40\x84", b"\x00\x00\x20\x84\x00\x00\x40\x84"):
        pos = img.raw.find(bgo_sig)
        while pos != -1:
            if pos % 4 == 0 and pos >= 44 and pos + 236 <= img.size:
                tbl_va = img.base + pos - 44
                if img.u32(tbl_va + 32) == 0x10000 and img.u32(tbl_va + 52) == 0x10000:
                    _type_undefined_array(prog, tbl_va, 70, UnsignedIntegerDataType.dataType, 4, "bgo_layout_table_u32", "structured_table_bytes", stats)
            pos = img.raw.find(bgo_sig, pos + 4)
    if img.base == 0x44000000 and img.contains(0x44035F07):
        words = [img.u32(0x44035E98 + 4 * i) for i in range(28)]
        if all(img.contains(w) for w in words):
            seeds.update(words)
            _type_undefined_array(prog, 0x44035E98, 28, Pointer32DataType(), 4, "boot_switch_jump_table", "pointer_table_bytes", stats)
    if img.contains(0x5040EA30 + 15):
        words = [img.u32(0x5040EA30 + 4 * i) for i in range(4)]
        if all(img.contains(w) for w in words):
            cnt = 4
            while img.contains(0x5040EA30 + 4 * cnt + 3) and img.contains(img.u32(0x5040EA30 + 4 * cnt)):
                cnt += 1
            for i in range(cnt):
                seeds.add(img.u32(0x5040EA30 + 4 * i))
            _type_undefined_array(prog, 0x5040EA30, cnt, Pointer32DataType(), 4, "prog061_jump_table", "pointer_table_bytes", stats)


def _type_sboxes(prog, img: Image, stats: dict) -> None:
    for sig, length, kind, label in (
        (S_BOX_SIGNATURE, 256, "feistel_sbox", "[Pass2] Feistel S-Box (256 bytes)"),
        (b"EXTRHEAD", 64, "extrhead_container_hdr", "[Pass2] EXTRHEAD container header (64 bytes)"),
    ):
        _type_signature_blocks(prog, img, sig, length, kind, label, stats)


def _looks_textual(blob: bytes, strict: bool = False) -> bool:
    body = blob[:-1]
    if strict:
        if body.startswith(KNOWN_TEXT_PREFIXES):
            return len(body) >= 8 and all(b in ALLOWED_TEXT_BYTES for b in body)
        if len(body) < 10 or any(b not in ALLOWED_TEXT_BYTES for b in body):
            return False
        alnum = sum(1 for c in body if chr(c).isalnum())
        return alnum * 10 >= 7 * len(body)
    letters = sum(1 for c in body if chr(c).isalpha())
    return len(body) >= 8 or letters * 2 >= len(body)


def _type_strings(prog, img: Image, ranges, stats: dict, conservative_text: bool = False) -> None:
    from ghidra.program.model.data import StringDataType

    listing = prog.getListing()
    for lo, hi in ranges:
        chunk = img.slice(lo, hi + 1)
        taken = []
        for m in STRING_RE.finditer(chunk):
            blob = m.group(0)
            start = lo + m.start()
            in_code = start < img.code_hi
            if not _looks_textual(blob, strict=(conservative_text and in_code)):
                continue
            if start > lo and chunk[m.start() - 1] != 0:
                continue
            try:
                listing.createData(va_addr(prog, start), StringDataType.dataType, len(blob))
            except Exception:
                continue
            stats["string_bytes"] += len(blob)
            stats["strings"] += 1
            mark_auto(prog, start, "ascii_string", len(blob))
            taken.append((m.start(), m.end()))
        if conservative_text and hi < img.code_hi:
            continue
        for m in SJIS_RE.finditer(chunk):
            if any(a < m.end() and m.start() < b for a, b in taken):
                continue
            start = lo + m.start()
            if conservative_text and start < img.code_hi:
                continue
            blob = m.group(0)
            try:
                text = blob[:-1].decode("cp932")
            except UnicodeDecodeError:
                continue
            if all(ord(c) < 0x80 for c in text) or chunk[m.start() - 1 : m.start()] not in (b"", b"\x00"):
                continue
            if create_byte_array(prog, start, len(blob)):
                stats["string_bytes"] += len(blob)
                stats["sjis_strings"] += 1
                mark_auto(prog, start, "shift_jis_string", len(blob))
                set_eol(prog, start, "[Pass2] cp932: " + text[:60])


def _type_pointer_runs(prog, img: Image, ranges, stats: dict, seeds: set) -> None:
    from ghidra.program.model.data import ArrayDataType, Pointer32DataType

    listing = prog.getListing()
    ptr_dt = Pointer32DataType()

    def flush(run) -> None:
        if len(run) < MIN_PTR_RUN or len({t for _, t in run}) < 2:
            return
        start = run[0][0]
        try:
            listing.createData(va_addr(prog, start), ArrayDataType(ptr_dt, len(run), 4))
        except Exception:
            return
        stats["pointer_table_bytes"] += 4 * len(run)
        stats["pointer_tables"] += 1
        mark_auto(prog, start, "pointer_table", 4 * len(run))
        seeds.update(t for _, t in run)

    for lo, hi in ranges:
        run = []
        va = lo + ((4 - (lo - img.base) % 4) % 4)
        while va + 3 <= hi:
            target = img.u32(va)
            if img.contains(target) and target != va:
                run.append((va, target))
            else:
                flush(run)
                run = []
            va += 4
        flush(run)


def _type_padding(prog, img: Image, ranges, stats: dict, min_run: int = MIN_PAD_RUN) -> None:
    for lo, hi in ranges:
        chunk = img.slice(lo, hi + 1)
        for pad in PAD_BYTES:
            pat = re.compile(re.escape(bytes([pad])) + b"{%d,}" % min_run)
            for m in pat.finditer(chunk):
                start = lo + m.start()
                length = m.end() - m.start()
                if create_byte_array(prog, start, length):
                    stats["padding_bytes"] += length
                    stats["padding_runs"] += 1
                    mark_auto(prog, start, "padding_0x%02X" % pad, length)
    for pad in PAD_BYTES:
        n = 0
        while n < img.size and img.raw[img.size - 1 - n] == pad:
            n += 1
        if n >= 8:
            _type_undefined_bytes(prog, img.end - n, img.end - 1, "tail_padding_0x%02X" % pad, "padding_bytes", stats)


def pass2_program(prog, img: Image) -> dict:
    stats = {
        "structured_table_bytes": 0,
        "string_bytes": 0,
        "strings": 0,
        "sjis_strings": 0,
        "pointer_table_bytes": 0,
        "pointer_tables": 0,
        "padding_bytes": 0,
        "padding_runs": 0,
    }
    seeds: set = set()
    with tx(prog, "Panadiga pass2 typed data"):
        _type_cli_table(prog, img, stats, seeds)
        _type_atapi_tables(prog, img, stats, seeds)
        _type_mecha_table(prog, img, stats)
        _type_bgo_and_jump_tables(prog, img, stats, seeds)
        _type_sboxes(prog, img, stats)
        _type_strings(prog, img, undefined_ranges(prog), stats, conservative_text=True)
        _type_pointer_runs(prog, img, undefined_ranges(prog), stats, seeds)
        _type_padding(prog, img, undefined_ranges(prog), stats)
    stats["pointer_seeds"] = len(seeds)
    return {"stats": stats, "seeds": sorted(seeds)}

# ---------------------------------------------------------------------------
# Pass 3 - clean validation, entry/orphan seeding, bounded gap sweep, Trap #4
# ---------------------------------------------------------------------------

ERR_FATAL = "fatal"
ERR_TRAP4 = "trap4"
ERR_EXTERNAL = "external"
ERR_NOP_RUN = "nop-run"


def _error_comment(prog, addr):
    for bm in prog.getBookmarkManager().getBookmarks(addr):
        if bm.getTypeString() == ERROR_BM:
            return str(bm.getComment())
    return None


def _classify_error(prog, addr, comment: str, is_fallthrough: bool) -> str:
    text = (comment or "").lower()
    if "maximum run of repeated byte" in text:
        if addr is not None and prog.getMemory().contains(addr):
            try:
                if (prog.getMemory().getByte(addr) & 0xFF) == 0xCB:
                    return ERR_NOP_RUN
            except Exception:
                pass
        return ERR_FATAL
    if "conflicting data" in text:
        return ERR_FATAL
    if "conflicting instruction" in text:
        return ERR_FATAL if is_fallthrough else ERR_TRAP4
    m = NONEXIST_ADDR_RE.search(text)
    if m is not None:
        ext_addr = va_addr(prog, int(m.group(1), 16))
        return ERR_EXTERNAL if prog.getMemory().contains(ext_addr) else ERR_FATAL
    if "uninitialized" in text or "non-existing memory" in text:
        return ERR_EXTERNAL
    return ERR_FATAL


def _drop_errors(prog, addrs) -> None:
    from ghidra.program.model.address import AddressSet
    from ghidra.util.task import TaskMonitor

    bm_mgr = prog.getBookmarkManager()
    for a in addrs:
        bm_mgr.removeBookmarks(AddressSet(a, a), ERROR_BM, TaskMonitor.DUMMY)


def error_snapshot(prog) -> set:
    out = set()
    it = prog.getBookmarkManager().getBookmarksIterator(ERROR_BM)
    while it.hasNext():
        out.add(to_u32(it.next().getAddress().getOffset()))
    return out


def note_trap4(prog, src_va: int, tgt) -> None:
    from ghidra.program.model.listing import CodeUnit

    listing = prog.getListing()
    cur = listing.getInstructionContaining(tgt)
    if cur is None or cur.getAddress().equals(tgt):
        return
    msg = "Trap4 mid-insn alias: branch 0x%08X lands +%d inside insn 0x%08X (compiler density optimization, not missing code)" % (
        src_va,
        int(tgt.subtract(cur.getAddress())),
        to_u32(cur.getAddress().getOffset()),
    )
    prog.getBookmarkManager().setBookmark(tgt, AUTO_BM_TYPE, TRAP4_CAT, msg)
    if listing.getComment(CodeUnit.EOL_COMMENT, cur.getAddress()) is None:
        listing.setComment(cur.getAddress(), CodeUnit.EOL_COMMENT, "[Trap4] " + msg)


def _resume_nop_run(prog, addr, pre: set) -> bool:
    """Drop repeated-0xCB Error bookmark and resume disassembly past the NOP delay sled."""
    from ghidra.app.cmd.disassemble import DisassembleCommand

    listing = prog.getListing()
    mem = prog.getMemory()
    cur = addr
    progressed = False
    for _ in range(8):
        if cur is None or not mem.contains(cur):
            break
        _drop_errors(prog, [cur])
        pre.discard(to_u32(cur.getOffset()))
        if listing.getInstructionContaining(cur) is not None:
            break
        cmd = DisassembleCommand(cur, None, True)
        cmd.applyTo(prog)
        dset = cmd.getDisassembledAddressSet()
        if dset is not None and not dset.isEmpty():
            progressed = True
        nxt = None
        for e in sorted(error_snapshot(prog) - pre):
            ea = va_addr(prog, e)
            if _classify_error(prog, ea, _error_comment(prog, ea), False) == ERR_NOP_RUN:
                nxt = ea
                break
        if nxt is None:
            break
        cur = nxt
    return progressed


def _disassemble_validated_ex(prog, va: int, min_insns: int = 2, need_terminal: bool = True, create_function: bool = False, pre_errors=None) -> tuple:
    """Flow-disassemble at ``va`` and return ``(AddressSet | None, reason)`` after validating every flow."""
    from ghidra.app.cmd.disassemble import DisassembleCommand
    from ghidra.app.cmd.function import CreateFunctionCmd
    from ghidra.program.model.symbol import SourceType

    listing = prog.getListing()
    addr = va_addr(prog, va)
    if not prog.getMemory().contains(addr):
        return None, "unmapped"
    if listing.getInstructionAt(addr) is not None:
        return None, "already"
    if listing.getInstructionContaining(addr) is not None:
        return None, "inside-insn"
    existing = listing.getDataContaining(addr)
    if existing is not None and existing.isDefined():
        return None, "data"
    if pre_errors is None:
        pre_errors = error_snapshot(prog)
    stale = _error_comment(prog, addr)
    if stale is not None:
        _drop_errors(prog, [addr])
    cmd = DisassembleCommand(addr, None, True)
    ok = cmd.applyTo(prog)
    dset = cmd.getDisassembledAddressSet()
    if not ok or dset is None or dset.isEmpty():
        _drop_errors(prog, [va_addr(prog, e) for e in (error_snapshot(prog) - pre_errors)])
        if stale is not None:
            prog.getBookmarkManager().setBookmark(addr, ERROR_BM, "Bad Instruction", stale[:100])
            pre_errors.add(va)
        return None, "decode-fail"

    for e in sorted(error_snapshot(prog) - pre_errors):
        ea = va_addr(prog, e)
        if _classify_error(prog, ea, _error_comment(prog, ea), False) == ERR_NOP_RUN:
            _resume_nop_run(prog, ea, pre_errors)
            if listing.getInstructionAt(ea) is not None:
                dset.add(ea)

    count = 0
    terminal = False
    fatal = False
    trap4 = []
    external = []
    it = listing.getInstructions(dset, True)
    while it.hasNext():
        ins = it.next()
        count += 1
        if not ins.hasFallthrough():
            terminal = True
        checks = [(t, False) for t in (ins.getFlows() or [])]
        fall = ins.getFallThrough()
        if fall is not None:
            checks.append((fall, True))
        for tgt, is_fall in checks:
            if listing.getInstructionAt(tgt) is not None:
                continue
            comment = _error_comment(prog, tgt)
            if comment is None:
                continue
            kind = _classify_error(prog, tgt, comment, is_fall)
            if kind == ERR_TRAP4:
                trap4.append((to_u32(ins.getAddress().getOffset()), tgt))
            elif kind == ERR_EXTERNAL:
                external.append(tgt)
            elif kind != ERR_NOP_RUN:
                fatal = True
        ins_err = _error_comment(prog, ins.getAddress())
        if ins_err is not None:
            kind = _classify_error(prog, ins.getAddress(), ins_err, False)
            if kind == ERR_EXTERNAL:
                external.append(ins.getAddress())
            elif kind not in (ERR_TRAP4, ERR_NOP_RUN):
                fatal = True

    if fatal or count < min_insns or (need_terminal and not terminal):
        for rng in dset.getAddressRanges():
            listing.clearCodeUnits(rng.getMinAddress(), rng.getMaxAddress(), False)
        _drop_errors(prog, [va_addr(prog, e) for e in (error_snapshot(prog) - pre_errors)])
        if stale is not None:
            prog.getBookmarkManager().setBookmark(addr, ERROR_BM, "Bad Instruction", stale[:100])
            pre_errors.add(va)
        return None, ("bad-decode" if fatal else ("short" if count < min_insns else "no-terminal"))

    for src_va, tgt in trap4:
        note_trap4(prog, src_va, tgt)
    for tgt in external:
        prog.getBookmarkManager().setBookmark(tgt, AUTO_BM_TYPE, EXTERNAL_CAT, "flow into uninitialized mapped memory")
    dropped = [t for _s, t in trap4] + external
    _drop_errors(prog, dropped)
    for a in dropped:
        pre_errors.discard(to_u32(a.getOffset()))
    pre_errors.update(error_snapshot(prog))
    if create_function and prog.getFunctionManager().getFunctionAt(addr) is None:
        CreateFunctionCmd(None, addr, None, SourceType.ANALYSIS).applyTo(prog)
    return dset, "ok"


def disassemble_validated(prog, va: int, min_insns: int = 2, need_terminal: bool = True, create_function: bool = False, pre_errors=None) -> tuple:
    dset, reason = _disassemble_validated_ex(prog, va, min_insns, need_terminal, create_function, pre_errors)
    return (int(dset.getNumAddresses()), reason) if dset is not None else (0, reason)


def redisassemble_bad_sites(prog, prune_seeds=None) -> dict:
    bm_mgr = prog.getBookmarkManager()
    listing = prog.getListing()
    fm = prog.getFunctionManager()
    rm = prog.getReferenceManager()
    sites = [(bm.getAddress(), str(bm.getComment())) for bm in _iter_bookmarks(bm_mgr, ERROR_BM)]
    stats = {"bad_sites_before": len(sites), "redecoded": 0, "trap4_converted": 0, "external_converted": 0, "nop_sleds": 0, "pruned": 0, "remaining": 0}
    pre = error_snapshot(prog)
    for addr, comment in sites:
        if _error_comment(prog, addr) is None:
            continue
        va = to_u32(addr.getOffset())
        kind = _classify_error(prog, addr, comment, False)
        if kind == ERR_TRAP4 or (listing.getInstructionContaining(addr) is not None and listing.getInstructionAt(addr) is None):
            note_trap4(prog, _infer_source_va(prog, addr, comment), addr)
            _drop_errors(prog, [addr])
            pre.discard(va)
            stats["trap4_converted"] += 1
        elif kind == ERR_EXTERNAL:
            bm_mgr.setBookmark(addr, AUTO_BM_TYPE, EXTERNAL_CAT, comment[:120])
            _drop_errors(prog, [addr])
            pre.discard(va)
            stats["external_converted"] += 1
        elif kind == ERR_NOP_RUN:
            if _resume_nop_run(prog, addr, pre):
                stats["nop_sleds"] += 1
            else:
                _drop_errors(prog, [addr])
                pre.discard(va)
        elif listing.getInstructionAt(addr) is not None and prune_seeds is None:
            _drop_errors(prog, [addr])
            pre.discard(va)
            stats["redecoded"] += 1
        else:
            added, _reason = (0, "prune") if prune_seeds is not None else disassemble_validated(prog, va, min_insns=1, need_terminal=False, pre_errors=pre)
            if added:
                stats["redecoded"] += 1
            elif prune_seeds is not None:
                fn = fm.getFunctionContaining(addr)
                if fn is not None and to_u32(fn.getEntryPoint().getOffset()) in prune_seeds and rm.getReferenceCountTo(fn.getEntryPoint()) == 0:
                    body = fn.getBody()
                    fm.removeFunction(fn.getEntryPoint())
                    for rng in body.getAddressRanges():
                        listing.clearCodeUnits(rng.getMinAddress(), rng.getMaxAddress(), False)
                    stats["pruned"] += 1
                _drop_errors(prog, [addr])
                pre.discard(va)
            else:
                stats["remaining"] += 1
    return stats

def _iter_bookmarks(bm_mgr, kind: str):
    it = bm_mgr.getBookmarksIterator(kind)
    out = []
    while it.hasNext():
        out.append(it.next())
    return out


def _infer_source_va(prog, addr, comment: str) -> int:
    listing = prog.getListing()
    for ref in prog.getReferenceManager().getReferencesTo(addr):
        ins = listing.getInstructionAt(ref.getFromAddress())
        if ins is not None:
            return to_u32(ins.getAddress().getOffset())
    m = FLOW_FROM_RE.search(comment or "")
    return int(m.group(1), 16) if m is not None else 0


def _ref_and_scalar_targets(prog, img: Image, undef_ranges, include_scalars: bool) -> set:
    from ghidra.program.model.address import AddressSet
    from ghidra.program.model.scalar import Scalar

    if not undef_ranges:
        return set()
    undef = AddressSet()
    for lo, hi in undef_ranges:
        if lo < img.code_hi:
            undef.add(va_addr(prog, lo), va_addr(prog, min(hi, img.code_hi - 1)))
    if undef.isEmpty():
        return set()
    out = set()
    it = prog.getReferenceManager().getReferenceDestinationIterator(undef, True)
    while it.hasNext():
        out.add(to_u32(it.next().getOffset()))
    if include_scalars:
        ins_it = prog.getListing().getInstructions(True)
        while ins_it.hasNext():
            ins = ins_it.next()
            if ins.getLength() < 5:
                continue
            for idx in range(ins.getNumOperands()):
                for obj in ins.getOpObjects(idx):
                    if isinstance(obj, Scalar):
                        val = to_u32(obj.getUnsignedValue())
                        if img.contains(val) and val < img.code_hi and undef.contains(va_addr(prog, val)):
                            out.add(val)
    return out


def _load_rust_audit(name: str) -> dict:
    for base_dir in (WORK_DIR, REPORT_DIR):
        path = base_dir / ("rust_audit_%s.json" % name)
        if path.exists() and path.stat().st_size > 0:
            return json.loads(path.read_text(encoding="utf-8"))
    return {}


def infer_generic_entry_seeds(img: Image) -> tuple:
    """Infer MN10300 reset-vector branches, header pointers, and early prologues for arbitrary binaries."""
    if img.raw[:4] == b"\x00\xf2\xf3\xfe":
        seeds, seen = [img.base + 0x11], {img.base + 0x11}
        probe_offs = (0x11, 0x19, 0x22, 0x2B, 0x34)
    elif img.raw[:8] == b"\n root=/":
        seeds, seen = [img.base + 0x400, img.base + 0x534], {img.base + 0x400, img.base + 0x534}
        probe_offs = (0x400, 0x410, 0x534, 0x540, 0x1400)
    else:
        seeds, seen = [img.base], {img.base}
        probe_offs = GENERIC_STAGE_RULES.get("header_probe_offsets", (0, 4, 8, 12, 16, 20, 24, 28, 32, 48, 64))
    for off in probe_offs:
        va = img.base + int(off)
        if not img.contains(va + 4):
            continue
        op = img.raw[off]
        tgt = None
        if op in (0xDC, 0xDD):
            disp = struct.unpack_from("<i", img.raw, off + 1)[0]
            tgt = to_u32(va + disp)
        elif op == 0xCC:
            disp = struct.unpack_from("<h", img.raw, off + 1)[0]
            tgt = to_u32(va + disp)
        elif op == 0xCA:
            disp = struct.unpack_from("<b", img.raw, off + 1)[0]
            tgt = to_u32(va + disp)
        for cand in (va if tgt is not None else None, tgt, img.u32(va)):
            if cand is not None and img.contains(cand) and cand not in seen and img.raw[cand - img.base] not in PAD_BYTES:
                seen.add(cand)
                seeds.append(cand)
    m = STRONG_PROLOGUE_RE.search(img.raw[: min(img.size, 0x2000)])
    if m is not None:
        va = img.base + m.start()
        if va not in seen:
            seeds.append(va)
    return tuple(seeds)


def _seed_entries(prog, img: Image, name: str, pre: set) -> dict:
    stats = {"candidates": 0, "accepted": 0, "bytes_added": 0}
    candidates = STAGE_ENTRY_SEEDS.get(name) or infer_generic_entry_seeds(img)
    for va in candidates:
        if not img.contains(va):
            continue
        stats["candidates"] += 1
        added, _reason = disassemble_validated(prog, va, min_insns=1, need_terminal=False, create_function=True, pre_errors=pre)
        if added:
            stats["accepted"] += 1
            stats["bytes_added"] += added
    return stats


def _batch_disassemble_and_prune(prog, seed_vas: list, chunk_size: int = 4000) -> dict:
    """Batch-disassemble seed VAs via Java AddressSet, create functions, and clean bad islands."""
    from ghidra.app.cmd.disassemble import DisassembleCommand
    from ghidra.app.cmd.function import CreateFunctionCmd
    from ghidra.program.model.address import AddressSet
    from ghidra.program.model.symbol import SourceType

    listing = prog.getListing()
    fm = prog.getFunctionManager()
    todo = [va for va in seed_vas if listing.getInstructionContaining(va_addr(prog, va)) is None and (listing.getDataContaining(va_addr(prog, va)) is None or not listing.getDataContaining(va_addr(prog, va)).isDefined())]
    bytes_added = 0
    for i in range(0, len(todo), chunk_size):
        chunk = todo[i : i + chunk_size]
        aset = AddressSet()
        for va in chunk:
            a = va_addr(prog, va)
            if listing.getInstructionContaining(a) is None:
                aset.add(a)
        if aset.isEmpty():
            continue
        cmd = DisassembleCommand(aset, None, True)
        cmd.applyTo(prog)
        dset = cmd.getDisassembledAddressSet()
        if dset is not None:
            bytes_added += int(dset.getNumAddresses())
        for va in chunk:
            a = va_addr(prog, va)
            if listing.getInstructionAt(a) is not None and fm.getFunctionAt(a) is None:
                CreateFunctionCmd(None, a, None, SourceType.ANALYSIS).applyTo(prog)
    cleanup = redisassemble_bad_sites(prog, prune_seeds=set(todo))
    accepted = sum(1 for va in todo if listing.getInstructionAt(va_addr(prog, va)) is not None)
    return {"candidates": len(todo), "accepted": accepted, "bytes_added": bytes_added, "cleanup": cleanup}


def _seed_strong_prologues_in_gaps(prog, img: Image, _pre: set) -> dict:
    matches = []
    for lo, hi in undefined_ranges(prog):
        if lo >= img.code_hi:
            continue
        cap = min(hi, img.code_hi - 1)
        if cap - lo + 1 < 6:
            continue
        for m in STRONG_PROLOGUE_RE.finditer(img.slice(lo, cap + 1)):
            matches.append(lo + m.start())
    return _batch_disassemble_and_prune(prog, matches)

def _collect_pointer_seeds(prog, img: Image, name: str, pass2_seeds: list) -> dict:
    import ghidra_mn103_fix_decompilation as fix

    seeds = {}

    def add(va: int, why: str) -> None:
        if img.contains(va) and va < img.code_hi:
            seeds.setdefault(va, why)

    for _site, target in _load_rust_audit(name).get("orphan_code_pointers", []):
        add(target, "rust_orphan")
    for va in pass2_seeds:
        add(va, "table_pointer")
    for _cdb, _label, va, _note in fix.KNOWN_DRV_ATAPI_SLOTS:
        add(va, "known_atapi_slot")
    for tbl_va, tbl_cnt, _lbl in fix.KNOWN_ORPHAN_POINTER_TABLES:
        for idx in range(tbl_cnt):
            if img.contains(tbl_va + 4 * idx + 3):
                add(img.u32(tbl_va + 4 * idx), "known_pointer_table")
    for va in _ref_and_scalar_targets(prog, img, undefined_ranges(prog), include_scalars=(img.size <= LARGE_IMAGE_BYTES)):
        add(va, "code_ref_or_imm32")
    return seeds


def _seed_dict(prog, img: Image, seeds: dict, pre: set) -> dict:
    listing = prog.getListing()
    pending = {va: src for va, src in seeds.items() if listing.getInstructionContaining(va_addr(prog, va)) is None}
    already = len(seeds) - len(pending)
    if img.size > LARGE_IMAGE_BYTES and len(pending) > 2000:
        plausible = [va for va in sorted(pending) if img.raw[va - img.base] not in PAD_BYTES]
        res = _batch_disassemble_and_prune(prog, plausible, chunk_size=2000)
        res["skipped_already"] = already
        return res
    result = {"candidates": len(seeds), "skipped_already": already, "accepted": 0, "rejected": {}, "bytes_added": 0, "by_source": {}}
    for va in sorted(pending):
        added, reason = disassemble_validated(prog, va, min_insns=3, need_terminal=True, create_function=True, pre_errors=pre)
        if added:
            result["accepted"] += 1
            result["bytes_added"] += added
            src = pending[va]
            result["by_source"][src] = result["by_source"].get(src, 0) + 1
        else:
            result["rejected"][reason] = result["rejected"].get(reason, 0) + 1
    return result


def _pad_prefix(img: Image, va: int, hi: int, limit: int = 3) -> int:
    n = 0
    while n < limit and va + n <= hi and img.raw[va + n - img.base] in (0x00, 0xFF, 0xCB):
        n += 1
    return n


def _island_tiles(prog, img: Image, end: int, hi: int) -> bool:
    nxt = end + 1
    if nxt > hi:
        return True
    nxt += _pad_prefix(img, nxt, hi)
    if nxt > hi:
        return True
    cu = prog.getListing().getCodeUnitAt(va_addr(prog, nxt))
    if cu is not None and (prog.getListing().getInstructionAt(va_addr(prog, nxt)) is not None or cu.isDefined()):
        return True
    return STRONG_PROLOGUE_RE.match(img.raw, nxt - img.base) is not None


def _try_weak_island(prog, img: Image, va: int, hi: int, pre: set) -> int:
    from ghidra.app.cmd.function import CreateFunctionCmd
    from ghidra.program.model.symbol import SourceType

    listing = prog.getListing()
    dset, _reason = _disassemble_validated_ex(prog, va, min_insns=3, need_terminal=True, create_function=False, pre_errors=pre)
    if dset is None:
        return 0
    end = to_u32(dset.getMaxAddress().getOffset())
    if not _island_tiles(prog, img, end, hi):
        for rng in dset.getAddressRanges():
            listing.clearCodeUnits(rng.getMinAddress(), rng.getMaxAddress(), False)
        return 0
    entry = va_addr(prog, va)
    if prog.getFunctionManager().getFunctionAt(entry) is None:
        CreateFunctionCmd(None, entry, None, SourceType.ANALYSIS).applyTo(prog)
    return int(dset.getNumAddresses())


def sweep_gaps(prog, img: Image, pre: set, min_gap: int = 6) -> dict:
    """Bounded sweep of undefined code-window gap heads right after terminal instructions."""
    max_rounds = 4 if img.size > LARGE_IMAGE_BYTES else 10
    stats = {"islands": 0, "island_bytes": 0, "rounds": 0}
    listing = prog.getListing()
    for _ in range(max_rounds):
        stats["rounds"] += 1
        progress = 0
        for lo, hi in undefined_ranges(prog):
            if lo >= img.code_hi:
                continue
            cap = min(hi, img.code_hi - 1)
            if cap - lo + 1 < min_gap:
                continue
            prev = listing.getInstructionContaining(va_addr(prog, lo - 1)) if lo > img.base else None
            if prev is None or prev.hasFallthrough():
                continue
            for off in range(_pad_prefix(img, lo, cap) + 1):
                va = lo + off
                if listing.getInstructionContaining(va_addr(prog, va)) is not None:
                    continue
                added = _try_weak_island(prog, img, va, cap, pre)
                if added:
                    progress += 1
                    stats["islands"] += 1
                    stats["island_bytes"] += added
                    break
        if progress == 0:
            break
    return stats


def annotate_trap4(prog, img: Image, name: str) -> dict:
    listing = prog.getListing()
    seen = set()
    it = listing.getInstructions(True)
    while it.hasNext():
        ins = it.next()
        src_va = to_u32(ins.getAddress().getOffset())
        for tgt in ins.getFlows() or []:
            cur = listing.getInstructionContaining(tgt)
            if cur is not None and not cur.getAddress().equals(tgt):
                tva = to_u32(tgt.getOffset())
                if tva not in seen:
                    note_trap4(prog, src_va, tgt)
                    seen.add(tva)
    for row in _load_rust_audit(name).get("mid_instruction_overlaps", []):
        src_va = row["source_va"]
        tva = row["target_va"]
        if tva in seen or listing.getInstructionAt(va_addr(prog, src_va)) is None:
            continue
        tgt = va_addr(prog, tva)
        cur = listing.getInstructionContaining(tgt)
        if cur is not None and not cur.getAddress().equals(tgt):
            note_trap4(prog, src_va, tgt)
            seen.add(tva)
    return {"trap4_aliases_bookmarked": len(seen)}


def pass3_program(prog, img: Image, name: str, pass2_seeds: list) -> dict:
    import ghidra_am34_memory_map as memmap

    report = {}
    with tx(prog, "Panadiga pass3 island seeding"):
        memmap.setup_memory_map(prog)
        report["volatile"] = ensure_volatile_windows(prog)
        report["redisassemble_before"] = redisassemble_bad_sites(prog)
        pre = error_snapshot(prog)
        report["entry_seeds"] = _seed_entries(prog, img, name, pre)
        report["strong_prologues"] = _seed_strong_prologues_in_gaps(prog, img, pre)
        report["pointer_seeds"] = _seed_dict(prog, img, _collect_pointer_seeds(prog, img, name, pass2_seeds), pre)
        report["sweep"] = sweep_gaps(prog, img, pre)
        report["redisassemble_after"] = redisassemble_bad_sites(prog)
        report["trap4"] = annotate_trap4(prog, img, name)
    return report

# ---------------------------------------------------------------------------
# Pass 4 - normalization, residual typing, verification & generic CLI
# ---------------------------------------------------------------------------

def ensure_volatile_windows(prog) -> dict:
    mem = prog.getMemory()
    space = prog.getAddressFactory().getDefaultAddressSpace()
    created = 0
    for name, lo, hi in MMIO_WINDOWS:
        if _create_uninit_block_if_free(mem, space, name, lo, hi - lo + 1, True, True, False, True):
            created += 1
    marked = 0
    for blk in mem.getBlocks():
        lo, hi = to_u32(blk.getStart().getOffset()), to_u32(blk.getEnd().getOffset())
        if not blk.isInitialized() and any(lo <= whi and wlo <= hi for _n, wlo, whi in MMIO_WINDOWS):
            if not blk.isVolatile():
                blk.setVolatile(True)
                marked += 1
    return {"blocks_created": created, "blocks_marked_volatile": marked}


def type_residual(prog, img: Image) -> dict:
    from ghidra.program.model.address import AddressSet
    from ghidra.util.task import TaskMonitor

    bm_mgr = prog.getBookmarkManager()
    err_vas = error_snapshot(prog)
    typed_ranges = 0
    pad_bytes = 0
    raw_bytes = 0
    cleared_errors = 0
    for lo, hi in undefined_ranges(prog):
        length = hi - lo + 1
        if length <= MAX_TINY_PAD and len(set(img.slice(lo, hi + 1))) == 1:
            kind, cat = "alignment_pad_0x%02X" % img.raw[lo - img.base], AUTO_DATA_CAT
            pad_bytes += length
        else:
            kind, cat = "unclassified_raw", RESIDUAL_CAT
            raw_bytes += length
        if create_byte_array(prog, lo, length):
            typed_ranges += 1
            mark_auto(prog, lo, kind, length, cat)
            if err_vas:
                hits = [va for va in err_vas if lo <= va <= hi]
                if hits:
                    bm_mgr.removeBookmarks(AddressSet(va_addr(prog, lo), va_addr(prog, hi)), ERROR_BM, TaskMonitor.DUMMY)
                    cleared_errors += len(hits)
                    err_vas.difference_update(hits)
    return {
        "residual_ranges": typed_ranges,
        "tiny_pad_bytes": pad_bytes,
        "residual_unclassified_bytes": raw_bytes,
        "stale_error_bookmarks_cleared": cleared_errors,
    }


def decompile_tainted(prog) -> dict:
    from ghidra.app.decompiler import DecompInterface
    from ghidra.util.task import ConsoleTaskMonitor

    results = {}
    di = DecompInterface()
    di.openProgram(prog)
    try:
        for va in TAINTED_FUNCTIONS:
            fn = prog.getFunctionManager().getFunctionContaining(va_addr(prog, va))
            if fn is None:
                continue
            res = di.decompileFunction(fn, 120, ConsoleTaskMonitor())
            text = res.getDecompiledFunction().getC() if res.decompileCompleted() else ""
            results["0x%08X" % va] = {
                "function": fn.getName(),
                "completed": bool(res.decompileCompleted()),
                "bad_tokens": [t for t in DECOMP_BAD_TOKENS if t in text],
                "lines": text.count("\n"),
            }
    finally:
        di.dispose()
    return results


def spot_check_opcodes(prog, name: str) -> dict:
    listing = prog.getListing()
    out = {}
    for va in SPOT_CHECK_OPCODES.get(name, ()):
        ins = listing.getInstructionContaining(va_addr(prog, va))
        if ins is None:
            out["0x%08X" % va] = None
        else:
            ops = ",".join(ins.getDefaultOperandRepresentation(i) for i in range(ins.getNumOperands()))
            out["0x%08X" % va] = "%s @ 0x%08X: %s %s" % (ins.getMnemonicString(), to_u32(ins.getAddress().getOffset()), ins.getMnemonicString(), ops)
    return out


def pass4_program(prog, img: Image, name: str, pass2_seeds: list, switches: bool) -> dict:
    import ghidra_auto_unbrick_audit as audit
    import ghidra_mn103_fix_decompilation as fix
    from ghidra.util.task import TaskMonitor

    WORK_DIR.mkdir(parents=True, exist_ok=True)
    report = {}
    is_large = img.size > LARGE_IMAGE_BYTES
    with tx(prog, "Panadiga pass4 decompilation normalization"):
        fix_res = fix.run_mn103_decompilation_fixes(prog, apply_purge=True, do_switches=(switches and not is_large), resolve_traps=True)
        if switches and is_large:
            targets = fix._find_computed_jump_functions(prog)[:60]
            fix_res["switches"] = fix.recover_mn103_switches(prog, targets=targets, max_passes=1, timeout_secs=15, monitor_obj=TaskMonitor.DUMMY)
            fix_res["switches"]["bounded_large_image"] = True
        report["fix_decompilation"] = fix_res
        report["volatile"] = ensure_volatile_windows(prog)
        report["unbrick_audit"] = {"stage": audit.run_unbrick_audit(prog, str(WORK_DIR / ("ghidra_audit_%s.json" % name))).get("detected_stage")}
    with tx(prog, "Panadiga pass4 finalization"):
        report["redisassemble"] = redisassemble_bad_sites(prog)
        pre = error_snapshot(prog)
        report["strong_prologues"] = _seed_strong_prologues_in_gaps(prog, img, pre)
        report["seeding"] = _seed_dict(prog, img, _collect_pointer_seeds(prog, img, name, pass2_seeds), pre)
        report["sweep"] = sweep_gaps(prog, img, pre)
        report["redisassemble_final"] = redisassemble_bad_sites(prog)
        report["trap4"] = annotate_trap4(prog, img, name)
        late = {
            "structured_table_bytes": 0, "string_bytes": 0, "strings": 0, "sjis_strings": 0,
            "pointer_table_bytes": 0, "pointer_tables": 0, "padding_bytes": 0, "padding_runs": 0,
        }
        _type_strings(prog, img, undefined_ranges(prog), late)
        _type_padding(prog, img, undefined_ranges(prog), late)
        report["late_data"] = late
        report["residual"] = type_residual(prog, img)
    report["decompile_tainted"] = decompile_tainted(prog)
    report["spot_checks"] = spot_check_opcodes(prog, name)
    return report


def bookmark_bytes(prog, category: str) -> dict:
    totals = {}
    for bm in _iter_bookmarks(prog.getBookmarkManager(), AUTO_BM_TYPE):
        if bm.getCategory() != category:
            continue
        kind, _, tail = str(bm.getComment()).partition(" len=")
        entry = totals.setdefault(kind.split("_0x")[0], [0, 0])
        entry[0] += 1
        entry[1] += int(tail) if tail.isdigit() else 0
    return {k: {"items": v[0], "bytes": v[1]} for k, v in sorted(totals.items())}


def verify_program(prog, name: str) -> dict:
    rep = measure(prog)
    rep["instruction_bytes"] = instruction_bytes(prog)
    rep["auto_data"] = bookmark_bytes(prog, AUTO_DATA_CAT)
    rep["residual_unclassified"] = bookmark_bytes(prog, RESIDUAL_CAT)
    bm_mgr = prog.getBookmarkManager()
    rep["trap4_aliases"] = sum(1 for bm in _iter_bookmarks(bm_mgr, AUTO_BM_TYPE) if bm.getCategory() == TRAP4_CAT)
    rep["external_flows"] = sum(1 for bm in _iter_bookmarks(bm_mgr, AUTO_BM_TYPE) if bm.getCategory() == EXTERNAL_CAT)
    rep["decompile_tainted"] = decompile_tainted(prog)
    rep["spot_checks"] = spot_check_opcodes(prog, name)
    rep["gate_undefined_code_bytes_zero"] = rep["undefined_code_bytes"] == 0
    rep["gate_bad_bookmarks_zero"] = rep["bad_instruction_bookmarks"] == 0
    rep["gate_mn_todo_in_functions_zero"] = rep["mn_todo_in_functions"] == 0
    rep["gate_tainted_clean"] = all(not v["bad_tokens"] and v["completed"] for v in rep["decompile_tainted"].values())
    return rep


def load_seed_file(name: str) -> list:
    for base_dir in (WORK_DIR, REPORT_DIR):
        path = base_dir / ("pass2_seeds_%s.json" % name)
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
    return []


def _auto_unpack_if_container(bin_path: pathlib.Path, opts):
    total_len = bin_path.stat().st_size
    with bin_path.open("rb") as fh:
        raw_head = fh.read(0x2000)
    _fmt, _kname, rk, entries = parse_frm_toc(raw_head, total_len=total_len)
    is_container = opts.frm is not None or bool(entries) or rk is not None or raw_head[:8] == b"EXTRHEAD" or raw_head[:12] == b"PANASONIC\x00\x00\x00"
    if not is_container:
        return None
    out_dir = WORK_DIR / "decrypted_stages"
    manifest, targets = decrypt_and_prepare_runtime_stages(bin_path, out_dir, opts.model, opts.region)
    return manifest, targets


def _resolve_targets(opts) -> list:
    if opts.frm is not None:
        frm_path = pathlib.Path(opts.frm).resolve()
        if not frm_path.is_file():
            raise SystemExit("frm container not found: %s" % frm_path)
        _manifest, targets = decrypt_and_prepare_runtime_stages(
            frm_path, WORK_DIR / "decrypted_stages", opts.model, opts.region
        )
        return targets
    if opts.binary is not None:
        bin_path = pathlib.Path(opts.binary).resolve()
        if not bin_path.is_file():
            raise SystemExit("binary not found: %s" % bin_path)
        unpacked = _auto_unpack_if_container(bin_path, opts)
        if unpacked is not None:
            return unpacked[1]
        raw_prog = opts.program_name or ("/" + bin_path.name)
        prog_path = raw_prog if raw_prog.startswith("/") else "/" + raw_prog
        name = opts.only or prog_path.rsplit("/", 1)[-1]
        base_va = infer_base_va(bin_path, opts.model, opts.base)
        return [(name, prog_path, bin_path, base_va)]
    names = [opts.only] if opts.only else list(STAGES)
    targets = []
    for name in names:
        if name not in STAGES:
            raise SystemExit("unknown stage program '%s' (known: %s; or pass --binary)" % (name, ", ".join(STAGES)))
        proj_path, bin_path = STAGES[name]
        base_va = to_u32(int(opts.base, 0)) if opts.base is not None else STAGE_BASES[name]
        targets.append((name, proj_path, bin_path, base_va))
    return targets


def _run_all_passes(prog, img: Image, name: str, opts, pyghidra=None) -> dict:
    from ghidra.util.task import TaskMonitor

    rep = {"program": name, "model": opts.model, "base_va": "0x%08X" % img.base, "baseline": measure(prog)}
    if not str(prog.getLanguageID()).lower().startswith("mn10300"):
        if pyghidra is not None:
            pyghidra.analyze(prog, TaskMonitor.DUMMY)
        with tx(prog, "Panadiga non-MN103 residual typing"):
            rep["residual"] = type_residual(prog, img)
        rep["verify"] = verify_program(prog, name)
        if not opts.no_save:
            prog.save("Panadiga generic auto-analyze", TaskMonitor.DUMMY)
        return rep
    p2 = pass2_program(prog, img)
    write_json(WORK_DIR, "pass2_seeds_%s.json" % name, p2["seeds"])
    rep["pass2"] = p2["stats"]
    rep["pass3"] = pass3_program(prog, img, name, p2["seeds"])
    rep["pass4"] = pass4_program(prog, img, name, p2["seeds"], not opts.no_switches)
    rep["verify"] = verify_program(prog, name)
    if not opts.no_save:
        prog.save("Panadiga zero-undef all", TaskMonitor.DUMMY)
    return rep


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("stage", choices=["decrypt-runtime", "import-loader", "baseline", "pass2", "pass3", "pass4", "verify", "all"])
    ap.add_argument("--only", default=None, help="restrict to one program name")
    ap.add_argument("--binary", default=None, help="path to an arbitrary raw firmware binary or container to analyze")
    ap.add_argument("--frm", default=None, help="path to PANA_DVD.FRM / PANA__SD.FRM / DIGADIAG.FRM for full PyGhidra decrypt + analysis")
    ap.add_argument("--region", default=None, help="region code override (e.g. LB, EC, EE, VP) for TV/electrical interpolation")
    ap.add_argument("--base", default=None, help="base virtual address for --binary (e.g. 0x57000000); inferred if omitted")
    ap.add_argument("--program-name", default=None, help="program path inside Ghidra project (default /<binary.name>)")
    ap.add_argument("--model", default=DEFAULT_MODEL, help="SSOT DIGA model ID for memory map projection (default: %s)" % DEFAULT_MODEL)
    ap.add_argument("--project-dir", default=PROJECT_DIR, help="Ghidra project directory")
    ap.add_argument("--project-name", default=PROJECT_NAME, help="cloned Ghidra project name (source project refused)")
    ap.add_argument("--language", default=LANGUAGE_ID, help="Ghidra SLEIGH language ID")
    ap.add_argument("--compiler", default=COMPILER_ID, help="Ghidra compiler spec ID")
    ap.add_argument("--out", default=None, help="report file name or explicit path override")
    ap.add_argument("--no-save", action="store_true", help="dry run: do not save the program")
    ap.add_argument("--no-switches", action="store_true", help="pass4: skip decompiler switch recovery")
    opts = ap.parse_args()
    if opts.stage == "decrypt-runtime":
        src_arg = opts.frm or opts.binary
        if not src_arg:
            raise SystemExit("decrypt-runtime requires --frm <PATH> or --binary <PATH>")
        src_path = pathlib.Path(src_arg).resolve()
        if not src_path.is_file():
            raise SystemExit("input not found: %s" % src_path)
        out_dir = pathlib.Path(opts.out).resolve() if opts.out else (WORK_DIR / "decrypted_stages")
        manifest, _targets = decrypt_and_prepare_runtime_stages(src_path, out_dir, opts.model, opts.region)
        print(json.dumps(manifest, indent=2))
        return 0
    guard_project(opts.project_name)
    pyghidra = start_ghidra()
    from ghidra.util.task import TaskMonitor

    gpr_missing = not (pathlib.Path(opts.project_dir) / ("%s.gpr" % opts.project_name)).exists()
    project = pyghidra.open_project(opts.project_dir, opts.project_name, create=gpr_missing)
    targets = _resolve_targets(opts)

    if opts.stage == "import-loader":
        if opts.binary is not None or opts.frm is not None:
            for _name, prog_path, bin_path, base_va in targets:
                import_binary(pyghidra, project, prog_path, bin_path, base_va, opts.language, opts.compiler)
                apply_model_memory_layout(project, pyghidra, prog_path, base_va, opts.model)
        else:
            import_loader(pyghidra, project)
            layout_loader(project, pyghidra)
        return 0

    reports = []
    for name, proj_path, bin_path, base_va in targets:
        t0 = time.time()
        if opts.stage == "all" and project.getProjectData().getFile(proj_path) is None:
            import_binary(pyghidra, project, proj_path, bin_path, base_va, opts.language, opts.compiler)
            apply_model_memory_layout(project, pyghidra, proj_path, base_va, opts.model)
        with pyghidra.program_context(project, proj_path) as prog:
            if opts.stage == "baseline":
                rep = measure(prog)
            elif opts.stage == "verify":
                rep = verify_program(prog, name)
            else:
                img = Image(prog, bin_path)
                if opts.stage == "all":
                    rep = _run_all_passes(prog, img, name, opts, pyghidra)
                elif opts.stage == "pass2":
                    res = pass2_program(prog, img)
                    write_json(WORK_DIR, "pass2_seeds_%s.json" % name, res["seeds"])
                    rep = {"program": name, "pass2": res["stats"]}
                elif opts.stage == "pass3":
                    rep = {"program": name, "pass3": pass3_program(prog, img, name, load_seed_file(name))}
                else:
                    rep = {"program": name, "pass4": pass4_program(prog, img, name, load_seed_file(name), not opts.no_switches)}
                if opts.stage != "all":
                    rep["after"] = measure(prog, with_todo=False)
                if opts.stage != "all" and not opts.no_save:
                    prog.save("Panadiga zero-undef %s" % opts.stage, TaskMonitor.DUMMY)
        rep["seconds"] = round(time.time() - t0, 1)
        reports.append(rep)
        print(json.dumps(rep, indent=2, default=str))
        sys.stdout.flush()

    dest_dir = REPORT_DIR if opts.stage in ("baseline", "verify") else WORK_DIR
    suffix = ("_" + opts.only) if opts.only else (("_" + targets[0][0]) if opts.binary and targets else "")
    out_name = opts.out or ("%s%s.json" % (opts.stage, suffix))
    out_path = pathlib.Path(out_name)
    if out_path.is_absolute() or len(out_path.parts) > 1:
        dest_dir = out_path.parent
        out_name = out_path.name
    print("report:", write_json(dest_dir, out_name, reports))
    return 0


if __name__ == "__main__":
    sys.exit(main())
