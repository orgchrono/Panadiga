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

"#;
