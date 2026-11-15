//! Pure deterministic Jython 2.7 / Python-3 script renderers driven by `specs`.

use super::specs::{
    AUDIT_KEYWORDS, DRV_S211_EXPECTED_SIZE, LabeledAddrSpec, MAIN3060_EXPECTED_SIZE,
    MAIN3410_EXPECTED_SIZE, MemoryRegionSpec, SymbolSpec, audit_mmio_constants,
    critical_xref_targets, memory_region_specs, symbol_specs,
};
use crate::constants::*;
use crate::re::bgo_unbrick::{
    OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, ORIG_A2CD_HOOK,
    ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PATCH_A2CD_AUTOFORMAT, PATCH_DEVINFO_BRA, PATCH_SUM_BRA,
    PROG061_BASE_VA, PROG061_EXPECTED_SIZE,
};
use crate::re::main_passthrough::{
    MAIN_BASE_VA, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
    ORIG_FPC_STATUS_CTRL, ORIG_ODD_INIT_STATUS, ORIG_ODD_STATE_BRANCH, PATCH_FPC_STATUS_CTRL,
    PATCH_ODD_INIT_STATUS, PATCH_ODD_STATE_BRANCH,
};
use crate::re::prog061_uart_monitor::PATCH_A2CD_UART_MONITOR;
use std::fmt::Write as _;

fn py_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

pub fn format_py_byte_tuple(bytes: &[u8]) -> String {
    let mut out = String::from("(");
    for (idx, b) in bytes.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        let _ = write!(&mut out, "0x{:02X}", b);
    }
    if bytes.len() == 1 {
        out.push(',');
    }
    out.push(')');
    out
}

fn render_memory_regions(specs: &[MemoryRegionSpec]) -> String {
    let mut out = String::new();
    for spec in specs {
        let _ = writeln!(
            &mut out,
            "        ({:?}, 0x{:08X}, 0x{:08X}, {}, {}, {}, {}),",
            spec.name,
            spec.start,
            spec.size,
            py_bool(spec.read),
            py_bool(spec.write),
            py_bool(spec.execute),
            py_bool(spec.is_volatile),
        );
    }
    out
}

fn render_symbol_specs(specs: &[SymbolSpec]) -> String {
    let mut out = String::new();
    for spec in specs {
        let _ = writeln!(
            &mut out,
            "        (0x{:08X}, {:?}, {:?}, {}),",
            spec.address,
            spec.name,
            spec.comment,
            py_bool(spec.is_primary),
        );
    }
    out
}

fn render_labeled_tuples(specs: &[LabeledAddrSpec]) -> String {
    let mut out = String::new();
    for spec in specs {
        let _ = writeln!(&mut out, "    (0x{:08X}, {:?}),", spec.address, spec.label);
    }
    out
}

fn render_keywords() -> String {
    let mut out = String::new();
    for kw in AUDIT_KEYWORDS {
        let _ = writeln!(&mut out, "    {:?},", kw);
    }
    out
}

/// Generates the Ghidra Memory Map setup Python script (Jython 2.7 + Python 3 compatible).
pub fn generate_ghidra_memory_map() -> String {
    let regions_block = render_memory_regions(&memory_region_specs());
    format!(
        r#"# Ghidra Script: Panasonic DMR-EH55 Memory Map Setup
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT)
#
# Configures the complete memory map of Panasonic AM34 / MN103S (UniPhier)
# matching the DMR-EH55 VEP79138K digital board layout (Jython 2.7 + Py3 safe).

from __future__ import print_function

def _range_overlaps_existing(mem, start, size):
    end = start + size - 1
    for blk in mem.getBlocks():
        blk_start = blk.getStart().getOffset() & 0xFFFFFFFF
        blk_end = blk.getEnd().getOffset() & 0xFFFFFFFF
        if start <= blk_end and blk_start <= end:
            return blk.getName()
    return None

def setup_memory_map(prog=None):
    if prog is None:
        prog = currentProgram
    mem = prog.getMemory()
    addr_factory = prog.getAddressFactory()
    default_space = addr_factory.getDefaultAddressSpace()

    regions = [
        # (Name, StartHex, SizeBytes, Read, Write, Execute, IsVolatile)
{regions_block}    ]

    print("[Panadiga] Configuring Panasonic DMR-EH55 Memory Map...")
    created = 0
    for name, start, size, r, w, x, vol in regions:
        addr = default_space.getAddress(start)
        overlap = _range_overlaps_existing(mem, start, size)
        if overlap is None:
            try:
                block = mem.createUninitializedBlock(name, addr, size, False)
                block.setRead(r)
                block.setWrite(w)
                block.setExecute(x)
                block.setVolatile(vol)
                block.setComment("Panasonic DMR-EH55 HW Subsystem: " + name)
                created += 1
                print("  [+] Created block: %s at 0x%08X (size: 0x%X)" % (name, start, size))
            except Exception as e:
                print("  [-] Could not create block %s: %s" % (name, str(e)))
        else:
            print("  [*] Overlap skipped for %s at 0x%08X (covered by %s)" % (name, start, overlap))

    print("[Panadiga] Memory Map setup complete! (%d new blocks)" % created)
    return created

if __name__ == "__main__":
    setup_memory_map()
"#
    )
}

