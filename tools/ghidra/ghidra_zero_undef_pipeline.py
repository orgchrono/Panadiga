"""Panadiga 4-pass zero-``??`` Ghidra recovery pipeline (CPython 3 + PyGhidra).

Operates ONLY on the cloned project ``Panadiga_MN103S_ZeroUndef`` and the patched
Ghidra install clone (``C:\\ghidra\\staging\\ghidra_12.1.4_PATCHED``); the original
``Panadiga_MN103S`` project and stock ``GhidraMn10300`` language are never touched.

Usage (from repo root)::

    C:\\ghidra\\.venv\\Scripts\\python.exe tools\\ghidra\\ghidra_zero_undef_pipeline.py <stage> [--only NAME]

Stages: ``import-loader``, ``baseline``, ``pass2``, ``pass3``, ``pass4``, ``verify``.
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
LANGUAGE_ID = "mn10300:LE:32:default"
COMPILER_ID = "default"

LOADER_NAME = "eh55_loader_ver061_256k.bin"
LOADER_SRC = REPO / "firmware_and_images" / LOADER_NAME
LOADER_BASE = 0x44000000

STAGES = {
    "BOOT.BIN": ("/BOOT.BIN", pathlib.Path(r"C:\ghidra\binaries\BOOT.BIN")),
    LOADER_NAME: ("/" + LOADER_NAME, LOADER_SRC),
    "eh55_frm_PROG061_BOOT.bin": ("/eh55_frm_PROG061_BOOT.bin", pathlib.Path(r"C:\ghidra\binaries\unpacked\eh55_frm_PROG061_BOOT.bin")),
    "DRV_decomp.bin": ("/DRV_decomp.bin", pathlib.Path(r"C:\ghidra\binaries\DRV_decomp.bin")),
    "EH55_DRV_S211.bin": ("/EH55_DRV_S211.bin", pathlib.Path(r"C:\ghidra\binaries\unpacked\EH55_DRV_S211.bin")),
    "EH55_MAIN_3410PQ.bin": ("/EH55_MAIN_3410PQ.bin", pathlib.Path(r"C:\ghidra\binaries\unpacked\EH55_MAIN_3410PQ.bin")),
}

STAGE_ENTRY_SEEDS = {
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

MMIO_WINDOWS = (
    ("PANADIGA_MMIO_10", 0x10000000, 0x100FFFFF),
    ("PANADIGA_NOR_CS1_CFI", 0x84000000, 0x847FFFFF),
    ("PANADIGA_AIOF_88", 0x88000000, 0x88FFFFFF),
    ("PANADIGA_ASIC_C0", 0xC0000000, 0xC0FFFFFF),
    ("PANADIGA_ATA_SC0_D4", 0xD4000000, 0xD4FFFFFF),
)

TAINTED_FUNCTIONS = (0x50423540, 0x5042358B, 0x5771ABFB, 0x4400C696, 0x4400D32F, 0x440236ED)
DECOMP_BAD_TOKENS = ("mn_todo", "halt_baddata", "Bad instruction", "BAD_INSTRUCTION")
SPOT_CHECK_OPCODES = {
    LOADER_NAME: (0x440242B7, 0x4400159A),
    "eh55_frm_PROG061_BOOT.bin": (0x504313E6, 0x504009E4, 0x5040EA2F),
    "EH55_MAIN_3410PQ.bin": (0x57000D2C,),
}

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


def guard_project() -> None:
    if PROJECT_NAME == SOURCE_PROJECT_NAME:
        raise SystemExit("refusing to operate on the read-only source project")


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


# ---------------------------------------------------------------------------
# Measurement & reporting
# ---------------------------------------------------------------------------

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


# ---------------------------------------------------------------------------
# Pass 1 - loader import
# ---------------------------------------------------------------------------

def import_loader(pyghidra, project) -> None:
    from generic.stl import Pair
    from ghidra.app.util.importer import ProgramLoader
    from ghidra.app.util.opinion import BinaryLoader
    from ghidra.util.task import TaskMonitor
    from java.io import File
    from java.util import ArrayList

    if project.getProjectData().getFile("/" + LOADER_NAME) is not None:
        print("[pass1] %s already present in %s" % (LOADER_NAME, PROJECT_NAME))
        return
    args = ArrayList()
    args.add(Pair("Base Address", "0x%X" % LOADER_BASE))
    loaded = (
        ProgramLoader.builder()
        .source(File(str(LOADER_SRC)))
        .project(project)
        .projectFolderPath("/")
        .language(LANGUAGE_ID)
        .compiler(COMPILER_ID)
        .loaders(BinaryLoader)
        .loaderArgs(args)
        .monitor(TaskMonitor.DUMMY)
        .load()
    )
    try:
        loaded.save(TaskMonitor.DUMMY)
        print("[pass1] imported %s at 0x%X" % (LOADER_NAME, LOADER_BASE))
    finally:
        loaded.close()


def layout_loader(project, pyghidra) -> None:
    """Move the raw ``ram`` block to 0x44000000 and mirror BOOT.BIN peripheral blocks."""
    from ghidra.util.task import TaskMonitor

    with pyghidra.program_context(project, "/BOOT.BIN") as ref:
        templates = [
            (b.getName(), to_u32(b.getStart().getOffset()), int(b.getSize()), b.isRead(), b.isWrite(), b.isExecute(), b.isVolatile())
            for b in ref.getMemory().getBlocks()
            if not b.isInitialized() and b.getName() != "mmio"
        ]
    with pyghidra.program_context(project, "/" + LOADER_NAME) as prog:
        mem = prog.getMemory()
        space = prog.getAddressFactory().getDefaultAddressSpace()
        with tx(prog, "Panadiga loader memory layout"):
            ram = mem.getBlock("ram")
            if to_u32(ram.getStart().getOffset()) != LOADER_BASE:
                mem.moveBlock(ram, space.getAddress(LOADER_BASE), TaskMonitor.DUMMY)
            for name, start, size, r, w, x, vol in templates:
                if mem.getBlock(name) is None:
                    blk = mem.createUninitializedBlock(name, space.getAddress(start), size, False)
                    blk.setRead(r)
                    blk.setWrite(w)
                    blk.setExecute(x)
                    blk.setVolatile(vol)
        prog.save("Panadiga loader layout", TaskMonitor.DUMMY)


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


def _type_bgo_and_jump_tables(prog, img: Image, stats: dict, seeds: set) -> None:
    from ghidra.program.model.data import Pointer32DataType, UnsignedIntegerDataType

    if img.base == 0x44000000 and img.contains(0x440337D0) and img.u32(0x440337D0) == 0x84130000:
        _type_undefined_array(prog, 0x440337A4, 70, UnsignedIntegerDataType.dataType, 4, "bgo_layout_table_u32", "structured_table_bytes", stats)
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
    pos = img.raw.find(S_BOX_SIGNATURE)
    while pos != -1:
        va = img.base + pos
        if pos + 256 <= img.size:
            typed = _type_undefined_bytes(prog, va, va + 255, "feistel_sbox", "structured_table_bytes", stats)
            if typed:
                set_eol(prog, va, "[Pass2] Feistel S-Box (256 bytes)")
        pos = img.raw.find(S_BOX_SIGNATURE, pos + 1)


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
    # Also type trailing erased/padding run at image tail when >= 8 bytes
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

    # Resume any 0xCB NOP delay sled hit inside this single-entry flow before validating.
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


def _seed_entries(prog, img: Image, name: str, pre: set) -> dict:
    stats = {"candidates": 0, "accepted": 0, "bytes_added": 0}
    for va in STAGE_ENTRY_SEEDS.get(name, ()):
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
# Pass 4 - normalization, residual typing, verification
# ---------------------------------------------------------------------------

def ensure_volatile_windows(prog) -> dict:
    mem = prog.getMemory()
    space = prog.getAddressFactory().getDefaultAddressSpace()
    created = 0
    for name, lo, hi in MMIO_WINDOWS:
        overlap = any(to_u32(b.getStart().getOffset()) <= hi and lo <= to_u32(b.getEnd().getOffset()) for b in mem.getBlocks())
        if not overlap:
            blk = mem.createUninitializedBlock(name, space.getAddress(lo), hi - lo + 1, False)
            blk.setRead(True)
            blk.setWrite(True)
            blk.setExecute(False)
            blk.setVolatile(True)
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
    typed_ranges = 0
    pad_bytes = 0
    raw_bytes = 0
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
    return {"residual_ranges": typed_ranges, "tiny_pad_bytes": pad_bytes, "residual_unclassified_bytes": raw_bytes}


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


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("stage", choices=["import-loader", "baseline", "pass2", "pass3", "pass4", "verify"])
    ap.add_argument("--only", default=None, help="restrict to one program name")
    ap.add_argument("--out", default=None, help="report file name override")
    ap.add_argument("--no-save", action="store_true", help="dry run: do not save the program")
    ap.add_argument("--no-switches", action="store_true", help="pass4: skip decompiler switch recovery")
    opts = ap.parse_args()
    guard_project()
    pyghidra = start_ghidra()
    from ghidra.util.task import TaskMonitor

    project = pyghidra.open_project(PROJECT_DIR, PROJECT_NAME)
    names = [opts.only] if opts.only else list(STAGES)

    if opts.stage == "import-loader":
        import_loader(pyghidra, project)
        layout_loader(project, pyghidra)
        return 0

    reports = []
    for name in names:
        proj_path, bin_path = STAGES[name]
        t0 = time.time()
        with pyghidra.program_context(project, proj_path) as prog:
            if opts.stage == "baseline":
                rep = measure(prog)
            elif opts.stage == "verify":
                rep = verify_program(prog, name)
            else:
                img = Image(prog, bin_path)
                if opts.stage == "pass2":
                    res = pass2_program(prog, img)
                    write_json(WORK_DIR, "pass2_seeds_%s.json" % name, res["seeds"])
                    rep = {"program": name, "pass2": res["stats"]}
                elif opts.stage == "pass3":
                    rep = {"program": name, "pass3": pass3_program(prog, img, name, load_seed_file(name))}
                else:
                    rep = {"program": name, "pass4": pass4_program(prog, img, name, load_seed_file(name), not opts.no_switches)}
                rep["after"] = measure(prog, with_todo=False)
                if not opts.no_save:
                    prog.save("Panadiga zero-undef %s" % opts.stage, TaskMonitor.DUMMY)
        rep["seconds"] = round(time.time() - t0, 1)
        reports.append(rep)
        print(json.dumps(rep, indent=2, default=str))
        sys.stdout.flush()

    dest_dir = REPORT_DIR if opts.stage in ("baseline", "verify") else WORK_DIR
    out_name = opts.out or ("%s%s.json" % (opts.stage, "_" + opts.only if opts.only else ""))
    print("report:", write_json(dest_dir, out_name, reports))
    return 0


if __name__ == "__main__":
    sys.exit(main())
