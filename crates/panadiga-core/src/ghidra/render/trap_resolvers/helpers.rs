//! Pure Jython/Python helper functions & lookup tables for MN103S Traps 1..7 + AM33 Sleigh gaps.

pub(super) const PYTHON_HELPERS: &str = r##"# =========================================================================
# Panadiga MN103S Ghidra Traps & AM33 Sleigh Gap Resolvers (Helpers)
# =========================================================================

KNOWN_ORPHAN_POINTER_TABLES = (
    # (start_va, entry_count, table_label)
    (0x44035EB0, 6, "boot_switch_jump_table"),
    (0x5040EA30, 16, "prog061_trap4_jump_table"),
    (0x574DD010, 16, "main_hdd_cmd_state_table"),
    (0x400DEC9C, 9, "drv_atapi_switch_table_0"),
    (0x400DF3CC, 15, "drv_handler_table_1"),
    (0x400DF47C, 15, "drv_handler_table_2"),
)

KNOWN_DRV_ATAPI_SLOTS = (
    # (cdb0, slot_name, handler_va, notes)
    (0x3B, "drv_atapi_write_buffer_3b", 0x4002168D, "MMC WRITE BUFFER (microcode/RAM payload)"),
    (0x3C, "drv_atapi_read_buffer_3c", 0x40021476, "MMC READ BUFFER (SRAM/SDRAM/buffer readback)"),
    (0xED, "drv_atapi_vendor_ed_209f3", 0x400209F3, "Matsushita 0xED buffer dispatcher (DAT_00000242 0x91/0x12/0x93/0x13)"),
    (0xFD, "drv_atapi_vendor_fd_20147", 0x40020147, "Matsushita 0xFD vendor handler"),
    (0xFE, "drv_atapi_vendor_fe_2025e", 0x4002025E, "Matsushita 0xFE vendor MMODE/SLOG/unlock"),
)

KNOWN_UNIMPL_STUBS = (0x40014559,)
AM33_REG_NAMES = ("R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7", "A0", "A1", "A2", "A3", "D0", "D1", "D2", "D3")

def _u32_le_from_bytes(buf, off):
    return (
        (buf[off] & 0xFF)
        | ((buf[off + 1] & 0xFF) << 8)
        | ((buf[off + 2] & 0xFF) << 16)
        | ((buf[off + 3] & 0xFF) << 24)
    )

def _u16_le_from_bytes(buf, off):
    return (buf[off] & 0xFF) | ((buf[off + 1] & 0xFF) << 8)

