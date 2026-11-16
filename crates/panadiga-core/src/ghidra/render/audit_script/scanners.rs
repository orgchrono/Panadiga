//! Python ASCII keyword, LE32 MMIO literal, Ghidra memory/bookmark, and XREF scanners.

pub(super) const PYTHON_SCANNERS: &str = r#"def scan_ascii_keywords(raw_bytes, base_addr, keywords=AUDIT_KEYWORDS, max_hits_per_kw=8):
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
"#;
