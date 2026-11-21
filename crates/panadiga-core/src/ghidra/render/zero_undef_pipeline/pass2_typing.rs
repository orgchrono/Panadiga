//! Pass 2 helpers (structured tables, CP932 mecha table, BGO tables, S-boxes, strings, pointer tables, padding)
//! for the generated `ghidra_zero_undef_pipeline.py` script.

pub(super) const PY_PASS2: &str = r#"# ---------------------------------------------------------------------------
# Pass 2 - typed data, literal pools, padding
# ---------------------------------------------------------------------------

def mark_auto(prog, va: int, kind: str, length: int, category: str = AUTO_DATA_CAT) -> None:
    prog.getBookmarkManager().setBookmark(va_addr(prog, va), AUTO_BM_TYPE, category, "%s len=%d" % (kind, length))


def set_eol(prog, va: int, text: str) -> None:
    from ghidra.program.model.listing import CodeUnit

    prog.getListing().setComment(va_addr(prog, va), CodeUnit.EOL_COMMENT, text)


def create_byte_array(prog, va: int, count: int) -> bool:
    from ghidra.program.model.data import ArrayDataType, ByteDataType

    listing = prog.getListing()
    done = 0
    while done < count:
        chunk = min(MAX_ARRAY_CHUNK, count - done)
        dt = ByteDataType.dataType if chunk == 1 else ArrayDataType(ByteDataType.dataType, chunk, 1)
        try:
            listing.createData(va_addr(prog, va + done), dt)
        except Exception:
            return False
        done += chunk
    return True


def _type_undefined_bytes(prog, va_lo: int, va_hi: int, kind: str, stats_key: str, stats: dict) -> int:
    total = 0
    for lo, hi in clip_to_undefined(prog, va_lo, va_hi):
        n = hi - lo + 1
        if create_byte_array(prog, lo, n):
            total += n
            stats[stats_key] += n
            mark_auto(prog, lo, kind, n)
    return total


def _type_undefined_array(prog, va: int, count: int, unit_dt, unit_size: int, kind: str, stats_key: str, stats: dict) -> int:
    from ghidra.program.model.data import ArrayDataType

    listing = prog.getListing()
    total = 0
    run_start = None
    run_len = 0

    def flush() -> None:
        nonlocal total, run_start, run_len
        if run_start is None or run_len <= 0:
            return
        dt = unit_dt if run_len == 1 else ArrayDataType(unit_dt, run_len, unit_size)
        try:
            listing.createData(va_addr(prog, run_start), dt)
            n = run_len * unit_size
            total += n
            stats[stats_key] += n
            mark_auto(prog, run_start, kind, n)
        except Exception:
            pass
        run_start = None
        run_len = 0

    for idx in range(count):
        slot = va + idx * unit_size
        if clip_to_undefined(prog, slot, slot + unit_size - 1) == [(slot, slot + unit_size - 1)]:
            if run_start is None:
                run_start = slot
            run_len += 1
        else:
            flush()
            _type_undefined_bytes(prog, slot, slot + unit_size - 1, kind + "_part", stats_key, stats)
    flush()
    return total


