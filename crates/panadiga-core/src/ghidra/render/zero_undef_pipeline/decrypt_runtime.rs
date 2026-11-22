//! Full pure-Python Matsushita decryption, `.FRM`/`MainList`/`EXTRHEAD` unpacking,
//! and dynamic bench runtime anchor discovery for `ghidra_zero_undef_pipeline.py`.

pub(super) const PY_DECRYPT_RUNTIME: &str = r#"# ---------------------------------------------------------------------------
# Universal Panasonic Decrypt & Dynamic Bench Runtime (`--frm` / `decrypt-runtime`)
# ---------------------------------------------------------------------------

PANA_FEISTEL_SBOX = tuple(bytearray.fromhex(
    "0001bb9c3f5ec73e36a49293389b8d1a3c84f72acb3d70ad30a2c1950339a50e"
    "e6fb091eb9c424575fc35dfa6056fded81f34aff85dbcda0d09d8b264c500c1d"
    "7d77de7a0a0b73e15476e8c228059899294bee87425965b81fdf235863691c79"
    "64c56f133bce08a7b212207fe289212b41497c82bef2438e86029ed8678fa3e7"
    "bcf866224de5bad2daea6125b0c07ba64896dddc7190555b4f4e6dd11134aa1b"
    "f41780b7cc8cd9378a2c94167240ef9ac847f6fc2eb32fbdcfa9151018ec6a6e"
    "c9915378a83aaf0de0973251b1f5272d358814e3fe06d7d6aba1d5e445449f0f"
    "5a83ebca0704b4b652aed4333175d36c62bf6bf9ac46f05c197e68b5f174c6e9"
))

PANA_DVD_KEY_CATALOG = (
    ("old_df72", "df7215c7427fd010"),
    ("le2011_08e0", "08e03d859af9f3ee"),
    ("y2011_af4d", "af4d5b16c298c16d"),
    ("y2012_9407", "9407311b7561f97e"),
    ("y2012_5d88", "5d88719745a4efff"),
    ("y2013_38bb", "38bb0c17d362701f"),
    ("y2013_ec23", "ec238cd791456646"),
    ("hualu_0a1e", "0a1ee6c5bdc4933f"),
    ("y2014_8f8c", "8f8ca88482e43d9b"),
    ("y2014_21fc", "21fca17361e66b4e"),
    ("y2016_fdfe", "fdfe0bee4c90fa2c"),
    ("y2018_06c9", "06c943f3b997f7e0"),
)

_TOC_MAGIC_PREFIXES = (b"PROG", b"DRV ", b"MAIN", b"STB ")


def _ror8(val: int, bits: int) -> int:
    return ((val >> bits) | (val << (8 - bits))) & 0xFF


def _rol8(val: int, bits: int) -> int:
    return ((val << bits) | (val >> (8 - bits))) & 0xFF


def _feistel_f_round(b0: int, b1: int, b2: int, b3: int, k0: int, k1: int, k2: int, k3: int):
    v3, v4 = k1 ^ b1, k0 ^ b0
    x3, x2 = k3 ^ b3, k2 ^ b2
    v1, v5 = x3 >> 2, _ror8(x3, 2)
    v2, v6 = x2 >> 5, _ror8(x2, 5)
    o3 = ((v6 >> 1) | (v1 << 7)) & 0xFF
    o2 = ((v3 & 0x7F) | (v2 << 7)) & 0xFF
    o1 = ((v4 >> 1) | (v3 & 0x80)) & 0xFF
    o0 = ((v5 >> 1) | (v4 << 7)) & 0xFF
    s = PANA_FEISTEL_SBOX[v4 ^ _rol8(v3, 1) ^ v6 ^ v5]
    return (
        (o0 + _rol8(s, 3)) & 0xFF,
        (o1 + _rol8(s, 2)) & 0xFF,
        (o2 + _rol8(s, 1)) & 0xFF,
        (o3 + s) & 0xFF,
    )


def derive_round_key_8b(key_8b: bytes) -> tuple:
    rk = bytearray(32)
    rk[:7] = key_8b[:7]
    rk[1] ^= key_8b[7]
    rk[4] ^= key_8b[7]
    state = (0x35, 0x7D, 0xFB, 0x9A)
    for rnd in range(16):
        off = (rnd & 7) * 4
        state = _feistel_f_round(rk[off], rk[off + 1], rk[off + 2], rk[off + 3], *state)
        rk[off:off + 4] = state
    return tuple(rk)