def scan_drv_cli_entries(raw_bytes, base_va, max_entries=160):
    """Trap 1 helper: parse DRV <8sIII> 20-byte CLI dispatch records cleanly."""
    if raw_bytes is None:
        return []
    out = []
    limit = min(len(raw_bytes) // 20, max_entries)
    for idx in range(limit):
        off = idx * 20
        name_raw = raw_bytes[off:off + 8]
        nul = len(name_raw)
        for i in range(8):
            if (name_raw[i] & 0xFF) == 0:
                nul = i
                break
        if nul == 0:
            break
        ok = True
        chars = []
        for i in range(nul):
            ch = name_raw[i] & 0xFF
            if ch < 0x20 or ch > 0x7E:
                ok = False
                break
            chars.append(chr(ch))
        if not ok:
            break
        out.append({
            "index": idx,
            "entry_va": (base_va + off) & 0xFFFFFFFF,
            "name": "".join(chars),
            "handler_va": _u32_le_from_bytes(raw_bytes, off + 8),
            "flags": _u32_le_from_bytes(raw_bytes, off + 12),
            "arg_mask": _u32_le_from_bytes(raw_bytes, off + 16),
        })
    return out

def scan_pointer_table_runs(raw_bytes, base_va, code_ranges, min_run=6, stub_vas=KNOWN_UNIMPL_STUBS):
    """Trap 1 helper: find contiguous runs of LE32 code pointers inside initialized regions."""
    if raw_bytes is None or not code_ranges:
        return []
    stub_set = set(stub_vas)
    hits = []
    run = []
    limit = (len(raw_bytes) // 4) * 4
    for off in range(0, limit, 4):
        val = _u32_le_from_bytes(raw_bytes, off)
        in_code = any(lo <= val < hi for lo, hi in code_ranges)
        if in_code:
            run.append(((base_va + off) & 0xFFFFFFFF, val))
        else:
            if len(run) >= min_run:
                hits.extend((s, t) for s, t in run if t not in stub_set)
            run = []
    if len(run) >= min_run:
        hits.extend((s, t) for s, t in run if t not in stub_set)
    return hits

def classify_mn103_bitop_trap(mnemonic, operand_text):
    """Trap 2 helper: identify indirect BCLR 0xff / BSET RMW bitfield sites."""
    m = (mnemonic or "").strip().lower()
    op = (operand_text or "").strip().lower()
    if m not in ("bclr", "bset", "btst"):
        return None
    if "0xff" in op or "0x0,a" in op or "(a" in op or ",(" in op:
        return "[Trap2-BitOp] %s %s: indirect RMW bitmask side effect" % (m.upper(), operand_text)
    return None

def classify_decompiler_warning(c_text):
    """Trap 3 helper: detect decompiler unreachable block, mn_todo, and halt_baddata warnings."""
    if not c_text:
        return []
    flags = []
    for needle, label in (
        ("Removing unreachable block", "UNREACHABLE_BLOCK_PRUNED"),
        ("mn_todo", "MN_TODO_PCODE"),
        ("halt_baddata", "HALT_BADDATA"),
        ("in_LAR", "IN_LAR_HARDWARE_LOOP"),
    ):
        if needle in c_text:
            flags.append(label)
    if "in_A3" in c_text or "+ -" in c_text:
        flags.append("NEGATIVE_FRAME_A3")
    return flags

def detect_mid_instruction_overlap(insn_spans, flow_targets):
    """Trap 4 helper: find branch/jump targets landing strictly inside existing instructions."""
    overlaps = []
    for tgt in flow_targets:
        for start_va, end_va in insn_spans:
            if start_va < tgt < end_va:
                overlaps.append({"target_va": tgt, "containing_start": start_va, "containing_end": end_va})
                break
    return overlaps

def is_flash_cfi_address(va):
    """Trap 5 helper: detect uncached/cached Spansion CFI/JEDEC command bus addresses."""
    u = va & 0xFFFFFFFF
    low = u & 0x0FFF
    in_nor = (0x84000000 <= u < 0x847F0000) or (0x80000000 <= u < 0x807F0000)
    return in_nor and (low in (0x000, 0x554, 0x555, 0xAAA, 0xAAB))

def analyze_setlb_loop_and_frame(opcodes, base_va):
    """Trap 6 helper: pair MN103 SETLB (0xDB) with closing Lcc/LRA (0xD0..0xDF)."""
    if opcodes is None:
        return []
    pairs = []
    n = len(opcodes)
    for i in range(n):
        if (opcodes[i] & 0xFF) == 0xDB:
            body_va = (base_va + i + 1) & 0xFFFFFFFF
            max_j = min(n, i + 64)
            for j in range(i + 1, max_j):
                op = opcodes[j] & 0xFF
                if 0xD0 <= op <= 0xD9 or op in (0xDC, 0xDF):
                    pairs.append({
                        "setlb_va": (base_va + i) & 0xFFFFFFFF,
                        "loop_body_va": body_va,
                        "loop_end_va": (base_va + j) & 0xFFFFFFFF,
                        "end_opcode": op,
                    })
                    break
    return pairs

def identify_firmware_revision_profile(blob_len, keywords):
    """Trap 7 helper: distinguish DRV_decomp vs EH55_DRV_S211 vs BOOT/PROG061/MAIN revisions."""
    kws = set(keywords or ())
    if "SLOGON" in kws or "MMODE" in kws or blob_len in (1396188, 1396224):
        return {
            "stage": "DRV",
            "revision": "DRV_decomp",
            "pass_handler": 0x4013420A,
            "mmode_handler": 0x40134382,
            "cli_table_va": 0x400DF5C0,
        }
    if "S211" in kws or "R6-MV4" in kws or blob_len == 2097152:
        return {
            "stage": "DRV",
            "revision": "EH55_DRV_S211",
            "pass_handler": 0x40130EFA,
            "mmode_handler": None,
            "cli_table_va": None,
        }
    if "Loader ver0.61" in kws or blob_len == 262144:
        return {"stage": "BOOT_OR_PROG061", "revision": "ver0.61", "cli_table_va": None}
    return {"stage": "UNKNOWN", "revision": "generic", "cli_table_va": None}

def decode_cp932_mecha_record(raw_52b):
    """Trap 7 helper: decode 52-byte <u16 code><CP932 text> DRV error dictionary record."""
    if raw_52b is None or len(raw_52b) < 4:
        return None
    code = _u16_le_from_bytes(raw_52b, 0)
    if code == 0 or code == 0xFFFF:
        return None
    payload = bytes(bytearray(b & 0xFF for b in raw_52b[2:52])).split(b"\x00", 1)[0]
    try:
        text = payload.decode("cp932", errors="replace")
    except Exception:
        text = "".join(chr(b) if 0x20 <= b <= 0x7E else "." for b in payload)
    return {"code": code, "code_hex": "0x%04X" % code, "text": text}

def is_inside_decoded_instruction(va, insn_spans):
    """Trap 7 helper: filter false-positive LE16/LE32 hits inside opcode bytes (e.g. 0x4006f5e2)."""
    return any(start_va < va < end_va for start_va, end_va in insn_spans)

def decode_am33_f9_ea_postinc_load(b0, b1, b2):
    """Trap 8 helper: decode 0xF9 0xEA <reg> -> MOVHU (Rm0+), Rn2 missing from am33.sinc."""
    if (b0 & 0xFF) != 0xF9 or (b1 & 0xFF) != 0xEA:
        return None
    reg = b2 & 0xFF
    return "MOVHU (%s+),%s" % (AM33_REG_NAMES[reg & 0x0F], AM33_REG_NAMES[(reg >> 4) & 0x0F])
"##;