/// Generates the Ghidra Symbols and Peripheral importer script (Jython 2.7 + Python 3 safe).
pub fn generate_ghidra_symbols() -> String {
    let symbols_block = render_symbol_specs(&symbol_specs());
    format!(
        r#"# Ghidra Script: Panasonic DMR-EH55 Symbol & Peripheral Importer
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT)

from __future__ import print_function

try:
    from ghidra.program.model.listing import CodeUnit
    from ghidra.program.model.symbol import SourceType
except ImportError:
    CodeUnit = None
    SourceType = None

def get_panadiga_symbol_specs():
    return [
{symbols_block}    ]

def import_symbols(prog=None):
    if prog is None:
        prog = currentProgram
    sym_table = prog.getSymbolTable()
    listing = prog.getListing()
    mem = prog.getMemory()
    addr_factory = prog.getAddressFactory()
    default_space = addr_factory.getDefaultAddressSpace()
    plate_comment = CodeUnit.PLATE_COMMENT if CodeUnit is not None else 3

    symbols = get_panadiga_symbol_specs()
    print("[Panadiga] Annotating Panasonic DMR-EH55 Symbols and Peripherals...")
    imported_count = 0
    skipped_unmapped = 0

    for addr_val, name, comment, is_primary in symbols:
        addr = default_space.getAddress(addr_val)
        if mem.getBlock(addr) is None:
            skipped_unmapped += 1
            continue
        try:
            sym = sym_table.createLabel(addr, name, SourceType.USER_DEFINED)
            if is_primary and sym is not None:
                sym.setPrimary()
            listing.setComment(addr, plate_comment, comment)
            imported_count += 1
            print("  [+] Labeled 0x%08X -> %s" % (addr_val, name))
        except Exception as e:
            listing.setComment(addr, plate_comment, comment)
            print("  [*] Updated comment at 0x%08X -> %s (%s)" % (addr_val, name, str(e)))

    print("[Panadiga] Symbol import complete! Annotated %d symbols (%d unmapped skipped)." % (imported_count, skipped_unmapped))
    return imported_count

if __name__ == "__main__":
    import_symbols()
"#
    )
}