def _type_cli_table(prog, img: Image, stats: dict, seeds: set) -> None:
    from ghidra.program.model.data import ArrayDataType, ByteDataType, Pointer32DataType, StructureDataType, UnsignedIntegerDataType

    pos = img.raw.find(b"SLOGON\x00\x00")
    if pos < 0:
        return
    tbl_va = img.base + pos
    count = 0
    for idx in range(160):
        off = tbl_va + idx * CLI_ENTRY_SIZE
        if not img.contains(off + CLI_ENTRY_SIZE - 1):
            break
        name = img.slice(off, off + 8)
        nul = name.find(b"\x00")
        if nul <= 0 or any(c < 0x20 or c > 0x7E for c in name[:nul]):
            break
        count += 1
    if count < 8:
        return

    entry = StructureDataType("panadiga_drv_cli_entry", 0)
    entry.add(ArrayDataType(ByteDataType.dataType, 8, 1), 8, "name", "NUL-padded ASCII command name")
    entry.add(UnsignedIntegerDataType.dataType, 4, "policy_w0", "<8sIII> policy/arg descriptor")
    entry.add(UnsignedIntegerDataType.dataType, 4, "flags", "<8sIII> flags")
    ptr_dt = Pointer32DataType()
    entry.add(ptr_dt, 4, "handler", "<8sIII> command handler pointer")

    listing = prog.getListing()
    for idx in range(count):
        off = tbl_va + idx * CLI_ENTRY_SIZE
        handler = img.u32(off + 16)
        if img.contains(handler):
            seeds.add(handler)
        if clip_to_undefined(prog, off, off + CLI_ENTRY_SIZE - 1) == [(off, off + CLI_ENTRY_SIZE - 1)]:
            try:
                listing.createData(va_addr(prog, off), entry)
                stats["structured_table_bytes"] += CLI_ENTRY_SIZE
                continue
            except Exception:
                pass
        for sub_off, dt in ((8, UnsignedIntegerDataType.dataType), (12, UnsignedIntegerDataType.dataType), (16, ptr_dt)):
            if clip_to_undefined(prog, off + sub_off, off + sub_off + 3) == [(off + sub_off, off + sub_off + 3)]:
                try:
                    listing.createData(va_addr(prog, off + sub_off), dt)
                    stats["structured_table_bytes"] += 4
                except Exception:
                    pass
        _type_undefined_bytes(prog, off, off + 7, "drv_cli_name_tail", "structured_table_bytes", stats)
    mark_auto(prog, tbl_va, "drv_cli_table<8sIII>", count * CLI_ENTRY_SIZE)


def _type_atapi_tables(prog, img: Image, stats: dict, seeds: set) -> None:
    from ghidra.program.model.data import Pointer32DataType

    if img.base != 0x40000000:
        return
    raw = img.raw
    lut_off = None
    for off in range(0xC0000, min(0xF5000, img.size - 256)):
        if raw[off] == 0x1A and raw[off + 3] == 0x01 and raw[off + 0x12] == 0 and raw[off + 0x1B] == 0x0C and raw[off + 0x1E] == 0x0A:
            lut_off = off
            break
    if lut_off is None:
        return
    lut_va = img.base + lut_off
    max_idx = max(raw[lut_off : lut_off + 256])
    ptr_off = lut_off - 4 * (max_idx + 1)
    while ptr_off >= 4 and img.contains(img.u32(img.base + ptr_off - 4)):
        ptr_off -= 4
    ptr_va = img.base + ptr_off
    cnt = (lut_off - ptr_off) // 4
    for idx in range(cnt):
        tgt = img.u32(ptr_va + 4 * idx)
        if img.contains(tgt):
            seeds.add(tgt)
    _type_undefined_array(prog, ptr_va, cnt, Pointer32DataType(), 4, "atapi_handler_table", "pointer_table_bytes", stats)
    _type_undefined_bytes(prog, lut_va, lut_va + 255, "atapi_cdb_lut_256", "structured_table_bytes", stats)
    if lut_va == 0x400DEA64:
        _type_undefined_bytes(prog, 0x400DE35C, 0x400DE4B1, "atapi_sense_lut", "structured_table_bytes", stats)
        _type_undefined_bytes(prog, 0x400DE4B2, 0x400DE695, "atapi_attr3_table", "structured_table_bytes", stats)
        _type_undefined_bytes(prog, 0x400DE696, 0x400DE7DF, "atapi_attr2_table", "structured_table_bytes", stats)


def _valid_mecha_rec(rec: bytes) -> bool:
    code = struct.unpack_from("<H", rec, 0)[0]
    if code == 0xFFFF:
        return False
    body = rec[2:].split(b"\x00", 1)[0]
    if len(body) < 2 or any(b < 0x20 and b not in (9, 10, 13) for b in body):
        return False
    try:
        body.decode("cp932")
    except UnicodeDecodeError:
        return False
    return True


