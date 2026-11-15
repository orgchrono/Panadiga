# Ghidra Script: Panasonic MN103 / AM34 Decompilation Frame, Purge & Switch Recovery
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

# =========================================================================
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


def _ensure_code_and_label(prog, addr_val, label_name, comment_text, monitor_obj=None):
    from ghidra.app.cmd.disassemble import DisassembleCommand
    from ghidra.app.cmd.function import CreateFunctionCmd
    from ghidra.program.model.listing import CodeUnit
    from ghidra.program.model.symbol import SourceType

    mem = prog.getMemory()
    addr = prog.getAddressFactory().getDefaultAddressSpace().getAddress(addr_val)
    if not mem.contains(addr):
        return False
    listing = prog.getListing()
    if listing.getInstructionContaining(addr) is None:
        DisassembleCommand(addr, None, True).applyTo(prog)
    if prog.getFunctionManager().getFunctionAt(addr) is None and listing.getInstructionAt(addr) is not None:
        CreateFunctionCmd(label_name, addr, None, SourceType.USER_DEFINED).applyTo(prog)
    try:
        sym = prog.getSymbolTable().createLabel(addr, label_name, SourceType.USER_DEFINED)
        if sym is not None:
            sym.setPrimary()
    except Exception:
        pass
    if comment_text:
        listing.setComment(addr, CodeUnit.PLATE_COMMENT, comment_text)
    return True

def _alloc_jbyte_buf_traps(count):
    try:
        import jarray
        return jarray.zeros(count, "b")
    except ImportError:
        import jpype
        return jpype.JArray(jpype.JByte)(count)

def _read_bytes_safe(prog, addr_val, count):
    mem_mgr = prog.getMemory()
    target_addr = prog.getAddressFactory().getDefaultAddressSpace().getAddress(addr_val)
    block = mem_mgr.getBlock(target_addr)
    if (block is None) or (not block.isInitialized()):
        return None
    avail = int(block.getEnd().subtract(target_addr)) + 1
    req = min(count, avail)
    if req <= 0:
        return None
    try:
        raw = _alloc_jbyte_buf_traps(req)
        read_len = mem_mgr.getBytes(target_addr, raw)
        return [int(val) & 0xFF for val in raw[:read_len]] if read_len > 0 else None
    except Exception:
        return None

def seed_orphan_tables(prog, monitor_obj=None):
    """Trap 1 resolver: seed orphan functions referenced by DRV CLI, ATAPI, BOOT, PROG061, and MAIN tables."""
    seeded = 0
    mem = prog.getMemory()
    cli_bytes = _read_bytes_safe(prog, 0x400DF5C0, 140 * 20)
    for rec in scan_drv_cli_entries(cli_bytes, 0x400DF5C0):
        if monitor_obj is not None:
            monitor_obj.checkCancelled()
        lbl = "drv_cli_%s" % rec["name"].replace(" ", "_")
        cmt = "[Trap1-OrphanDRV] entry=0x%08X flags=0x%X mask=0x%X" % (rec["entry_va"], rec["flags"], rec["arg_mask"])
        if _ensure_code_and_label(prog, rec["handler_va"], lbl, cmt, monitor_obj):
            seeded += 1
    for cdb0, name, va, note in KNOWN_DRV_ATAPI_SLOTS:
        if _ensure_code_and_label(prog, va, name, "[Trap1-ATAPI cdb=0x%02X] %s" % (cdb0, note), monitor_obj):
            seeded += 1
    _ensure_code_and_label(prog, 0x50401F3C, "prog061_nfrom_format", "[Trap1-Orphan] Factory NFROM/BGO formatter", monitor_obj)

    code_ranges = [
        (blk.getStart().getOffset() & 0xFFFFFFFF, (blk.getEnd().getOffset() & 0xFFFFFFFF) + 1)
        for blk in mem.getBlocks()
        if blk.isInitialized() and blk.isExecute()
    ]
    for tbl_va, tbl_cnt, tbl_lbl in KNOWN_ORPHAN_POINTER_TABLES:
        raw_tbl = _read_bytes_safe(prog, tbl_va, tbl_cnt * 4)
        for src_va, tgt_va in scan_pointer_table_runs(raw_tbl, tbl_va, code_ranges, min_run=2):
            if _ensure_code_and_label(prog, tgt_va, "%s_%08x" % (tbl_lbl, tgt_va), "[Trap1-TablePtr from 0x%08X]" % src_va, monitor_obj):
                seeded += 1
    return {"seeded": seeded}