_PRECOMPUTED_ROUND_KEYS = tuple((name, derive_round_key_8b(bytes.fromhex(hx))) for name, hx in PANA_DVD_KEY_CATALOG)


def _feistel_block(blk: bytes, rk: tuple, decrypt: bool = True) -> bytes:
    l0, l1, l2, l3, r0, r1, r2, r3 = blk[0], blk[1], blk[2], blk[3], blk[4], blk[5], blk[6], blk[7]
    for step in range(16):
        rnd = (15 - step) if decrypt else step
        off = (rnd & 7) * 4
        f0, f1, f2, f3 = _feistel_f_round(l0, l1, l2, l3, rk[off], rk[off + 1], rk[off + 2], rk[off + 3])
        l0, l1, l2, l3, r0, r1, r2, r3 = f0 ^ r0, f1 ^ r1, f2 ^ r2, f3 ^ r3, l0, l1, l2, l3
    return bytes((r0, r1, r2, r3, l0, l1, l2, l3))


def feistel_decrypt_buf(raw: bytes, rk: tuple) -> bytes:
    aligned = len(raw) & ~7
    out = bytearray(raw)
    for off in range(0, aligned, 8):
        out[off:off + 8] = _feistel_block(out[off:off + 8], rk, True)
    return bytes(out)


def feistel_crypt_edges(raw: bytes, span: int, rk: tuple, decrypt: bool = True) -> bytes:
    aligned_span = span & ~7
    aligned_len = len(raw) & ~7
    if aligned_span == 0 or aligned_len == 0:
        return bytes(raw)
    out = bytearray(raw)
    ranges = ((0, aligned_len),) if aligned_len <= 2 * aligned_span else ((0, aligned_span), (aligned_len - aligned_span, aligned_len))
    for lo, hi in ranges:
        for off in range(lo, hi, 8):
            out[off:off + 8] = _feistel_block(out[off:off + 8], rk, decrypt)
    return bytes(out)


def strip_panasonic_outer_header(raw: bytes) -> bytes:
    if len(raw) >= 48 and raw[:12] == b"PANASONIC\x00\x00\x00" and raw[0x2C:0x30] == b"\x00\x00\x00\x00":
        return raw[48:]
    return raw


def adler32_panasonic_strided(data: bytes, step: int = 16) -> int:
    stride = max(1, step)
    s1, s2 = 1, 0
    for b in data[::stride]:
        s1 = (s1 + b) % 65521
        s2 = (s2 + s1) % 65521
    return s1 | (s2 << 16)


def network_decipher(data: bytes) -> bytes:
    out = bytearray(data)
    counter = 0
    for idx, b in enumerate(out):
        if (idx & 0xFF) == 0:
            counter = 904
        out[idx] = b ^ ((counter >> 8) & 0xFF)
        counter = (counter + b + 38563) & 0xFFFFFFFF
    return bytes(out)


def lzss_decompress_pana(data: bytes, expected_size: int) -> bytes:
    win = bytearray(0x1000)
    dst = bytearray()
    src_i, win_pos, flags, end = 0, 0x0FEE, 0, len(data)
    while src_i < end:
        flags >>= 1
        if (flags & 0x100) == 0:
            if src_i >= end:
                break
            flags = data[src_i] | 0xFF00
            src_i += 1
        if (flags & 1) == 0:
            if src_i + 1 >= end:
                break
            b1, b2 = data[src_i], data[src_i + 1]
            src_i += 2
            offset = b1 | ((b2 & 0xF0) << 4)
            for _ in range((b2 & 0x0F) + 3):
                c = win[offset]
                dst.append(c)
                win[win_pos] = c
                win_pos = (win_pos + 1) & 0x0FFF
                offset = (offset + 1) & 0x0FFF
                if expected_size > 0 and len(dst) >= expected_size:
                    return bytes(dst)
        else:
            c = data[src_i]
            src_i += 1
            dst.append(c)
            win[win_pos] = c
            win_pos = (win_pos + 1) & 0x0FFF
            if expected_size > 0 and len(dst) >= expected_size:
                return bytes(dst)
    return bytes(dst)


def detect_feistel_key(raw: bytes):
    toc_bytes = strip_panasonic_outer_header(raw)
    scan_limit = min(len(toc_bytes), 0x2000) & ~7
    for name, rk in _PRECOMPUTED_ROUND_KEYS:
        for base in range(0, min(scan_limit, 0x80), 0x40):
            for off in (base, base + 48, base + 64):
                if off + 8 <= len(toc_bytes):
                    dec = _feistel_block(toc_bytes[off:off + 8], rk, True)
                    if dec.startswith(_TOC_MAGIC_PREFIXES):
                        return name, rk
    return None, None


