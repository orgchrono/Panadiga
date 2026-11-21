//! Pass 3 helpers (validated disassembly, Trap #4 handling, entry & generic header seeding,
//! orphan pointer propagation, bounded gap sweep) for `ghidra_zero_undef_pipeline.py`.

pub(super) const PY_PASS3: &str = r#"# ---------------------------------------------------------------------------
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
"#;