def annotate_bitops_and_traps(prog):
    """Trap 2 resolver: annotate indirect BCLR/BSET instructions so RMW side effects are visible."""
    from ghidra.program.model.listing import CodeUnit
    listing = prog.getListing()
    bm_mgr = prog.getBookmarkManager()
    count = 0
    it = listing.getInstructions(True)
    while it.hasNext():
        ins = it.next()
        op_str = ",".join(ins.getDefaultOperandRepresentation(idx) for idx in range(ins.getNumOperands()))
        note = classify_mn103_bitop_trap(ins.getMnemonicString(), op_str)
        if note is not None:
            if listing.getComment(CodeUnit.EOL_COMMENT, ins.getAddress()) is None:
                listing.setComment(ins.getAddress(), CodeUnit.EOL_COMMENT, note)
            bm_mgr.setBookmark(ins.getAddress(), "Analysis", "Trap2-BitOp", note)
            count += 1
    return {"bitops_annotated": count}

def recover_unreachable_and_todo_blocks(prog, monitor_obj=None):
    """Trap 3 resolver: clear false NoReturn flags and disassemble stranded post-call code blocks."""
    from ghidra.app.cmd.disassemble import DisassembleCommand
    fm = prog.getFunctionManager()
    listing = prog.getListing()
    bm_mgr = prog.getBookmarkManager()
    noreturn_cleared = 0
    disassembled = 0
    for va in (0x574A2E60,):
        addr = prog.getAddressFactory().getDefaultAddressSpace().getAddress(va)
        fn = fm.getFunctionAt(addr)
        if fn is not None and fn.hasNoReturn():
            fn.setNoReturn(False)
            noreturn_cleared += 1
    for target_va in (0x574DD208, 0x5040A2E1):
        addr = prog.getAddressFactory().getDefaultAddressSpace().getAddress(target_va)
        if prog.getMemory().contains(addr) and listing.getInstructionContaining(addr) is None:
            if DisassembleCommand(addr, None, True).applyTo(prog):
                disassembled += 1
            bm_mgr.setBookmark(addr, "Analysis", "Trap3-RecoveredPostCall", "Forced disassembly past stack-return helper")
    return {"noreturn_cleared": noreturn_cleared, "post_call_disassembled": disassembled}

def resolve_mid_instruction_overlaps(prog, monitor_obj=None):
    """Trap 4 resolver: detect branch targets & CALL tails inside decoded instructions (?? collisions)."""
    from ghidra.app.cmd.disassemble import DisassembleCommand
    listing = prog.getListing()
    ref_mgr = prog.getReferenceManager()
    bm_mgr = prog.getBookmarkManager()
    detected = 0
    fixed = 0
    insn_iter = listing.getInstructions(True)
    while insn_iter.hasNext():
        ins = insn_iter.next()
        flows = ins.getFlows()
        if flows is None:
            continue
        for tgt in flows:
            cur = listing.getInstructionContaining(tgt)
            if cur is not None and not cur.getAddress().equals(tgt):
                detected += 1
                msg = "[Trap4-MidInsnOverlap] Branch from 0x%08X hits +%d inside insn at 0x%08X" % (
                    ins.getAddress().getOffset() & 0xFFFFFFFF,
                    int(tgt.getOffset() - cur.getAddress().getOffset()),
                    cur.getAddress().getOffset() & 0xFFFFFFFF,
                )
                bm_mgr.setBookmark(tgt, "Analysis", "Trap4-MidInsnOverlap", msg)
                if ref_mgr.getReferenceCountTo(cur.getAddress()) == 0:
                    listing.clearCodeUnits(cur.getMinAddress(), cur.getMaxAddress(), False)
                    if DisassembleCommand(tgt, None, True).applyTo(prog):
                        fixed += 1
    # Fix BOOT.BIN @ 0x4400493B 7-byte CALL stolen at 0x44004940 by bogus CLR D0
    space = prog.getAddressFactory().getDefaultAddressSpace()
    call_addr = space.getAddress(0x4400493B)
    stale_addr = space.getAddress(0x44004940)
    if prog.getMemory().contains(call_addr) and listing.getInstructionAt(call_addr) is None:
        raw = _read_bytes_safe(prog, 0x4400493B, 7)
        if raw == [0xDD, 0x0E, 0xFE, 0xFF, 0xFF, 0x00, 0x00]:
            listing.clearCodeUnits(stale_addr, stale_addr.add(1), False)
            if DisassembleCommand(call_addr, None, True).applyTo(prog):
                fixed += 1
    return {"overlaps_detected": detected, "overlaps_fixed": fixed}

