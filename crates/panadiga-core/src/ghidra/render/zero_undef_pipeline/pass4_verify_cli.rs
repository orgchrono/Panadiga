//! Pass 4 normalization, residual typing, zero-`??` verification gate, and CLI entrypoint
//! for `ghidra_zero_undef_pipeline.py`.

pub(super) const PY_PASS4_CLI: &str = r#"# ---------------------------------------------------------------------------
# Pass 4 - normalization, residual typing, verification & generic CLI
# ---------------------------------------------------------------------------

def ensure_volatile_windows(prog) -> dict:
    mem = prog.getMemory()
    space = prog.getAddressFactory().getDefaultAddressSpace()
    created = 0
    for name, lo, hi in MMIO_WINDOWS:
        if _create_uninit_block_if_free(mem, space, name, lo, hi - lo + 1, True, True, False, True):
            created += 1
    marked = 0
    for blk in mem.getBlocks():
        lo, hi = to_u32(blk.getStart().getOffset()), to_u32(blk.getEnd().getOffset())
        if not blk.isInitialized() and any(lo <= whi and wlo <= hi for _n, wlo, whi in MMIO_WINDOWS):
            if not blk.isVolatile():
                blk.setVolatile(True)
                marked += 1
    return {"blocks_created": created, "blocks_marked_volatile": marked}


def type_residual(prog, img: Image) -> dict:
    typed_ranges = 0
    pad_bytes = 0
    raw_bytes = 0
    for lo, hi in undefined_ranges(prog):
        length = hi - lo + 1
        if length <= MAX_TINY_PAD and len(set(img.slice(lo, hi + 1))) == 1:
            kind, cat = "alignment_pad_0x%02X" % img.raw[lo - img.base], AUTO_DATA_CAT
            pad_bytes += length
        else:
            kind, cat = "unclassified_raw", RESIDUAL_CAT
            raw_bytes += length
        if create_byte_array(prog, lo, length):
            typed_ranges += 1
            mark_auto(prog, lo, kind, length, cat)
    return {"residual_ranges": typed_ranges, "tiny_pad_bytes": pad_bytes, "residual_unclassified_bytes": raw_bytes}


def decompile_tainted(prog) -> dict:
    from ghidra.app.decompiler import DecompInterface
    from ghidra.util.task import ConsoleTaskMonitor

    results = {}
    di = DecompInterface()
    di.openProgram(prog)
    try:
        for va in TAINTED_FUNCTIONS:
            fn = prog.getFunctionManager().getFunctionContaining(va_addr(prog, va))
            if fn is None:
                continue
            res = di.decompileFunction(fn, 120, ConsoleTaskMonitor())
            text = res.getDecompiledFunction().getC() if res.decompileCompleted() else ""
            results["0x%08X" % va] = {
                "function": fn.getName(),
                "completed": bool(res.decompileCompleted()),
                "bad_tokens": [t for t in DECOMP_BAD_TOKENS if t in text],
                "lines": text.count("\n"),
            }
    finally:
        di.dispose()
    return results


def spot_check_opcodes(prog, name: str) -> dict:
    listing = prog.getListing()
    out = {}
    for va in SPOT_CHECK_OPCODES.get(name, ()):
        ins = listing.getInstructionContaining(va_addr(prog, va))
        if ins is None:
            out["0x%08X" % va] = None
        else:
            ops = ",".join(ins.getDefaultOperandRepresentation(i) for i in range(ins.getNumOperands()))
            out["0x%08X" % va] = "%s @ 0x%08X: %s %s" % (ins.getMnemonicString(), to_u32(ins.getAddress().getOffset()), ins.getMnemonicString(), ops)
    return out