def _decompress_payload(payload: bytes, algo: int, expected_size: int) -> bytes:
    import gzip
    import lzma
    import zlib

    if algo == 0:
        return zlib.decompress(payload)
    if algo == 1:
        return gzip.decompress(payload)
    if algo == 2:
        return lzss_decompress_pana(payload, expected_size)
    if algo == 3:
        for fmt in (lzma.FORMAT_AUTO, lzma.FORMAT_ALONE, lzma.FORMAT_RAW):
            try:
                return lzma.decompress(payload, format=fmt)
            except Exception:
                continue
        raise ValueError("LZMA decompression failed")
    raise ValueError("Unsupported EXTRHEAD compression algo %d" % algo)


def unpack_extrhead(raw: bytes, preferred_rk: tuple | None = None, depth: int = 0):
    if len(raw) < 0x40:
        return None
    is_plain = raw[:8] == b"EXTRHEAD"
    rk, double_crypt = (preferred_rk, False) if is_plain else (None, False)
    if not is_plain:
        candidates = [(preferred_rk, "preferred")] if preferred_rk is not None else [(k, n) for n, k in _PRECOMPUTED_ROUND_KEYS]
        for cand_rk, _ in candidates:
            d1 = _feistel_block(raw[:8], cand_rk, True)
            if d1 == b"EXTRHEAD":
                rk, double_crypt = cand_rk, False
                break
            d2 = _feistel_block(d1, cand_rk, True)
            if d2 == b"EXTRHEAD":
                rk, double_crypt = cand_rk, True
                break
        if rk is None:
            return None
    hdr = raw[:0x40] if is_plain else feistel_decrypt_buf(raw[:0x40], rk)
    if double_crypt and rk is not None:
        hdr = feistel_decrypt_buf(hdr, rk)
    if hdr[:8] != b"EXTRHEAD":
        return None
    tag = hdr[0x08:0x0C].decode("ascii", errors="replace").strip()
    flag, algo = struct.unpack_from("<HH", hdr, 0x0C)
    dst_len, dst_addr, src_len = struct.unpack_from("<III", hdr, 0x10)
    footer_off = struct.unpack_from("<I", hdr, 0x20)[0]
    hdr_chk, adler_step = struct.unpack_from("<II", hdr, 0x28)
    build_name = hdr[0x30:0x40].split(b"\x00", 1)[0].decode("ascii", errors="replace").strip()
    span = (footer_off + 8) if footer_off >= 0x40 else (0x40 + src_len + 8)
    aligned_end = min(len(raw) & ~7, (span + 7) & ~7)
    work = raw[:aligned_end] if aligned_end >= 0x40 else raw
    if double_crypt and rk is not None:
        work = feistel_decrypt_buf(work, rk)
    if not is_plain and rk is not None:
        if tag == "DRV":
            drv_span = min(0x2800, len(work) & ~7)
            work = feistel_decrypt_buf(work[:drv_span], rk) + work[drv_span:]
        elif tag == "MAIN":
            work = feistel_crypt_edges(work, 0x19000, rk, True)
    comp = work[0x40:0x40 + src_len]
    decomp = _decompress_payload(comp, algo, dst_len)
    chk_ok = adler32_panasonic_strided(decomp, adler_step or 16) == hdr_chk
    if depth < 4 and len(decomp) >= 0x40 and decomp[:8] == b"EXTRHEAD":
        nested = unpack_extrhead(decomp, None, depth + 1)
        if nested is not None:
            nested_dec, nested_meta = nested
            nested_meta["checksum_valid"] = chk_ok and nested_meta["checksum_valid"]
            return nested_dec, nested_meta
    meta = {
        "tag": tag, "flag": flag, "algo": algo, "dst_len": dst_len,
        "dst_addr": "0x%08X" % dst_addr, "build_name": build_name, "checksum_valid": chk_ok,
    }
    return decomp, meta


