//! TOC parsing, `MainList`/slot unpacking, and `decrypt_runtime()` manifest builder.

pub(super) const PY_DECRYPT_RUNTIME_PLAN: &str = r#"def _parse_toc_entries(dec: bytes, stride: int, raw_len: int) -> list:
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