def _type_mecha_table(prog, img: Image, stats: dict) -> None:
    pos = img.raw.find(b"\xa7\x830.5mW")
    if pos < 0:
        return
    start, end = pos, pos + MECHA_RECORD_SIZE
    while start >= MECHA_RECORD_SIZE and _valid_mecha_rec(img.raw[start - MECHA_RECORD_SIZE : start]):
        start -= MECHA_RECORD_SIZE
    while end + MECHA_RECORD_SIZE <= img.size and _valid_mecha_rec(img.raw[end : end + MECHA_RECORD_SIZE]):
        end += MECHA_RECORD_SIZE
    count = (end - start) // MECHA_RECORD_SIZE
    for idx in range(count):
        rec_va = img.base + start + idx * MECHA_RECORD_SIZE
        _type_undefined_bytes(prog, rec_va, rec_va + MECHA_RECORD_SIZE - 1, "drv_mecha_error_rec", "structured_table_bytes", stats)
    set_eol(prog, img.base + start, "[Pass2] CP932 mecha error table (%d x 0x34-byte records)" % count)


def _type_bgo_and_jump_tables(prog, img: Image, stats: dict, seeds: set) -> None:
    from ghidra.program.model.data import Pointer32DataType, UnsignedIntegerDataType

    if img.base == 0x44000000 and img.contains(0x440337D0) and img.u32(0x440337D0) == 0x84130000:
        _type_undefined_array(prog, 0x440337A4, 70, UnsignedIntegerDataType.dataType, 4, "bgo_layout_table_u32", "structured_table_bytes", stats)
    if img.base == 0x44000000 and img.contains(0x44035F07):
        words = [img.u32(0x44035E98 + 4 * i) for i in range(28)]
        if all(img.contains(w) for w in words):
            seeds.update(words)
            _type_undefined_array(prog, 0x44035E98, 28, Pointer32DataType(), 4, "boot_switch_jump_table", "pointer_table_bytes", stats)
    if img.contains(0x5040EA30 + 15):
        words = [img.u32(0x5040EA30 + 4 * i) for i in range(4)]
        if all(img.contains(w) for w in words):
            cnt = 4
            while img.contains(0x5040EA30 + 4 * cnt + 3) and img.contains(img.u32(0x5040EA30 + 4 * cnt)):
                cnt += 1
            for i in range(cnt):
                seeds.add(img.u32(0x5040EA30 + 4 * i))
            _type_undefined_array(prog, 0x5040EA30, cnt, Pointer32DataType(), 4, "prog061_jump_table", "pointer_table_bytes", stats)


def _type_sboxes(prog, img: Image, stats: dict) -> None:
    pos = img.raw.find(S_BOX_SIGNATURE)
    while pos != -1:
        va = img.base + pos
        if pos + 256 <= img.size:
            typed = _type_undefined_bytes(prog, va, va + 255, "feistel_sbox", "structured_table_bytes", stats)
            if typed:
                set_eol(prog, va, "[Pass2] Feistel S-Box (256 bytes)")
        pos = img.raw.find(S_BOX_SIGNATURE, pos + 1)


def _looks_textual(blob: bytes, strict: bool = False) -> bool:
    body = blob[:-1]
    if strict:
        if body.startswith(KNOWN_TEXT_PREFIXES):
            return len(body) >= 8 and all(b in ALLOWED_TEXT_BYTES for b in body)
        if len(body) < 10 or any(b not in ALLOWED_TEXT_BYTES for b in body):
            return False
        alnum = sum(1 for c in body if chr(c).isalnum())
        return alnum * 10 >= 7 * len(body)
    letters = sum(1 for c in body if chr(c).isalpha())
    return len(body) >= 8 or letters * 2 >= len(body)


