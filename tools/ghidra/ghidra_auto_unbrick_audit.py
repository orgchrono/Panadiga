# Ghidra Script: Panasonic DMR-EH55 Automated Stage & Unbrick Forensics Audit
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

ORIG_SUM_BEQ = (0xC8, 0x0A)
PATCH_SUM_BRA = (0xCA, 0x0A)
ORIG_DEVINFO_BEQ = (0xC8, 0x15)
PATCH_DEVINFO_BRA = (0xCA, 0x15)
ORIG_A2BC_BGO_CALL = (0xDD, 0xD2, 0x81, 0xFF, 0xFF, 0x00, 0x00)
PATCH_A2BC_BRA_MONITOR = (0xCA, 0x11, 0xCB, 0xCB, 0xCB, 0xCB, 0xCB)
ORIG_A2CD_HOOK = (0xFC, 0xCC, 0xB0, 0x3E, 0x43, 0x50, 0xF8, 0x07, 0xFC, 0xDD, 0xF7, 0xAA, 0xFF, 0xFF, 0x00, 0x00, 0x80, 0xFF, 0xCA, 0x32)
PATCH_A2CD_AUTOFORMAT = (0xDD, 0x6F, 0x7C, 0xFF, 0xFF, 0x00, 0x00, 0xA0, 0x00, 0xC8, 0x0B, 0x80, 0xFF, 0xCA, 0x37, 0xCB, 0xCB, 0xCB, 0xCB, 0xCB)
PATCH_A2CD_UART_MONITOR = (0xDD, 0xC7, 0xF1, 0x02, 0x00, 0x00, 0x00, 0xA0, 0x00, 0xC8, 0x30, 0xA0, 0x01, 0xC8, 0x07, 0x80, 0xFF, 0xCA, 0x33, 0xCB)

ORIG_FPC_STATUS_CTRL = (0xCF, 0xE0, 0xF8, 0xFE, 0xD0, 0x82)
PATCH_FPC_STATUS_CTRL = (0x80, 0x01, 0xF0, 0xFC, 0xCB, 0xCB)
ORIG_ODD_STATE_BRANCH = (0xBF, 0x03, 0xC1, 0x10)
PATCH_ODD_STATE_BRANCH = (0xCC, 0xD0, 0x00, 0xCB)
ORIG_ODD_INIT_STATUS = (0xDD, 0xCD, 0x3C, 0x00, 0x00, 0x00, 0x00)
PATCH_ODD_INIT_STATUS = (0x80, 0x00, 0xCB, 0xCB, 0xCB, 0xCB, 0xCB)

AUDIT_KEYWORDS = (
    "SetBusWidth",
    "-91006",
    "-90002",
    "BGO driver init",
    "Control information",
    "nfrom",
    "startup drive",
    "80040904",
    "_dev_SERIAL",
    "EXTRHEAD",
    "Firm list",
    "PANAMON1",
    "DDR CHECK",
)

AUDIT_MMIO_CONSTANTS = (
    (0x89001850, "SDCD_BUS_WIDTH_CFG"),
    (0xD4002008, "SC0TXB_UART0_TX_BYTE"),
    (0xD400200C, "SC0STR_UART0_STATUS"),
    (0x84130000, "FLASH_NOR_EH55_BGO_START"),
    (0x84200000, "FLASH_NOR_VER063_BGO_START"),
    (0x89000500, "DAVIS_IDE_PRI_ODD_CH0"),
    (0x89000D00, "DAVIS_IDE_SEC_HDD_CH1"),
    (0x88000000, "AIOF_VMODESEL"),
    (0x89800624, "AVDD_NPALCTL0"),
    (0xDC010000, "IC7501_TBUS_CTRL"),
)

CRITICAL_XREF_TARGETS = (
    (0x4400479F, "boot_uart0_getc_dead"),
    (0x4400488A, "boot_console_getline_dead"),
    (0x44027826, "boot_get_sd_status_1bit"),
    (0x44027F93, "boot_get_sd_status_4bit"),
    (0x50401F3C, "prog061_nfrom_format"),
    (0x574DD1FC, "main_hdd_cmd_lba48_clamp"),
    (0x574E499C, "main3060_hdd_cmd_lba48_clamp"),
    (0x57498738, "main3410_deviceinfo_check"),
    (0x574A01AA, "main3060_deviceinfo_check"),
)