def pass4_program(prog, img: Image, name: str, pass2_seeds: list, switches: bool) -> dict:
    import ghidra_auto_unbrick_audit as audit
    import ghidra_mn103_fix_decompilation as fix
    from ghidra.util.task import TaskMonitor

    WORK_DIR.mkdir(parents=True, exist_ok=True)
    report = {}
    is_large = img.size > LARGE_IMAGE_BYTES
    with tx(prog, "Panadiga pass4 decompilation normalization"):
        fix_res = fix.run_mn103_decompilation_fixes(prog, apply_purge=True, do_switches=(switches and not is_large), resolve_traps=True)
        if switches and is_large:
            targets = fix._find_computed_jump_functions(prog)[:60]
            fix_res["switches"] = fix.recover_mn103_switches(prog, targets=targets, max_passes=1, timeout_secs=15, monitor_obj=TaskMonitor.DUMMY)
            fix_res["switches"]["bounded_large_image"] = True
        report["fix_decompilation"] = fix_res
        report["volatile"] = ensure_volatile_windows(prog)
        report["unbrick_audit"] = {"stage": audit.run_unbrick_audit(prog, str(WORK_DIR / ("ghidra_audit_%s.json" % name))).get("detected_stage")}
    with tx(prog, "Panadiga pass4 finalization"):
        report["redisassemble"] = redisassemble_bad_sites(prog)
        pre = error_snapshot(prog)
        report["strong_prologues"] = _seed_strong_prologues_in_gaps(prog, img, pre)
        report["seeding"] = _seed_dict(prog, img, _collect_pointer_seeds(prog, img, name, pass2_seeds), pre)
        report["sweep"] = sweep_gaps(prog, img, pre)
        report["redisassemble_final"] = redisassemble_bad_sites(prog)
        report["trap4"] = annotate_trap4(prog, img, name)
        late = {
            "structured_table_bytes": 0, "string_bytes": 0, "strings": 0, "sjis_strings": 0,
            "pointer_table_bytes": 0, "pointer_tables": 0, "padding_bytes": 0, "padding_runs": 0,
        }
        _type_strings(prog, img, undefined_ranges(prog), late)
        _type_padding(prog, img, undefined_ranges(prog), late)
        report["late_data"] = late
        report["residual"] = type_residual(prog, img)
    report["decompile_tainted"] = decompile_tainted(prog)
    report["spot_checks"] = spot_check_opcodes(prog, name)
    return report


def bookmark_bytes(prog, category: str) -> dict:
    totals = {}
    for bm in _iter_bookmarks(prog.getBookmarkManager(), AUTO_BM_TYPE):
        if bm.getCategory() != category:
            continue
        kind, _, tail = str(bm.getComment()).partition(" len=")
        entry = totals.setdefault(kind.split("_0x")[0], [0, 0])
        entry[0] += 1
        entry[1] += int(tail) if tail.isdigit() else 0
    return {k: {"items": v[0], "bytes": v[1]} for k, v in sorted(totals.items())}


def verify_program(prog, name: str) -> dict:
    rep = measure(prog)
    rep["instruction_bytes"] = instruction_bytes(prog)
    rep["auto_data"] = bookmark_bytes(prog, AUTO_DATA_CAT)
    rep["residual_unclassified"] = bookmark_bytes(prog, RESIDUAL_CAT)
    bm_mgr = prog.getBookmarkManager()
    rep["trap4_aliases"] = sum(1 for bm in _iter_bookmarks(bm_mgr, AUTO_BM_TYPE) if bm.getCategory() == TRAP4_CAT)
    rep["external_flows"] = sum(1 for bm in _iter_bookmarks(bm_mgr, AUTO_BM_TYPE) if bm.getCategory() == EXTERNAL_CAT)
    rep["decompile_tainted"] = decompile_tainted(prog)
    rep["spot_checks"] = spot_check_opcodes(prog, name)
    rep["gate_undefined_code_bytes_zero"] = rep["undefined_code_bytes"] == 0
    rep["gate_bad_bookmarks_zero"] = rep["bad_instruction_bookmarks"] == 0
    rep["gate_mn_todo_in_functions_zero"] = rep["mn_todo_in_functions"] == 0
    rep["gate_tainted_clean"] = all(not v["bad_tokens"] and v["completed"] for v in rep["decompile_tainted"].values())
    return rep


def load_seed_file(name: str) -> list:
    for base_dir in (WORK_DIR, REPORT_DIR):
        path = base_dir / ("pass2_seeds_%s.json" % name)
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
    return []