/// Generates an automated Ghidra Jython 2.7 / Python-3 stage detection, patch-verification,
/// MMIO-constant & XREF audit script (`ghidra_auto_unbrick_audit.py`).
pub fn generate_ghidra_auto_unbrick_audit() -> String {
    let kw_block = render_keywords();
    let mmio_block = render_labeled_tuples(&audit_mmio_constants());
    let xref_block = render_labeled_tuples(&critical_xref_targets());

    format!(
        r#"# Ghidra Script: Panasonic DMR-EH55 Automated Stage & Unbrick Forensics Audit
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT)
#
# Auto-detects open firmware (BOOT.BIN, PROG061, MAIN3410PQ, MAIN3060NP, DRV_S211, NOR),
# maps missing AM34 MMIO/SDRAM blocks, imports SSOT symbols, verifies live BGO & AV TBC
# opcodes, hunts unindexed MMIO literals & diagnostic strings, adds bookmarks, and writes JSON.

from __future__ import print_function

import json
import os
import struct
import sys
import tempfile

_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if _SCRIPT_DIR not in sys.path:
    sys.path.insert(0, _SCRIPT_DIR)

from ghidra_am34_memory_map import setup_memory_map
from ghidra_import_symbols_panadiga import import_symbols

ORIG_SUM_BEQ = {orig_sum_beq}
PATCH_SUM_BRA = {patch_sum_bra}
ORIG_DEVINFO_BEQ = {orig_devinfo_beq}
PATCH_DEVINFO_BRA = {patch_devinfo_bra}
ORIG_A2CD_HOOK = {orig_a2cd}
PATCH_A2CD_AUTOFORMAT = {patch_a2cd_auto}
PATCH_A2CD_UART_MONITOR = {patch_a2cd_mon}

ORIG_FPC_STATUS_CTRL = {orig_fpc}
PATCH_FPC_STATUS_CTRL = {patch_fpc}
ORIG_ODD_STATE_BRANCH = {orig_odd_br}
PATCH_ODD_STATE_BRANCH = {patch_odd_br}
ORIG_ODD_INIT_STATUS = {orig_odd_st}
PATCH_ODD_INIT_STATUS = {patch_odd_st}

AUDIT_KEYWORDS = (
{kw_block})

AUDIT_MMIO_CONSTANTS = (
{mmio_block})

CRITICAL_XREF_TARGETS = (
{xref_block})

def detect_stage_from_metadata(image_base, prog_name, strings_found, exec_size):
    name_up = (prog_name or "").upper()
    hits = set(strings_found or ())
    if image_base == 0x{boot_base:08X} or "BOOT" in name_up or "SetBusWidth" in hits or "-91006" in hits:
        return "BOOT.BIN"
    if image_base == 0x{prog_base:08X} or exec_size == 0x{prog_size:X} or "PROG061" in name_up or "BGO driver init" in hits:
        return "PROG061"
    if exec_size == {main3060_size} or "3060NP" in name_up:
        return "MAIN3060NP"
    if image_base == 0x{main_base:08X} or exec_size == {main3410_size} or "3410PQ" in name_up or "MAIN" in name_up:
        return "MAIN3410PQ"
    if image_base == 0x{drv_base:08X} or exec_size == {drv_size} or "DRV" in name_up:
        return "DRV_S211"
    if image_base in (0x{flash_base:08X}, 0x{nor_uncached:08X}) or exec_size == 0x{flash_size:X}:
        return "NOR_8MB"
    return "UNKNOWN"

def _label_pair(actual, orig, patch, orig_name, patch_name):
    if actual is None:
        return "UNMAPPED"
    t = tuple(b & 0xFF for b in actual)
    if t == orig:
        return orig_name
    if t == patch:
        return patch_name
    return "CUSTOM(" + " ".join(["%02X" % b for b in t[:8]]) + ")"

def classify_prog061_sites(sum_bytes, devinfo_bytes, a2cd_bytes):
    sum_state = _label_pair(sum_bytes, ORIG_SUM_BEQ, PATCH_SUM_BRA, "FACTORY", "BYPASSED")
    dev_state = _label_pair(devinfo_bytes, ORIG_DEVINFO_BEQ, PATCH_DEVINFO_BRA, "FACTORY", "BYPASSED")
    a2cd_t = tuple(b & 0xFF for b in a2cd_bytes) if a2cd_bytes is not None else None
    if a2cd_t == ORIG_A2CD_HOOK:
        a2cd_state = "FACTORY"
    elif a2cd_t == PATCH_A2CD_AUTOFORMAT:
        a2cd_state = "AUTO_FORMAT_BGO"
    elif a2cd_t == PATCH_A2CD_UART_MONITOR:
        a2cd_state = "UART_MONITOR_PANAMON1"
    elif a2cd_t is None:
        a2cd_state = "UNMAPPED"
    else:
        a2cd_state = "CUSTOM"

    if sum_state == "FACTORY" and dev_state == "FACTORY" and a2cd_state == "FACTORY":
        mode = "FACTORY_ORIGINAL"
    elif sum_state == "BYPASSED" and dev_state == "BYPASSED" and a2cd_state == "FACTORY":
        mode = "BYPASS_CHECKS_ONLY"
    elif sum_state == "BYPASSED" and dev_state == "BYPASSED" and a2cd_state in ("AUTO_FORMAT_BGO", "UART_MONITOR_PANAMON1"):
        mode = a2cd_state
    else:
        mode = "CUSTOM_OR_PARTIAL"

    return {{
        "mode": mode,
        "sites": {{
            "0x{prog_sum:08X}_bgo_sum_beq": sum_state,
            "0x{prog_devinfo:08X}_devinfo_check_beq": dev_state,
            "0x{prog_a2cd:08X}_bgo_init_fail_hook": a2cd_state,
        }},
    }}

def classify_main3410_sites(fpc_bytes, branch_bytes, status_bytes):
    fpc_state = _label_pair(fpc_bytes, ORIG_FPC_STATUS_CTRL, PATCH_FPC_STATUS_CTRL, "FACTORY", "PATCHED")
    br_state = _label_pair(branch_bytes, ORIG_ODD_STATE_BRANCH, PATCH_ODD_STATE_BRANCH, "FACTORY", "PATCHED")
    st_state = _label_pair(status_bytes, ORIG_ODD_INIT_STATUS, PATCH_ODD_INIT_STATUS, "FACTORY", "PATCHED")

    states = (fpc_state, br_state, st_state)
    if all(s == "FACTORY" for s in states):
        mode = "FACTORY_ORIGINAL"
    elif all(s == "PATCHED" for s in states):
        mode = "AV_PASSTHROUGH"
    else:
        mode = "CUSTOM_OR_PARTIAL"

    return {{
        "mode": mode,
        "sites": {{
            "0x{main_fpc_ctrl:08X}_fpc_status_ctrl": fpc_state,
            "0x{main_odd_branch:08X}_odd_state_branch": br_state,
            "0x{main_odd_status:08X}_odd_init_status": st_state,
        }},
    }}

def scan_ascii_keywords(raw_bytes, base_addr, keywords=AUDIT_KEYWORDS, max_hits_per_kw=8):
    haystack = bytearray(raw_bytes)
    hits = {{}}
    for kw in keywords:
        needle = bytearray(kw.encode("ascii"))
        pos = 0
        addrs = []
        while len(addrs) < max_hits_per_kw:
            idx = haystack.find(needle, pos)
            if idx < 0:
                break
            addrs.append("0x%08X" % (base_addr + idx))
            pos = idx + len(needle)
        if addrs:
            hits[kw] = addrs
    return hits

def scan_le32_constants(raw_bytes, base_addr, target_consts=AUDIT_MMIO_CONSTANTS, max_hits=16):
    haystack = bytearray(raw_bytes)
    results = {{}}
    for val, label in target_consts:
        needle = bytearray(struct.pack("<I", val))
        pos = 0
        addrs = []
        while len(addrs) < max_hits:
            idx = haystack.find(needle, pos)
            if idx < 0:
                break
            addrs.append("0x%08X" % (base_addr + idx))
            pos = idx + 1
        if addrs:
            results[label] = addrs
    return results

def _alloc_jbyte_buf(count):
    try:
        import jarray
        return jarray.zeros(count, "b")
    except ImportError:
        import jpype
        return jpype.JArray(jpype.JByte)(count)

def _read_prog_bytes(prog, addr_val, count):
    mem = prog.getMemory()
    addr = prog.getAddressFactory().getDefaultAddressSpace().getAddress(addr_val)
    blk = mem.getBlock(addr)
    if blk is None or not blk.isInitialized():
        return None
    try:
        buf = _alloc_jbyte_buf(count)
        n = mem.getBytes(addr, buf)
        if n != count:
            return None
        return [int(b) & 0xFF for b in buf]
    except Exception:
        return None

def _read_initialized_block_u8(blk):
    size = int(blk.getSize())
    data = bytearray(size)
    chunk_size = 0x100000
    offset = 0
    while offset < size:
        n = min(chunk_size, size - offset)
        jbuf = _alloc_jbyte_buf(n)
        addr = blk.getStart().add(offset)
        blk.getBytes(addr, jbuf)
        try:
            data[offset:offset + n] = memoryview(jbuf).cast("B")
        except Exception:
            data[offset:offset + n] = bytearray(int(b) & 0xFF for b in jbuf)
        offset += n
    return data

def _add_bookmark(bm_mgr, space, addr_hex, category, comment):
    if bm_mgr is None:
        return
    try:
        addr = space.getAddress(int(addr_hex, 16))
        bm_mgr.setBookmark(addr, "Analysis", category, comment)
    except Exception:
        pass

def _collect_xref_counts(prog):
    ref_mgr = prog.getReferenceManager()
    space = prog.getAddressFactory().getDefaultAddressSpace()
    mem = prog.getMemory()
    counts = {{}}
    for addr_val, name in CRITICAL_XREF_TARGETS + AUDIT_MMIO_CONSTANTS:
        addr = space.getAddress(addr_val)
        if mem.getBlock(addr) is None:
            continue
        refs = []
        it = ref_mgr.getReferencesTo(addr)
        while it.hasNext() and len(refs) < 32:
            refs.append("0x%08X" % (it.next().getFromAddress().getOffset() & 0xFFFFFFFF))
        counts[name] = {{"target": "0x%08X" % addr_val, "xref_count": len(refs), "callers": refs}}
    return counts

def _default_report_path(prog):
    exe_path = prog.getExecutablePath()
    if exe_path:
        parent = os.path.dirname(os.path.abspath(exe_path))
        base = os.path.basename(exe_path) + ".panadiga_audit.json"
        if os.path.isdir(parent):
            return os.path.join(parent, base)
    return os.path.join(tempfile.gettempdir(), "panadiga_ghidra_audit.json")

def run_unbrick_audit(prog=None, output_json=None):
    if prog is None:
        prog = currentProgram
    setup_memory_map(prog)
    imported = import_symbols(prog)

    mem = prog.getMemory()
    space = prog.getAddressFactory().getDefaultAddressSpace()
    bm_mgr = prog.getBookmarkManager()
    image_base = prog.getImageBase().getOffset() & 0xFFFFFFFF

    exec_size = 0
    string_hits = {{}}
    mmio_lit_hits = {{}}
    for blk in mem.getBlocks():
        if not blk.isInitialized():
            continue
        blk_size = int(blk.getSize())
        exec_size += blk_size
        blk_base = blk.getStart().getOffset() & 0xFFFFFFFF
        raw = _read_initialized_block_u8(blk)
        for kw, addrs in scan_ascii_keywords(raw, blk_base).items():
            string_hits.setdefault(kw, []).extend(addrs)
            _add_bookmark(bm_mgr, space, addrs[0], "Panadiga-Str", kw)
        for label, addrs in scan_le32_constants(raw, blk_base).items():
            mmio_lit_hits.setdefault(label, []).extend(addrs)
            _add_bookmark(bm_mgr, space, addrs[0], "Panadiga-MMIO", label)

    stage = detect_stage_from_metadata(image_base, prog.getName(), string_hits.keys(), exec_size)
    prog061_status = classify_prog061_sites(
        _read_prog_bytes(prog, 0x{prog_sum:08X}, {orig_sum_len}),
        _read_prog_bytes(prog, 0x{prog_devinfo:08X}, {orig_devinfo_len}),
        _read_prog_bytes(prog, 0x{prog_a2cd:08X}, {orig_a2cd_len}),
    )
    main3410_status = classify_main3410_sites(
        _read_prog_bytes(prog, 0x{main_fpc_ctrl:08X}, {orig_fpc_len}),
        _read_prog_bytes(prog, 0x{main_odd_branch:08X}, {orig_odd_br_len}),
        _read_prog_bytes(prog, 0x{main_odd_status:08X}, {orig_odd_st_len}),
    )

    for site_addr in ("0x{prog_sum:08X}", "0x{prog_devinfo:08X}", "0x{prog_a2cd:08X}", "0x{main_fpc_ctrl:08X}", "0x{main_odd_branch:08X}", "0x{main_odd_status:08X}"):
        _add_bookmark(bm_mgr, space, site_addr, "Panadiga-Unbrick", "Verified Panadiga patch/audit site")

    report = {{
        "program": prog.getName(),
        "image_base": "0x%08X" % image_base,
        "initialized_bytes": exec_size,
        "detected_stage": stage,
        "symbols_annotated": imported,
        "prog061_bgo_status": prog061_status,
        "main3410_passthrough_status": main3410_status,
        "diagnostic_strings": string_hits,
        "mmio_literal_pools": mmio_lit_hits,
        "xrefs": _collect_xref_counts(prog),
    }}

    out_path = output_json or _default_report_path(prog)
    with open(out_path, "w") as fh:
        json.dump(report, fh, indent=2, sort_keys=True)

    print("=" * 72)
    print("[Panadiga Audit] Stage=%s | Base=0x%08X | PROG061=%s | MAIN3410=%s" % (
        stage, image_base, prog061_status["mode"], main3410_status["mode"]
    ))
    print("[Panadiga Audit] Strings matched: %d | MMIO literals: %d | Report: %s" % (
        len(string_hits), len(mmio_lit_hits), out_path
    ))
    print("=" * 72)
    return report

if __name__ == "__main__":
    run_unbrick_audit()
"#,
        orig_sum_beq = format_py_byte_tuple(&ORIG_SUM_BEQ),
        patch_sum_bra = format_py_byte_tuple(&PATCH_SUM_BRA),
        orig_devinfo_beq = format_py_byte_tuple(&ORIG_DEVINFO_BEQ),
        patch_devinfo_bra = format_py_byte_tuple(&PATCH_DEVINFO_BRA),
        orig_a2cd = format_py_byte_tuple(&ORIG_A2CD_HOOK),
        patch_a2cd_auto = format_py_byte_tuple(&PATCH_A2CD_AUTOFORMAT),
        patch_a2cd_mon = format_py_byte_tuple(&PATCH_A2CD_UART_MONITOR),
        orig_fpc = format_py_byte_tuple(&ORIG_FPC_STATUS_CTRL),
        patch_fpc = format_py_byte_tuple(&PATCH_FPC_STATUS_CTRL),
        orig_odd_br = format_py_byte_tuple(&ORIG_ODD_STATE_BRANCH),
        patch_odd_br = format_py_byte_tuple(&PATCH_ODD_STATE_BRANCH),
        orig_odd_st = format_py_byte_tuple(&ORIG_ODD_INIT_STATUS),
        patch_odd_st = format_py_byte_tuple(&PATCH_ODD_INIT_STATUS),
        boot_base = AM34_BOOT_BIN_BASE,
        prog_base = PROG061_BASE_VA,
        prog_size = PROG061_EXPECTED_SIZE,
        main3060_size = MAIN3060_EXPECTED_SIZE,
        main3410_size = MAIN3410_EXPECTED_SIZE,
        main_base = MAIN_BASE_VA,
        drv_base = AM34_DRV_S211_BASE,
        drv_size = DRV_S211_EXPECTED_SIZE,
        flash_base = AM34_FLASH_BASE,
        nor_uncached = AM34_FLASH_UNCACHED_BASE,
        flash_size = AM34_FLASH_SIZE,
        prog_sum = PROG061_BASE_VA + OFF_BGO_SUM_BEQ as u32,
        prog_devinfo = PROG061_BASE_VA + OFF_DEVINFO_CHECK_BEQ as u32,
        prog_a2cd = PROG061_BASE_VA + OFF_BGO_INIT_FAIL_HOOK as u32,
        main_fpc_ctrl = MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32,
        main_odd_branch = MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32,
        main_odd_status = MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32,
        orig_sum_len = ORIG_SUM_BEQ.len(),
        orig_devinfo_len = ORIG_DEVINFO_BEQ.len(),
        orig_a2cd_len = ORIG_A2CD_HOOK.len(),
        orig_fpc_len = ORIG_FPC_STATUS_CTRL.len(),
        orig_odd_br_len = ORIG_ODD_STATE_BRANCH.len(),
        orig_odd_st_len = ORIG_ODD_INIT_STATUS.len(),
    )
}