def _type_strings(prog, img: Image, ranges, stats: dict, conservative_text: bool = False) -> None:
    from ghidra.program.model.data import StringDataType

    listing = prog.getListing()
    for lo, hi in ranges:
        chunk = img.slice(lo, hi + 1)
        taken = []
        for m in STRING_RE.finditer(chunk):
            blob = m.group(0)
            start = lo + m.start()
            in_code = start < img.code_hi
            if not _looks_textual(blob, strict=(conservative_text and in_code)):
                continue
            if start > lo and chunk[m.start() - 1] != 0:
                continue
            try:
                listing.createData(va_addr(prog, start), StringDataType.dataType, len(blob))
            except Exception:
                continue
            stats["string_bytes"] += len(blob)
            stats["strings"] += 1
            mark_auto(prog, start, "ascii_string", len(blob))
            taken.append((m.start(), m.end()))
        if conservative_text and hi < img.code_hi:
            continue
        for m in SJIS_RE.finditer(chunk):
            if any(a < m.end() and m.start() < b for a, b in taken):
                continue
            start = lo + m.start()
            if conservative_text and start < img.code_hi:
                continue
            blob = m.group(0)
            try:
                text = blob[:-1].decode("cp932")
            except UnicodeDecodeError:
                continue
            if all(ord(c) < 0x80 for c in text) or chunk[m.start() - 1 : m.start()] not in (b"", b"\x00"):
                continue
            if create_byte_array(prog, start, len(blob)):
                stats["string_bytes"] += len(blob)
                stats["sjis_strings"] += 1
                mark_auto(prog, start, "shift_jis_string", len(blob))
                set_eol(prog, start, "[Pass2] cp932: " + text[:60])


def _type_pointer_runs(prog, img: Image, ranges, stats: dict, seeds: set) -> None:
    from ghidra.program.model.data import ArrayDataType, Pointer32DataType

    listing = prog.getListing()
    ptr_dt = Pointer32DataType()

    def flush(run) -> None:
        if len(run) < MIN_PTR_RUN or len({t for _, t in run}) < 2:
            return
        start = run[0][0]
        try:
            listing.createData(va_addr(prog, start), ArrayDataType(ptr_dt, len(run), 4))
        except Exception:
            return
        stats["pointer_table_bytes"] += 4 * len(run)
        stats["pointer_tables"] += 1
        mark_auto(prog, start, "pointer_table", 4 * len(run))
        seeds.update(t for _, t in run)

    for lo, hi in ranges:
        run = []
        va = lo + ((4 - (lo - img.base) % 4) % 4)
        while va + 3 <= hi:
            target = img.u32(va)
            if img.contains(target) and target != va:
                run.append((va, target))
            else:
                flush(run)
                run = []
            va += 4
        flush(run)


def _type_padding(prog, img: Image, ranges, stats: dict, min_run: int = MIN_PAD_RUN) -> None:
    for lo, hi in ranges:
        chunk = img.slice(lo, hi + 1)
        for pad in PAD_BYTES:
            pat = re.compile(re.escape(bytes([pad])) + b"{%d,}" % min_run)
            for m in pat.finditer(chunk):
                start = lo + m.start()
                length = m.end() - m.start()
                if create_byte_array(prog, start, length):
                    stats["padding_bytes"] += length
                    stats["padding_runs"] += 1
                    mark_auto(prog, start, "padding_0x%02X" % pad, length)
    for pad in PAD_BYTES:
        n = 0
        while n < img.size and img.raw[img.size - 1 - n] == pad:
            n += 1
        if n >= 8:
            _type_undefined_bytes(prog, img.end - n, img.end - 1, "tail_padding_0x%02X" % pad, "padding_bytes", stats)


def pass2_program(prog, img: Image) -> dict:
    stats = {
        "structured_table_bytes": 0,
        "string_bytes": 0,
        "strings": 0,
        "sjis_strings": 0,
        "pointer_table_bytes": 0,
        "pointer_tables": 0,
        "padding_bytes": 0,
        "padding_runs": 0,
    }
    seeds: set = set()
    with tx(prog, "Panadiga pass2 typed data"):
        _type_cli_table(prog, img, stats, seeds)
        _type_atapi_tables(prog, img, stats, seeds)
        _type_mecha_table(prog, img, stats)
        _type_bgo_and_jump_tables(prog, img, stats, seeds)
        _type_sboxes(prog, img, stats)
        _type_strings(prog, img, undefined_ranges(prog), stats, conservative_text=True)
        _type_pointer_runs(prog, img, undefined_ranges(prog), stats, seeds)
        _type_padding(prog, img, undefined_ranges(prog), stats)
    stats["pointer_seeds"] = len(seeds)
    return {"stats": stats, "seeds": sorted(seeds)}
"#;