def detect_stage_from_metadata(image_base, prog_name, strings_found, exec_size):
    name_up = (prog_name or "").upper()
    hits = set(strings_found or ())
    if image_base == 0x50400000:
        return "PROG061"
    if image_base == 0x57000000:
        return "MAIN3060NP" if (exec_size == 13427852 or "3060NP" in name_up) else "MAIN3410PQ"
    if image_base == 0x44000000:
        return "BOOT.BIN"
    if image_base == 0x90202000:
        return "DRV_S211"
    if image_base in (0x80000000, 0x84000000) or exec_size == 0x800000:
        return "NOR_8MB"
    if exec_size == 13427852 or "3060NP" in name_up:
        return "MAIN3060NP"
    if exec_size == 13003052 or "3410PQ" in name_up or "MAIN" in name_up or "_dev_SERIAL" in hits:
        return "MAIN3410PQ"
    if "PROG061" in name_up:
        return "PROG061"
    if exec_size == 1441792 or "DRV" in name_up:
        return "DRV_S211"
    if "BOOT" in name_up or "-91006" in hits:
        return "BOOT.BIN"
    if exec_size == 0x40000 and "BGO driver init" in hits:
        return "PROG061"
    if "SetBusWidth" in hits:
        return "BOOT.BIN"
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

def classify_prog061_sites(sum_bytes, devinfo_bytes, a2cd_bytes, a2bc_bytes=None):
    sum_state = _label_pair(sum_bytes, ORIG_SUM_BEQ, PATCH_SUM_BRA, "FACTORY", "BYPASSED")
    dev_state = _label_pair(devinfo_bytes, ORIG_DEVINFO_BEQ, PATCH_DEVINFO_BRA, "FACTORY", "BYPASSED")
    a2bc_t = tuple(b & 0xFF for b in a2bc_bytes) if a2bc_bytes is not None else None
    a2bc_state = (
        "NOT_CHECKED"
        if a2bc_bytes is None
        else _label_pair(a2bc_bytes, ORIG_A2BC_BGO_CALL, PATCH_A2BC_BRA_MONITOR, "FACTORY", "BRA_TO_MONITOR")
    )
    a2cd_t = tuple(b & 0xFF for b in a2cd_bytes) if a2cd_bytes is not None else None
    if a2cd_t == ORIG_A2CD_HOOK:
        a2cd_state = "FACTORY"
    elif a2cd_t == PATCH_A2CD_AUTOFORMAT:
        a2cd_state = "AUTO_FORMAT_BGO"
    elif a2cd_t == PATCH_A2CD_UART_MONITOR:
        if a2bc_t is not None and a2bc_t != PATCH_A2BC_BRA_MONITOR:
            a2cd_state = "UART_MONITOR_HOOK_CONDITIONAL_STALE"
        else:
            a2cd_state = "UART_MONITOR_PANAMON1"
    elif a2cd_t is None:
        a2cd_state = "UNMAPPED"
    else:
        a2cd_state = "CUSTOM"

    if sum_state == "FACTORY" and dev_state == "FACTORY" and a2cd_state == "FACTORY" and a2bc_state in ("FACTORY", "NOT_CHECKED"):
        mode = "FACTORY_ORIGINAL"
    elif sum_state == "BYPASSED" and dev_state == "BYPASSED" and a2cd_state == "FACTORY" and a2bc_state in ("FACTORY", "NOT_CHECKED"):
        mode = "BYPASS_CHECKS_ONLY"
    elif sum_state == "BYPASSED" and dev_state == "BYPASSED" and a2cd_state == "AUTO_FORMAT_BGO" and a2bc_state in ("FACTORY", "NOT_CHECKED"):
        mode = "AUTO_FORMAT_BGO"
    elif sum_state == "BYPASSED" and dev_state == "BYPASSED" and a2cd_state == "UART_MONITOR_PANAMON1" and a2bc_state in ("BRA_TO_MONITOR", "NOT_CHECKED"):
        mode = "UART_MONITOR_PANAMON1"
    else:
        mode = "CUSTOM_OR_PARTIAL"

    return {
        "mode": mode,
        "sites": {
            "0x504230F0_bgo_sum_beq": sum_state,
            "0x50405097_devinfo_check_beq": dev_state,
            "0x5040A2BC_bgo_init_call": a2bc_state,
            "0x5040A2CD_bgo_init_fail_hook": a2cd_state,
        },
    }

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

    return {
        "mode": mode,
        "sites": {
            "0x5771ABFB_fpc_status_ctrl": fpc_state,
            "0x574F7B8C_odd_state_branch": br_state,
            "0x574F7C5C_odd_init_status": st_state,
        },
    }