/// Generates a Jython 2.7 / Python-3 compatible MN103 decompilation fixer script
/// (`ghidra_mn103_fix_decompilation.py`) porting caller call-frame propagation
/// (`0xCD`/`0xDD`), stack-purge normalization (`ret -> purge 0`), and computed
/// switch jump-table recovery without repeating auto-analysis.
pub fn generate_ghidra_mn103_fix_decompilation() -> String {
    r#"# Ghidra Script: Panasonic MN103 / AM34 Decompilation Frame, Purge & Switch Recovery
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT; MN103 ISA + nsud-dev/ghidra-mn103)
#
# Jython 2.7 + Python 3 safe. Restores MN103 callee entry context (`ctx_entry`,
# `ctx_regs`, `ctx_frame`) from `call d16/d32,regs,imm8` sites, normalizes unknown
# or non-zero function stack purges to 0 (`ret [regs],N` restores caller SP),
# and recovers indirect `jmp (aN)` switch tables via the Ghidra decompiler.

from __future__ import print_function

MN103_REGMASK_ORDER = "d2 d3 a2 a3 other exreg0 exreg1 exother".split()

def decode_mn103_regmask(regs_byte):
    mask = regs_byte & 0xFF
    return tuple(name for idx, name in enumerate(MN103_REGMASK_ORDER) if (mask & (0x80 >> idx)) != 0)