def _auto_unpack_if_container(bin_path: pathlib.Path, opts):
    total_len = bin_path.stat().st_size
    with bin_path.open("rb") as fh:
        raw_head = fh.read(0x2000)
    _fmt, _kname, rk, entries = parse_frm_toc(raw_head, total_len=total_len)
    is_container = opts.frm is not None or bool(entries) or rk is not None or raw_head[:8] == b"EXTRHEAD" or raw_head[:12] == b"PANASONIC\x00\x00\x00"
    if not is_container:
        return None
    out_dir = WORK_DIR / "decrypted_stages"
    manifest, targets = decrypt_and_prepare_runtime_stages(bin_path, out_dir, opts.model, opts.region)
    return manifest, targets


def _resolve_targets(opts) -> list:
    if opts.frm is not None:
        frm_path = pathlib.Path(opts.frm).resolve()
        if not frm_path.is_file():
            raise SystemExit("frm container not found: %s" % frm_path)
        _manifest, targets = decrypt_and_prepare_runtime_stages(
            frm_path, WORK_DIR / "decrypted_stages", opts.model, opts.region
        )
        return targets
    if opts.binary is not None:
        bin_path = pathlib.Path(opts.binary).resolve()
        if not bin_path.is_file():
            raise SystemExit("binary not found: %s" % bin_path)
        unpacked = _auto_unpack_if_container(bin_path, opts)
        if unpacked is not None:
            return unpacked[1]
        raw_prog = opts.program_name or ("/" + bin_path.name)
        prog_path = raw_prog if raw_prog.startswith("/") else "/" + raw_prog
        name = opts.only or prog_path.rsplit("/", 1)[-1]
        base_va = infer_base_va(bin_path, opts.model, opts.base)
        return [(name, prog_path, bin_path, base_va)]
    names = [opts.only] if opts.only else list(STAGES)
    targets = []
    for name in names:
        if name not in STAGES:
            raise SystemExit("unknown stage program '%s' (known: %s; or pass --binary)" % (name, ", ".join(STAGES)))
        proj_path, bin_path = STAGES[name]
        base_va = to_u32(int(opts.base, 0)) if opts.base is not None else STAGE_BASES[name]
        targets.append((name, proj_path, bin_path, base_va))
    return targets


