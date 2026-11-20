//! Main Python `run_unbrick_audit` entrypoint and JSON report serialization block.

use super::AuditSiteVas;
use crate::re::bgo_unbrick::{ORIG_A2BC_BGO_CALL, ORIG_A2CD_HOOK, ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ};
use crate::re::main_passthrough::{
    ORIG_FPC_STATUS_CTRL, ORIG_ODD_INIT_STATUS, ORIG_ODD_STATE_BRANCH,
};

pub(super) fn render_audit_runner(vas: &AuditSiteVas) -> String {
    format!(
        r#"def run_unbrick_audit(prog=None, output_json=None):
    if prog is None:
        prog = currentProgram
    setup_memory_map(prog)
    imported = import_symbols(prog)

    mem = prog.getMemory()
    space = prog.getAddressFactory().getDefaultAddressSpace()
    bm_mgr = prog.getBookmarkManager()
    image_base = prog.getImageBase().getOffset() & 0xFFFFFFFF

    exec_size = 0
    first_init_base = None
    string_hits = {{}}
    mmio_lit_hits = {{}}
    for blk in mem.getBlocks():
        if not blk.isInitialized():
            continue
        blk_size = int(blk.getSize())
        exec_size += blk_size
        blk_base = blk.getStart().getOffset() & 0xFFFFFFFF
        if first_init_base is None:
            first_init_base = blk_base
        raw = _read_initialized_block_u8(blk)
        for kw, addrs in scan_ascii_keywords(raw, blk_base).items():
            string_hits.setdefault(kw, []).extend(addrs)
            _add_bookmark(bm_mgr, space, addrs[0], "Panadiga-Str", kw)
        for label, addrs in scan_le32_constants(raw, blk_base).items():
            mmio_lit_hits.setdefault(label, []).extend(addrs)
            _add_bookmark(bm_mgr, space, addrs[0], "Panadiga-MMIO", label)

    effective_base = image_base if image_base != 0 else (first_init_base or 0)
    stage = detect_stage_from_metadata(effective_base, prog.getName(), string_hits.keys(), exec_size)
    prog061_status = classify_prog061_sites(
        _read_prog_bytes(prog, 0x{prog_sum:08X}, {orig_sum_len}),
        _read_prog_bytes(prog, 0x{prog_devinfo:08X}, {orig_devinfo_len}),
        _read_prog_bytes(prog, 0x{prog_a2cd:08X}, {orig_a2cd_len}),
        _read_prog_bytes(prog, 0x{prog_a2bc:08X}, {orig_a2bc_len}),
    )
    main3410_status = classify_main3410_sites(
        _read_prog_bytes(prog, 0x{main_fpc_ctrl:08X}, {orig_fpc_len}),
        _read_prog_bytes(prog, 0x{main_odd_branch:08X}, {orig_odd_br_len}),
        _read_prog_bytes(prog, 0x{main_odd_status:08X}, {orig_odd_st_len}),
    )

    for site_addr in ("0x{prog_sum:08X}", "0x{prog_devinfo:08X}", "0x{prog_a2bc:08X}", "0x{prog_a2cd:08X}", "0x{main_fpc_ctrl:08X}", "0x{main_odd_branch:08X}", "0x{main_odd_status:08X}"):
        _add_bookmark(bm_mgr, space, site_addr, "Panadiga-Unbrick", "Verified Panadiga patch/audit site")

    report = {{
        "program": prog.getName(),
        "image_base": "0x%08X" % image_base,
        "initialized_bytes": exec_size,
        "detected_stage": stage,
        "symbols_annotated": imported,
        "prog061_bgo_status": prog061_status,
        "main3410_passthrough_status": main3410_status,
        "diagnostic_strings": string_hits,
        "mmio_literal_pools": mmio_lit_hits,
        "xrefs": _collect_xref_counts(prog),
    }}

    out_path = output_json or _default_report_path(prog)
    with open(out_path, "w") as fh:
        json.dump(report, fh, indent=2, sort_keys=True)

    print("=" * 72)
    print("[Panadiga Audit] Stage=%s | Base=0x%08X | PROG061=%s | MAIN3410=%s" % (
        stage, image_base, prog061_status["mode"], main3410_status["mode"]
    ))
    print("[Panadiga Audit] Strings matched: %d | MMIO literals: %d | Report: %s" % (
        len(string_hits), len(mmio_lit_hits), out_path
    ))
    print("=" * 72)
    return report

if __name__ == "__main__":
    run_unbrick_audit()
"#,
        prog_sum = vas.prog_sum,
        prog_devinfo = vas.prog_devinfo,
        prog_a2bc = vas.prog_a2bc,
        prog_a2cd = vas.prog_a2cd,
        main_fpc_ctrl = vas.main_fpc_ctrl,
        main_odd_branch = vas.main_odd_branch,
        main_odd_status = vas.main_odd_status,
        orig_sum_len = ORIG_SUM_BEQ.len(),
        orig_devinfo_len = ORIG_DEVINFO_BEQ.len(),
        orig_a2bc_len = ORIG_A2BC_BGO_CALL.len(),
        orig_a2cd_len = ORIG_A2CD_HOOK.len(),
        orig_fpc_len = ORIG_FPC_STATUS_CTRL.len(),
        orig_odd_br_len = ORIG_ODD_STATE_BRANCH.len(),
        orig_odd_st_len = ORIG_ODD_INIT_STATUS.len(),
    )
}