def extract_mn103_call_frame(raw_bytes):
    if raw_bytes is None:
        return None
    n = len(raw_bytes)
    if n == 0:
        return None
    op = raw_bytes[0] & 0xFF
    if op == 0xCD and n == 5:
        return (raw_bytes[3] & 0xFF, raw_bytes[4] & 0xFF)
    if op == 0xDD and n == 7:
        return (raw_bytes[5] & 0xFF, raw_bytes[6] & 0xFF)
    return None

def classify_stack_purge(purge_size, unknown_sentinel=-0x80000000, invalid_sentinel=-0x7FFFFFFF):
    if purge_size in (unknown_sentinel, invalid_sentinel):
        return "UNKNOWN"
    if purge_size == 0:
        return "OK"
    return "NONZERO"

def _resolve_mn103_ctx_regs(prog_ctx):
    r_entry = prog_ctx.getRegister("ctx_entry")
    r_regs = prog_ctx.getRegister("ctx_regs")
    r_frame = prog_ctx.getRegister("ctx_frame")
    if r_entry is None or r_regs is None or r_frame is None:
        return None
    return (r_entry, r_regs, r_frame)

def fix_mn103_call_frames(prog, monitor_obj=None):
    from java.math import BigInteger
    from ghidra.app.cmd.disassemble import DisassembleCommand

    prog_ctx = prog.getProgramContext()
    ctx_tuple = _resolve_mn103_ctx_regs(prog_ctx)
    if ctx_tuple is None:
        return {"supported": False, "call_targets": 0, "context_set": 0, "redisassembled": 0, "conflicts": 0, "skipped": 0}
    r_entry, r_regs, r_frame = ctx_tuple

    listing = prog.getListing()
    mem = prog.getMemory()
    want = {}
    conflicts = 0
    insn_iter = listing.getInstructions(True)
    while insn_iter.hasNext():
        ins = insn_iter.next()
        parsed = extract_mn103_call_frame(ins.getBytes())
        if parsed is None:
            continue
        flows = ins.getFlows()
        if flows is None or len(flows) != 1:
            continue
        target = flows[0]
        prev = want.get(target)
        if prev is not None and prev != parsed:
            conflicts += 1
        want[target] = parsed

    redo = []
    ctx_set = 0
    skipped = 0
    for target, pair in want.items():
        if not mem.contains(target):
            continue
        ve = prog_ctx.getValue(r_entry, target, False)
        vr = prog_ctx.getValue(r_regs, target, False)
        vf = prog_ctx.getValue(r_frame, target, False)
        matches = (
            ve is not None and ve.intValue() == 1
            and vr is not None and vr.intValue() == pair[0]
            and vf is not None and vf.intValue() == pair[1]
        )
        if matches:
            continue
        cur = listing.getInstructionContaining(target)
        if cur is not None and not cur.getAddress().equals(target):
            skipped += 1
            continue
        if cur is not None:
            redo.append(target)
            listing.clearCodeUnits(target, target, False)
        try:
            prog_ctx.setValue(r_entry, target, target, BigInteger.ONE)
            prog_ctx.setValue(r_regs, target, target, BigInteger.valueOf(pair[0]))
            prog_ctx.setValue(r_frame, target, target, BigInteger.valueOf(pair[1]))
            ctx_set += 1
        except Exception:
            skipped += 1

    redone = 0
    for target in redo:
        if monitor_obj is not None:
            monitor_obj.checkCancelled()
        if DisassembleCommand(target, None, True).applyTo(prog):
            redone += 1

    return {
        "supported": True,
        "call_targets": len(want),
        "context_set": ctx_set,
        "redisassembled": redone,
        "conflicts": conflicts,
        "skipped": skipped,
    }

