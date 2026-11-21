//! Jython/PyGhidra pass executors for MN103S Traps 1..7 + AM33 Sleigh gap recovery.

pub(super) const PYTHON_PASSES: &str = r##"
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
        if read_len <= 0:
            return None
        try:
            return bytearray(memoryview(raw).cast("B"))[:read_len]
        except Exception:
            return [int(val) & 0xFF for val in raw[:read_len]]
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
        if raw is not None and list(raw) == [0xDD, 0x0E, 0xFE, 0xFF, 0xFF, 0x00, 0x00]:
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
    def _process_f9_ea(blk, raw, i):
        site = blk.getStart().add(i)
        if listing.getInstructionContaining(site) is not None:
            return (0, 0)
        prev_cu = listing.getInstructionAt(site.subtract(1)) or listing.getInstructionContaining(site.subtract(1))
        if prev_cu is not None or (i > 0 and raw[i - 1] in (0x81, 0x98, 0xE2, 0xE4)):
            decoded = decode_am33_f9_ea_postinc_load(raw[i], raw[i + 1], raw[i + 2])
            msg = "[Trap8-AM33SleighGap] %s (missing in am33.sinc)" % decoded
            listing.setComment(site, CodeUnit.EOL_COMMENT, msg)
            bm_mgr.setBookmark(site, "Analysis", "Trap8-AM33-F9EA", msg)
            after = site.add(3)
            resumed_inc = 0
            if listing.getInstructionContaining(after) is None:
                if DisassembleCommand(after, None, True).applyTo(prog):
                    resumed_inc = 1
            return (1, resumed_inc)
        return (0, 0)

    for blk in mem.getBlocks():
        if not (blk.isInitialized() and blk.isExecute()):
            continue
        start_va = blk.getStart().getOffset() & 0xFFFFFFFF
        raw = _read_bytes_safe(prog, start_va, int(blk.getSize()))
        if raw is None or len(raw) < 4:
            continue
        if isinstance(raw, (bytes, bytearray)):
            pos = 0
            end = len(raw) - 2
            while True:
                pos = raw.find(b"\xF9\xEA", pos, end)
                if pos < 0:
                    break
                f_inc, r_inc = _process_f9_ea(blk, raw, pos)
                f9_ea_sites += f_inc
                resumed += r_inc
                pos += 1
        else:
            for i in range(len(raw) - 3):
                if raw[i] == 0xF9 and raw[i + 1] == 0xEA:
                    f_inc, r_inc = _process_f9_ea(blk, raw, i)
                    f9_ea_sites += f_inc
                    resumed += r_inc
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
"##;