def _parse_toc_entries(dec: bytes, stride: int, raw_len: int) -> list:
    entries = []
    sum_off = stride - 2
    for idx in range(len(dec) // stride):
        slot = dec[idx * stride:(idx + 1) * stride]
        name = slot[:0x0A].split(b"\x00", 1)[0].decode("ascii", errors="replace").strip()
        if not name:
            continue
        mod_off = struct.unpack_from("<I", slot, 0x0C)[0]
        mod_sz = struct.unpack_from("<I", slot, 0x20)[0]
        if mod_off >= raw_len and stride == 48 and idx == 0:
            alt_off = struct.unpack_from("<I", slot, 0x08)[0]
            if alt_off == 0x1800 and alt_off + mod_sz <= raw_len:
                mod_off = alt_off
        if mod_sz == 0 or mod_off + mod_sz > raw_len:
            continue
        chk_u16 = struct.unpack_from("<H", slot, sum_off)[0]
        sum_ok = (sum(slot[:sum_off]) & 0xFFFF) == chk_u16
        if not sum_ok and stride == 48:
            chk_u32 = struct.unpack_from("<I", slot, 0x2C)[0]
            sum_ok = adler32_panasonic_strided(slot[:0x2C], 1) == chk_u32
        if sum_ok:
            entries.append({"name": name, "offset": mod_off, "size": mod_sz})
    return entries


def _try_main_list(raw: bytes, rk: tuple | None):
    if len(raw) < 20:
        return None
    chk, _unk, lsz, segsz, _ctype = struct.unpack_from("<5I", raw, 0)
    if lsz < 20 or lsz > len(raw) or (lsz - 20) % 8 != 0:
        return None
    hdr_tail = raw[4:lsz]
    if adler32_panasonic_strided(hdr_tail, 1) != chk and (sum(hdr_tail) & 0xFFFFFFFF) != chk:
        return None
    crypt_span = 0x2800 if segsz == 0x2000000 else 0x1400
    cursor, chunks, last_meta = lsz, [], None
    for i in range((lsz - 20) // 8):
        ssz, _ssum = struct.unpack_from("<2I", raw, 20 + i * 8)
        seg = raw[cursor:cursor + ssz]
        if rk is not None and len(seg) >= crypt_span:
            seg = feistel_crypt_edges(seg, crypt_span, rk, True)
        unpacked = unpack_extrhead(seg, rk)
        if unpacked is None:
            return None
        chunks.append(unpacked[0])
        last_meta = unpacked[1]
        cursor += ssz
    return b"".join(chunks), last_meta


def _try_stb_zlib(buf: bytes):
    import zlib

    if len(buf) <= 264 or not buf.startswith(b"CRC:"):
        return None
    hdr_txt = buf[:256].decode("ascii", errors="replace")
    model = None
    for part in hdr_txt.split("\x00"):
        if part.startswith("Model:"):
            model = part[6:].strip()
            break
    payload = buf[256:]
    if "zlib:" in hdr_txt and payload[:1] == b"\x78":
        try:
            dec = zlib.decompress(payload)
        except Exception:
            dec = b""
        if dec:
            return dec, {
                "tag": "STB", "flag": 0, "algo": 0, "dst_len": len(dec),
                "dst_addr": "0x80000400", "build_name": model or "", "checksum_valid": True,
            }
    return payload, {
        "tag": "STB", "flag": 0, "algo": None, "dst_len": len(payload),
        "dst_addr": "0x80000400", "build_name": model or "", "checksum_valid": True,
    }


def parse_frm_toc(raw: bytes, total_len: int | None = None):
    raw_len = len(raw) if total_len is None else total_len
    kname, rk = detect_feistel_key(raw)
    if rk is not None:
        toc_bytes = strip_panasonic_outer_header(raw)
        dec = feistel_decrypt_buf(toc_bytes[:min(len(toc_bytes), 0x2000) & ~7], rk)
        for stride, label in ((64, "Dvd64B"), (48, "Dvd48B")):
            entries = _parse_toc_entries(dec, stride, raw_len)
            if entries:
                return "FRM_FEISTEL_%s" % label, kname, rk, entries
    for stride, label in ((64, "Sd64B"), (48, "Sd48B")):
        entries = _parse_toc_entries(raw[:min(len(raw), 0x2000)], stride, raw_len)
        if entries:
            return "FRM_SD_%s" % label, None, None, entries
    return "RAW_OR_STANDALONE", None, None, []


def scan_dynamic_bench_anchors(raw: bytes, base_va: int, stage_name: str, model_id: str, region_code: str | None) -> dict:
    bgo_tbl, bgo_base = None, None
    for sig_val in (0x84130000, 0x84200000):
        pat = struct.pack("<II", sig_val, 0x84400000)
        pos = raw.find(pat)
        if pos >= 44 and pos % 4 == 0 and pos + 236 <= len(raw):
            if struct.unpack_from("<I", raw, pos - 12)[0] == 0x10000 and struct.unpack_from("<I", raw, pos + 8)[0] == 0x10000:
                bgo_tbl = "0x%08X" % (base_va + pos - 44)
                bgo_base = "0x%08X" % sig_val
                break
    cli_va = None
    cli_pos = raw.find(b"SLOGON\x00\x00")
    if cli_pos >= 0:
        cli_va = "0x%08X" % (base_va + cli_pos)
    sbox_vas = []
    pos = raw.find(S_BOX_SIGNATURE)
    while pos != -1:
        if pos + 256 <= len(raw):
            sbox_vas.append("0x%08X" % (base_va + pos))
        pos = raw.find(S_BOX_SIGNATURE, pos + 1)
    reg_up = (region_code or "").strip().upper()
    tv_std = "PAL-M" if reg_up in ("LB", "LBS") else ("PAL/SECAM" if reg_up == "EE" else ("NTSC" if reg_up in ("P", "VP", "PC", "VPC") else "PAL"))
    vmodesel = "0x0002" if tv_std == "PAL-M" else ("0x0000" if tv_std.startswith("NTSC") else "0x0001")
    return {
        "stage_key": stage_name,
        "base_va": "0x%08X" % base_va,
        "size_bytes": len(raw),
        "bgo_layout_table_va": bgo_tbl,
        "bgo_partition_base": bgo_base,
        "cli_table_va": cli_va,
        "feistel_sbox_vas": sbox_vas,
        "region_interpolation": {"model_id": model_id, "region_code": reg_up or "AUTO", "tv_standard": tv_std, "aiof_vmodesel": vmodesel},
    }


def decrypt_and_prepare_runtime_stages(input_path: pathlib.Path, out_dir: pathlib.Path, model_id: str, region_code: str | None = None):
    raw = input_path.read_bytes()
    out_dir.mkdir(parents=True, exist_ok=True)
    fmt, kname, rk, entries = parse_frm_toc(raw)
    manifest_stages = []
    ghidra_targets = []
    items = []
    for e in entries:
        off, sz = e["offset"], e["size"]
        pad_end = min(len(raw), off + ((sz + 7) & ~7)) if rk is not None else (off + sz)
        items.append((e["name"], raw[off:pad_end], sz, rk is not None))
    if not items:
        up = unpack_extrhead(raw, None)
        items = [(input_path.name, up[0], len(up[0]), False)] if up else [(input_path.name, raw, len(raw), False)]
    for idx, (slot_name, slot_bytes, orig_size, outer_feistel) in enumerate(items):
        unpacked_bytes, meta = slot_bytes[:orig_size], None
        ml = _try_main_list(unpacked_bytes, rk)
        if ml is not None:
            unpacked_bytes, meta = ml
        else:
            dec_slot = feistel_decrypt_buf(slot_bytes, rk)[:orig_size] if (outer_feistel and rk is not None) else unpacked_bytes
            ml2 = _try_main_list(dec_slot, rk)
            if ml2 is not None:
                unpacked_bytes, meta = ml2
            else:
                ext = unpack_extrhead(dec_slot, rk) if (not slot_name.upper().startswith("PROG") or dec_slot[:8] == b"EXTRHEAD") else None
                if ext is not None:
                    unpacked_bytes, meta = ext
                else:
                    stb = _try_stb_zlib(dec_slot)
                    if stb is not None:
                        unpacked_bytes, meta = stb
                    else:
                        unpacked_bytes = dec_slot
        slug = re.sub(r"[^A-Za-z0-9]+", "_", slot_name.strip()).strip("_") or ("stage_%d" % idx)
        dst_file = out_dir / ("%02d_%s.bin" % (idx, slug))
        dst_file.write_bytes(unpacked_bytes)
        raw_dst = meta["dst_addr"] if meta else None
        base_override = raw_dst if (raw_dst and int(raw_dst, 0) not in (0, 0x90400000, 0x90202000)) else None
        base_va = infer_base_va(dst_file, model_id, base_override)
        anchors = scan_dynamic_bench_anchors(unpacked_bytes, base_va, slot_name, model_id, region_code)
        anchors["unpacked_path"] = str(dst_file)
        if meta:
            anchors["extrhead"] = meta
        manifest_stages.append(anchors)
        ghidra_targets.append((slug, "/" + dst_file.name, dst_file, base_va))
    manifest = {"input": str(input_path), "container_format": fmt, "feistel_key": kname, "stages": manifest_stages}
    write_json(out_dir, "bench_runtime_manifest.json", manifest)
    return manifest, ghidra_targets
"#;