def fix_mn103_stack_purge(prog, apply_fix=True):
    from ghidra.program.model.listing import Function

    fm = prog.getFunctionManager()
    unknown_sentinel = Function.UNKNOWN_STACK_DEPTH_CHANGE
    invalid_sentinel = Function.INVALID_STACK_DEPTH_CHANGE
    counts = {"total": 0, "unknown": 0, "nonzero": 0, "fixed": 0}
    fn_iter = fm.getFunctions(True)
    while fn_iter.hasNext():
        fn = fn_iter.next()
        counts["total"] += 1
        purge_val = fn.getStackPurgeSize()
        kind = classify_stack_purge(purge_val, unknown_sentinel, invalid_sentinel)
        if kind == "UNKNOWN":
            counts["unknown"] += 1
        elif kind == "NONZERO":
            counts["nonzero"] += 1
        if kind != "OK" and apply_fix:
            fn.setStackPurgeSize(0)
            counts["fixed"] += 1
    return counts

def _find_computed_jump_functions(prog):
    listing = prog.getListing()
    fm = prog.getFunctionManager()
    seen = {}
    ordered = []
    insn_iter = listing.getInstructions(True)
    while insn_iter.hasNext():
        ins = insn_iter.next()
        flow = ins.getFlowType()
        if flow.isJump() and flow.isComputed():
            fn = fm.getFunctionContaining(ins.getAddress())
            if fn is not None:
                entry = fn.getEntryPoint()
                if entry not in seen:
                    seen[entry] = True
                    ordered.append(fn)
    return ordered