def scan_ascii_keywords(raw_bytes, base_addr, keywords=AUDIT_KEYWORDS, max_hits_per_kw=8):
    haystack = bytearray(raw_bytes)
    hits = {}
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
    results = {}
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

def _is_mid_instruction_literal_hit(prog, addr_val):
    listing = prog.getListing()
    addr = prog.getAddressFactory().getDefaultAddressSpace().getAddress(addr_val)
    cur = listing.getInstructionContaining(addr)
    if cur is None:
        return False
    # Trap 7 false-positive LE32/LE16 hit occurs when constant matches bytes straddling opcode+imm or imm inside insn without primary alignment
    return not cur.getAddress().equals(addr)

def _collect_xref_counts(prog):
    ref_mgr = prog.getReferenceManager()
    space = prog.getAddressFactory().getDefaultAddressSpace()
    mem = prog.getMemory()
    counts = {}
    for addr_val, name in CRITICAL_XREF_TARGETS + AUDIT_MMIO_CONSTANTS:
        addr = space.getAddress(addr_val)
        if mem.getBlock(addr) is None:
            continue
        refs = []
        mid_insn = []
        it = ref_mgr.getReferencesTo(addr)
        while it.hasNext() and len(refs) < 32:
            from_addr = it.next().getFromAddress()
            from_u32 = from_addr.getOffset() & 0xFFFFFFFF
            if _is_mid_instruction_literal_hit(prog, from_u32):
                mid_insn.append("0x%08X" % from_u32)
            else:
                refs.append("0x%08X" % from_u32)
        counts[name] = {"target": "0x%08X" % addr_val, "xref_count": len(refs), "callers": refs, "mid_insn_false_hits": mid_insn}
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
    first_init_base = None
    string_hits = {}
    mmio_lit_hits = {}
    for blk in mem.getBlocks():
        if not blk.isInitialized():
            continue
        blk_size = int(blk.getSize())
        exec_size += blk_size
        blk_base = blk.getStart().getOffset() & 0xFFFFFFFF
        if first_init_base is None:
            first_init_base = blk_base
        raw = _read_initialized_block_u8(blk)
        for kw, addrs in scan_ascii_keywords(raw, blk_base).items():
            string_hits.setdefault(kw, []).extend(addrs)
            _add_bookmark(bm_mgr, space, addrs[0], "Panadiga-Str", kw)
        for label, addrs in scan_le32_constants(raw, blk_base).items():
            mmio_lit_hits.setdefault(label, []).extend(addrs)
            _add_bookmark(bm_mgr, space, addrs[0], "Panadiga-MMIO", label)

    effective_base = image_base if image_base != 0 else (first_init_base or 0)
    stage = detect_stage_from_metadata(effective_base, prog.getName(), string_hits.keys(), exec_size)
    prog061_status = classify_prog061_sites(
        _read_prog_bytes(prog, 0x504230F0, 2),
        _read_prog_bytes(prog, 0x50405097, 2),
        _read_prog_bytes(prog, 0x5040A2CD, 20),
        _read_prog_bytes(prog, 0x5040A2BC, 7),
    )
    main3410_status = classify_main3410_sites(
        _read_prog_bytes(prog, 0x5771ABFB, 6),
        _read_prog_bytes(prog, 0x574F7B8C, 4),
        _read_prog_bytes(prog, 0x574F7C5C, 7),
    )

    for site_addr in ("0x504230F0", "0x50405097", "0x5040A2BC", "0x5040A2CD", "0x5771ABFB", "0x574F7B8C", "0x574F7C5C"):
        _add_bookmark(bm_mgr, space, site_addr, "Panadiga-Unbrick", "Verified Panadiga patch/audit site")

    report = {
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
    }

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
