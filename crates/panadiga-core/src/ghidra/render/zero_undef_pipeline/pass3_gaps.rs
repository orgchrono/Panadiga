//! Pass 3 orphan pointer seeding, bounded gap sweep, Trap #4 bookmarking,
//! and orchestrator (`pass3_program`) for `ghidra_zero_undef_pipeline.py`.

pub(super) const PY_PASS3_GAPS: &str = r#"def _collect_pointer_seeds(prog, img: Image, name: str, pass2_seeds: list) -> dict:
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
"#;