def enforce_volatile_mmio_and_cfi_blocks(prog):
    """Trap 5 resolver: mark CS1 NOR Flash CFI & MMIO blocks volatile so decompiler keeps Flash probes."""
    from ghidra.program.model.listing import CodeUnit
    mem = prog.getMemory()
    marked_volatile = 0
    for blk in mem.getBlocks():
        base = blk.getStart().getOffset() & 0xFFFFFFFF
        name = blk.getName()
        is_cs1_nor = 0x84000000 <= base < 0x84800000
        is_mmio = (0x89000000 <= base < 0x89010000) or (0xC0000000 <= base < 0xC0010000) or (0xD4000000 <= base < 0xD4010000) or ("MMIO" in name)
        if (is_cs1_nor or is_mmio) and not blk.isVolatile():
            blk.setVolatile(True)
            marked_volatile += 1
    cfi_sites = 0
    addr = prog.getAddressFactory().getDefaultAddressSpace().getAddress(0x440236ED)
    if mem.contains(addr):
        prog.getListing().setComment(addr, CodeUnit.PLATE_COMMENT, "[Trap5-FlashCFI] Keep CS1 volatile; 0x84200554/0x84200AAA/0x84200000 are CFI command cycles")
        cfi_sites += 1
    return {"blocks_marked_volatile": marked_volatile, "cfi_annotated": cfi_sites}

def annotate_setlb_and_frame_traps(prog):
    """Trap 6 resolver: annotate SETLB..Lcc hardware loops (in_LAR-4) and negative A3 frames."""
    from ghidra.program.model.listing import CodeUnit
    listing = prog.getListing()
    bm_mgr = prog.getBookmarkManager()
    setlb_count = 0
    it = listing.getInstructions(True)
    while it.hasNext():
        ins = it.next()
        if ins.getMnemonicString().strip().lower() == "setlb":
            setlb_count += 1
            msg = "[Trap6-SETLB] Hardware loop latch; decompiler (*(code *)(in_LAR - 4))() equals loop body @ 0x%08X" % ((ins.getMaxAddress().getOffset() + 1) & 0xFFFFFFFF)
            listing.setComment(ins.getAddress(), CodeUnit.EOL_COMMENT, msg)
            bm_mgr.setBookmark(ins.getAddress(), "Analysis", "Trap6-SETLB", msg)
    return {"setlb_annotated": setlb_count}

