//! Renders the MN103 frame/purge/switch decompilation recovery Ghidra script.

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