def recover_mn103_switches(prog, targets=None, max_passes=2, timeout_secs=60, monitor_obj=None):
    from ghidra.app.cmd.disassemble import DisassembleCommand
    from ghidra.app.cmd.function import CreateFunctionCmd
    from ghidra.app.decompiler import DecompInterface
    from ghidra.program.model.listing import CodeUnit
    from ghidra.program.model.symbol import RefType, SourceType

    todo = list(targets) if targets is not None else _find_computed_jump_functions(prog)
    decomp = DecompInterface()
    decomp.openProgram(prog)
    ref_mgr = prog.getReferenceManager()
    listing = prog.getListing()

    tables_recovered = 0
    cases_disassembled = 0
    passes_run = 0
    try:
        for _ in range(max_passes):
            passes_run += 1
            pass_new = 0
            for fn in todo:
                if monitor_obj is not None:
                    monitor_obj.checkCancelled()
                res = decomp.decompileFunction(fn, timeout_secs, monitor_obj)
                if not res.decompileCompleted():
                    continue
                hf = res.getHighFunction()
                if hf is None:
                    continue
                jts = hf.getJumpTables()
                if jts is None:
                    continue
                fn_changed = False
                for jt in jts:
                    sw = jt.getSwitchAddress()
                    jt_new = False
                    for case_addr in jt.getCases():
                        if listing.getInstructionAt(case_addr) is None:
                            DisassembleCommand(case_addr, None, True).applyTo(prog)
                            cases_disassembled += 1
                            jt_new = True
                        already = any(r.getToAddress().equals(case_addr) for r in ref_mgr.getReferencesFrom(sw))
                        if not already:
                            ref_mgr.addMemoryReference(
                                sw, case_addr, RefType.COMPUTED_JUMP, SourceType.ANALYSIS, CodeUnit.MNEMONIC
                            )
                            jt_new = True
                    if jt_new:
                        pass_new += 1
                        fn_changed = True
                if fn_changed:
                    CreateFunctionCmd.fixupFunctionBody(prog, fn, monitor_obj)
            tables_recovered += pass_new
            if pass_new == 0:
                break
    finally:
        decomp.dispose()

    return {
        "candidates": len(todo),
        "passes": passes_run,
        "tables_recovered": tables_recovered,
        "cases_disassembled": cases_disassembled,
    }