def _run_all_passes(prog, img: Image, name: str, opts) -> dict:
    from ghidra.util.task import TaskMonitor

    rep = {"program": name, "model": opts.model, "base_va": "0x%08X" % img.base, "baseline": measure(prog)}
    p2 = pass2_program(prog, img)
    write_json(WORK_DIR, "pass2_seeds_%s.json" % name, p2["seeds"])
    rep["pass2"] = p2["stats"]
    rep["pass3"] = pass3_program(prog, img, name, p2["seeds"])
    rep["pass4"] = pass4_program(prog, img, name, p2["seeds"], not opts.no_switches)
    rep["verify"] = verify_program(prog, name)
    if not opts.no_save:
        prog.save("Panadiga zero-undef all", TaskMonitor.DUMMY)
    return rep


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("stage", choices=["decrypt-runtime", "import-loader", "baseline", "pass2", "pass3", "pass4", "verify", "all"])
    ap.add_argument("--only", default=None, help="restrict to one program name")
    ap.add_argument("--binary", default=None, help="path to an arbitrary raw firmware binary or container to analyze")
    ap.add_argument("--frm", default=None, help="path to PANA_DVD.FRM / PANA__SD.FRM / DIGADIAG.FRM for full PyGhidra decrypt + analysis")
    ap.add_argument("--region", default=None, help="region code override (e.g. LB, EC, EE, VP) for TV/electrical interpolation")
    ap.add_argument("--base", default=None, help="base virtual address for --binary (e.g. 0x57000000); inferred if omitted")
    ap.add_argument("--program-name", default=None, help="program path inside Ghidra project (default /<binary.name>)")
    ap.add_argument("--model", default=DEFAULT_MODEL, help="SSOT DIGA model ID for memory map projection (default: %s)" % DEFAULT_MODEL)
    ap.add_argument("--project-dir", default=PROJECT_DIR, help="Ghidra project directory")
    ap.add_argument("--project-name", default=PROJECT_NAME, help="cloned Ghidra project name (source project refused)")
    ap.add_argument("--language", default=LANGUAGE_ID, help="Ghidra SLEIGH language ID")
    ap.add_argument("--compiler", default=COMPILER_ID, help="Ghidra compiler spec ID")
    ap.add_argument("--out", default=None, help="report file name or explicit path override")
    ap.add_argument("--no-save", action="store_true", help="dry run: do not save the program")
    ap.add_argument("--no-switches", action="store_true", help="pass4: skip decompiler switch recovery")
    opts = ap.parse_args()
    if opts.stage == "decrypt-runtime":
        src_arg = opts.frm or opts.binary
        if not src_arg:
            raise SystemExit("decrypt-runtime requires --frm <PATH> or --binary <PATH>")
        src_path = pathlib.Path(src_arg).resolve()
        if not src_path.is_file():
            raise SystemExit("input not found: %s" % src_path)
        out_dir = pathlib.Path(opts.out).resolve() if opts.out else (WORK_DIR / "decrypted_stages")
        manifest, _targets = decrypt_and_prepare_runtime_stages(src_path, out_dir, opts.model, opts.region)
        print(json.dumps(manifest, indent=2))
        return 0
    guard_project(opts.project_name)
    pyghidra = start_ghidra()
    from ghidra.util.task import TaskMonitor

    project = pyghidra.open_project(opts.project_dir, opts.project_name)
    targets = _resolve_targets(opts)

    if opts.stage == "import-loader":
        if opts.binary is not None or opts.frm is not None:
            for _name, prog_path, bin_path, base_va in targets:
                import_binary(pyghidra, project, prog_path, bin_path, base_va, opts.language, opts.compiler)
                apply_model_memory_layout(project, pyghidra, prog_path, base_va, opts.model)
        else:
            import_loader(pyghidra, project)
            layout_loader(project, pyghidra)
        return 0

    reports = []
    for name, proj_path, bin_path, base_va in targets:
        t0 = time.time()
        if opts.stage == "all" and project.getProjectData().getFile(proj_path) is None:
            import_binary(pyghidra, project, proj_path, bin_path, base_va, opts.language, opts.compiler)
            apply_model_memory_layout(project, pyghidra, proj_path, base_va, opts.model)
        with pyghidra.program_context(project, proj_path) as prog:
            if opts.stage == "baseline":
                rep = measure(prog)
            elif opts.stage == "verify":
                rep = verify_program(prog, name)
            else:
                img = Image(prog, bin_path)
                if opts.stage == "all":
                    rep = _run_all_passes(prog, img, name, opts)
                elif opts.stage == "pass2":
                    res = pass2_program(prog, img)
                    write_json(WORK_DIR, "pass2_seeds_%s.json" % name, res["seeds"])
                    rep = {"program": name, "pass2": res["stats"]}
                elif opts.stage == "pass3":
                    rep = {"program": name, "pass3": pass3_program(prog, img, name, load_seed_file(name))}
                else:
                    rep = {"program": name, "pass4": pass4_program(prog, img, name, load_seed_file(name), not opts.no_switches)}
                if opts.stage != "all":
                    rep["after"] = measure(prog, with_todo=False)
                if opts.stage != "all" and not opts.no_save:
                    prog.save("Panadiga zero-undef %s" % opts.stage, TaskMonitor.DUMMY)
        rep["seconds"] = round(time.time() - t0, 1)
        reports.append(rep)
        print(json.dumps(rep, indent=2, default=str))
        sys.stdout.flush()

    dest_dir = REPORT_DIR if opts.stage in ("baseline", "verify") else WORK_DIR
    suffix = ("_" + opts.only) if opts.only else (("_" + targets[0][0]) if opts.binary and targets else "")
    out_name = opts.out or ("%s%s.json" % (opts.stage, suffix))
    out_path = pathlib.Path(out_name)
    if out_path.is_absolute() or len(out_path.parts) > 1:
        dest_dir = out_path.parent
        out_name = out_path.name
    print("report:", write_json(dest_dir, out_name, reports))
    return 0


if __name__ == "__main__":
    sys.exit(main())
"#;
