//! Image validator, memory layout helpers, and generic binary importer
//! for the generated `ghidra_zero_undef_pipeline.py` script.

pub(super) const PY_BOOTSTRAP_LAYOUT: &str = r#"class Image:
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


def _create_uninit_block_if_free(mem, space, name: str, start: int, size: int, r: bool, w: bool, x: bool, vol: bool) -> bool:
    if size <= 0 or mem.getBlock(name) is not None:
        return False
    end = to_u32(start + size - 1)
    if any(to_u32(b.getStart().getOffset()) <= end and start <= to_u32(b.getEnd().getOffset()) for b in mem.getBlocks()):
        return False
    blk = mem.createUninitializedBlock(name, space.getAddress(start), size, False)
    blk.setRead(r)
    blk.setWrite(w)
    blk.setExecute(x)
    blk.setVolatile(vol)
    return True


def infer_base_va(bin_path: pathlib.Path, model_id: str, explicit_base: str | None) -> int:
    if explicit_base is not None:
        return to_u32(int(explicit_base, 0))
    if bin_path.name in STAGE_BASES:
        return STAGE_BASES[bin_path.name]
    upper = bin_path.name.upper()
    if bin_path.is_file():
        with bin_path.open("rb") as fh:
            head = fh.read(0x400)
        if head.startswith(b"\n root=/dev/fma"):
            return 0x90100000
        if "PROG" in upper or "DIGADIAG" in upper:
            for idx in range(max(0, len(head) - 6)):
                if head[idx] == 0xFC and 0xDC <= head[idx + 1] <= 0xDF:
                    imm = struct.unpack_from("<I", head, idx + 2)[0]
                    if 0x50800000 <= imm < 0x50840000:
                        return 0x50800000
                    if 0x50030400 <= imm < 0x50060000:
                        return 0x50030400
    if "DIGADIAG" in upper:
        return 0x50400000
    model_info = MODEL_MEMORY_MAPS.get(model_id, {})
    for pat, field in (("PROG", "prog_load_va"), ("MAIN", "main_load_va"), ("DRV", "drv_load_va")):
        if pat in upper and model_info.get(field):
            va = to_u32(int(model_info[field], 0))
            if va == 0x90400000:
                return 0x50400000
            if va == 0x90202000:
                return 0x40000000
            return va
    for hint in GENERIC_STAGE_RULES.get("filename_base_hints", ()):
        if hint["pattern"].upper() in upper:
            return to_u32(int(hint["base_va"], 0))
    return to_u32(int(GENERIC_STAGE_RULES.get("default_fallback_base_va", "0x40000000"), 0))


def import_binary(
    pyghidra,
    project,
    prog_path: str,
    bin_path: pathlib.Path,
    base_va: int,
    language_id: str = LANGUAGE_ID,
    compiler_id: str = COMPILER_ID,
) -> bool:
    from generic.stl import Pair
    from ghidra.app.util.importer import ProgramLoader
    from ghidra.app.util.opinion import BinaryLoader
    from ghidra.util.task import TaskMonitor
    from java.io import File
    from java.util import ArrayList

    if project.getProjectData().getFile(prog_path) is not None:
        print("[pass1] %s already present in %s" % (prog_path, project.getName()))
        return False
    folder_path = prog_path.rsplit("/", 1)[0] or "/"
    prog_name = prog_path.rsplit("/", 1)[-1]
    args = ArrayList()
    args.add(Pair("Base Address", "0x%X" % base_va))
    loaded = (
        ProgramLoader.builder()
        .source(File(str(bin_path)))
        .project(project)
        .projectFolderPath(folder_path)
        .name(prog_name)
        .language(language_id)
        .compiler(compiler_id)
        .loaders(BinaryLoader)
        .loaderArgs(args)
        .monitor(TaskMonitor.DUMMY)
        .load()
    )
    try:
        loaded.save(TaskMonitor.DUMMY)
        print("[pass1] imported %s -> %s at 0x%X" % (bin_path, prog_path, base_va))
        return True
    finally:
        loaded.close()


def apply_model_memory_layout(project, pyghidra, prog_path: str, base_va: int, model_id: str = DEFAULT_MODEL) -> dict:
    import ghidra_am34_memory_map as memmap
    from ghidra.util.task import TaskMonitor

    templates = []
    model_regions = MODEL_MEMORY_MAPS.get(model_id, {}).get("regions", [])
    if model_regions:
        templates = [
            (r["name"], int(r["start"], 0), int(r["size"], 0), bool(r["read"]), bool(r["write"]), bool(r["execute"]), bool(r["is_volatile"]))
            for r in model_regions
        ]
    elif project.getProjectData().getFile("/BOOT.BIN") is not None:
        with pyghidra.program_context(project, "/BOOT.BIN") as ref:
            templates = [
                (b.getName(), to_u32(b.getStart().getOffset()), int(b.getSize()), b.isRead(), b.isWrite(), b.isExecute(), b.isVolatile())
                for b in ref.getMemory().getBlocks()
                if not b.isInitialized() and b.getName() != "mmio"
            ]

    created = 0
    with pyghidra.program_context(project, prog_path) as prog:
        mem = prog.getMemory()
        space = prog.getAddressFactory().getDefaultAddressSpace()
        with tx(prog, "Panadiga SSOT memory layout (%s)" % model_id):
            ram = mem.getBlock("ram")
            if ram is not None and to_u32(ram.getStart().getOffset()) != base_va:
                mem.moveBlock(ram, space.getAddress(base_va), TaskMonitor.DUMMY)
            for bname, start, size, r, w, x, vol in templates:
                if _create_uninit_block_if_free(mem, space, bname, start, size, r, w, x, vol):
                    created += 1
            if str(prog.getLanguageID()).lower().startswith("mn10300"):
                memmap.setup_memory_map(prog)
        prog.save("Panadiga layout (%s)" % model_id, TaskMonitor.DUMMY)
    return {"blocks_created": created, "model": model_id, "base_va": "0x%08X" % base_va}


def import_loader(pyghidra, project) -> None:
    import_binary(pyghidra, project, "/" + LOADER_NAME, LOADER_SRC, LOADER_BASE, LANGUAGE_ID, COMPILER_ID)


def layout_loader(project, pyghidra) -> None:
    apply_model_memory_layout(project, pyghidra, "/" + LOADER_NAME, LOADER_BASE, DEFAULT_MODEL)
"#;