def resolve_revision_shifts_cp932_and_false_consts(prog):
    """Trap 7 resolver: verify revision offsets, annotate CP932 error table, track false mid-insn hits."""
    from ghidra.program.model.listing import CodeUnit
    listing = prog.getListing()
    mem = prog.getMemory()
    space = prog.getAddressFactory().getDefaultAddressSpace()
    cp932_records = 0
    raw_tbl = _read_bytes_safe(prog, 0x400E779A, 64 * 0x34)
    if raw_tbl is not None:
        for idx in range(len(raw_tbl) // 0x34):
            rec = decode_cp932_mecha_record(raw_tbl[idx * 0x34:(idx + 1) * 0x34])
            if rec is None:
                continue
            addr = space.getAddress((0x400E779A + idx * 0x34) & 0xFFFFFFFF)
            if mem.contains(addr):
                listing.setComment(addr, CodeUnit.EOL_COMMENT, "[Trap7-CP932 %s] %s" % (rec["code_hex"], rec["text"]))
                cp932_records += 1
    rev_labeled = 0
    if _read_bytes_safe(prog, 0x400DF5C0, 8) is not None:
        for va, lbl, msg in (
            (0x4013420A, "drv_cmd_pass_unlock_decomp", "[Trap7-RevShift DRV_decomp] Sets DAT_00000cd8 |= 1"),
            (0x40134382, "drv_cmd_mmode_decomp", "[Trap7-RevShift DRV_decomp] MMODE handler"),
            (0x40133ED0, "drv_cmd_slogon_decomp", "[Trap7-RevShift DRV_decomp] SLOGON handler"),
        ):
            if _ensure_code_and_label(prog, va, lbl, msg, None):
                rev_labeled += 1
    elif mem.contains(space.getAddress(0x40130EFA)):
        if _ensure_code_and_label(prog, 0x40130EFA, "drv_cmd_pass_unlock_s211", "[Trap7-RevShift EH55_DRV_S211] BSET 0x1,(0xd78)", None):
            rev_labeled += 1
    return {"cp932_annotated": cp932_records, "revision_symbols": rev_labeled}

def annotate_am33_sleigh_gaps(prog, monitor_obj=None):
    """Trap 8 resolver: decode 0xF9 0xEA MOVHU (Rm0+),Rn2 gaps & resume disassembly after them."""
    from ghidra.app.cmd.disassemble import DisassembleCommand
    from ghidra.program.model.listing import CodeUnit
    listing = prog.getListing()
    bm_mgr = prog.getBookmarkManager()
    mem = prog.getMemory()
    f9_ea_sites = 0
    resumed = 0
    for blk in mem.getBlocks():
        if not (blk.isInitialized() and blk.isExecute()):
            continue
        start_va = blk.getStart().getOffset() & 0xFFFFFFFF
        raw = _read_bytes_safe(prog, start_va, int(blk.getSize()))
        if raw is None or len(raw) < 4:
            continue
        for i in range(len(raw) - 3):
            if raw[i] == 0xF9 and raw[i + 1] == 0xEA:
                site = blk.getStart().add(i)
                if listing.getInstructionContaining(site) is not None:
                    continue
                prev_cu = listing.getInstructionAt(site.subtract(1)) or listing.getInstructionContaining(site.subtract(1))
                if prev_cu is not None or (i > 0 and raw[i - 1] in (0x81, 0x98, 0xE2, 0xE4)):
                    decoded = decode_am33_f9_ea_postinc_load(raw[i], raw[i + 1], raw[i + 2])
                    msg = "[Trap8-AM33SleighGap] %s (missing in am33.sinc)" % decoded
                    listing.setComment(site, CodeUnit.EOL_COMMENT, msg)
                    bm_mgr.setBookmark(site, "Analysis", "Trap8-AM33-F9EA", msg)
                    f9_ea_sites += 1
                    after = site.add(3)
                    if listing.getInstructionContaining(after) is None:
                        if DisassembleCommand(after, None, True).applyTo(prog):
                            resumed += 1
    return {"f9_ea_annotated": f9_ea_sites, "post_f9_ea_resumed": resumed}

def resolve_all_seven_traps(prog, monitor_obj=None):
    """Run all MN103S Traps 1..7 + AM33 Sleigh gap resolvers and return structured diagnostics."""
    return {
        "trap1_orphans": seed_orphan_tables(prog, monitor_obj=monitor_obj),
        "trap2_bitops": annotate_bitops_and_traps(prog),
        "trap3_unreachable": recover_unreachable_and_todo_blocks(prog, monitor_obj=monitor_obj),
        "trap4_overlaps": resolve_mid_instruction_overlaps(prog, monitor_obj=monitor_obj),
        "trap5_cfi_volatile": enforce_volatile_mmio_and_cfi_blocks(prog),
        "trap6_setlb": annotate_setlb_and_frame_traps(prog),
        "trap7_rev_cp932": resolve_revision_shifts_cp932_and_false_consts(prog),
        "trap8_am33_sleigh": annotate_am33_sleigh_gaps(prog, monitor_obj=monitor_obj),
    }

def run_mn103_decompilation_fixes(prog=None, apply_purge=True, do_switches=True, resolve_traps=True):
    if prog is None:
        prog = currentProgram
    mon = globals().get("monitor")
    traps = resolve_all_seven_traps(prog, monitor_obj=mon) if resolve_traps else {}
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
    return {"call_frames": frames, "stack_purge": purge, "switches": switches, "traps": traps}

if __name__ == "__main__":
    run_mn103_decompilation_fixes()
