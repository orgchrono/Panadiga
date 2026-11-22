//! Pass 3 bookmark inspection, reference scanning, generic reset-vector inference,
//! and batch function disassembly helpers for `ghidra_zero_undef_pipeline.py`.

pub(super) const PY_PASS3_FUNCS: &str = r#"def _iter_bookmarks(bm_mgr, kind: str):
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
"#;