def run_mn103_decompilation_fixes(prog=None, apply_purge=True, do_switches=True):
    if prog is None:
        prog = currentProgram
    mon = globals().get("monitor")
    frames = fix_mn103_call_frames(prog, monitor_obj=mon)
    purge = fix_mn103_stack_purge(prog, apply_fix=apply_purge)
    switches = recover_mn103_switches(prog, monitor_obj=mon) if do_switches else {"candidates": 0, "passes": 0, "tables_recovered": 0, "cases_disassembled": 0}
    print(
        "[Panadiga MN103] Calls=%d ctx_set=%d redo=%d conflicts=%d | Purge unknown=%d fixed=%d | Switches=%d cases=%d"
        % (
            frames["call_targets"],
            frames["context_set"],
            frames["redisassembled"],
            frames["conflicts"],
            purge["unknown"],
            purge["fixed"],
            switches["tables_recovered"],
            switches["cases_disassembled"],
        )
    )
    return {"call_frames": frames, "stack_purge": purge, "switches": switches}

if __name__ == "__main__":
    run_mn103_decompilation_fixes()
"#.to_string()
}

pub fn render_pt_wrapper_script() -> &'static str {
    r#"# Ghidra Script: Panasonic DMR-EH55 Symbol Importer (Backward-Compatibility Wrapper)
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Jython 2.7 + Python 3 compatible)
from __future__ import print_function

import os
import runpy

_target = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ghidra_import_symbols_panadiga.py")
runpy.run_path(_target, init_globals={"currentProgram": globals().get("currentProgram")}, run_name="__main__")
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ghidra_scripts_use_verified_sc0_and_jython_safe_syntax() {
        let map_py = generate_ghidra_memory_map();
        let sym_py = generate_ghidra_symbols();
        let audit_py = generate_ghidra_auto_unbrick_audit();
        let fix_py = generate_ghidra_mn103_fix_decompilation();

        assert!(map_py.contains("SC0_UART0_MMIO") && map_py.contains("0xD4002000"));
        assert!(map_py.contains("SDCD_HOST_MMIO") && map_py.contains("0x89001800"));
        assert!(map_py.contains("FLASH_NOR_CS1_BGO") && map_py.contains("0x84200000"));
        assert!(sym_py.contains("SC0TXB_UART0_TX_BYTE") && sym_py.contains("0xD4002008"));
        assert!(sym_py.contains("SDCD_BUS_WIDTH_CFG") && sym_py.contains("0x89001850"));
        assert!(!sym_py.contains("0xC0000100"));
        assert!(
            fix_py.contains("decode_mn103_regmask") && fix_py.contains("extract_mn103_call_frame")
        );
        assert!(fix_py.contains("setStackPurgeSize(0)") && fix_py.contains("COMPUTED_JUMP"));

        assert!(
            !sym_py.contains("sym_table.setPrimary") && !sym_py.contains("listing.PLATE_COMMENT")
        );
        assert!(sym_py.contains("sym.setPrimary()") && sym_py.contains("CodeUnit.PLATE_COMMENT"));
        assert!(audit_py.contains("jpype.JArray(jpype.JByte)"));
        for script in [&map_py, &sym_py, &audit_py, &fix_py] {
            assert!(!script.contains("from pathlib import"));
            assert!(!script.contains("0xFFFFFFFFL"));
        }
    }
}
