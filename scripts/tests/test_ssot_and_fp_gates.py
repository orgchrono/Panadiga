"""Live and synthetic negative tests for SSOT, zero-panic, FP, and blob hygiene gates."""

from __future__ import annotations

from pathlib import Path

from scripts.panadiga_ops import ROOT
from scripts.validate_ssot_and_fp import (
    check_prod_rust_zero_panic_macros,
    check_prod_rust_zero_unwrap,
    check_python_no_unsafe_exec,
    check_tracked_large_blobs,
    check_ts_no_oop_classes,
    main as validate_ssot_main,
    run_ssot_and_fp_audit,
)


def test_live_repo_passes_all_ssot_and_fp_gates() -> None:
    report = run_ssot_and_fp_audit(ROOT)
    failures = {k: v for k, v in report.items() if v}
    assert failures == {}, f"SSOT/FP gate violations on live workspace: {failures}"
    assert validate_ssot_main(["--root", str(ROOT), "--check"]) == 0


def test_negative_guards_catch_unwrap_panic_blob_oop_and_unsafe_python(tmp_path: Path) -> None:
    bad_rs = tmp_path / "crates" / "demo" / "src" / "lib.rs"
    bad_rs.parent.mkdir(parents=True)
    bad_rs.write_text(
        "pub fn broken(x: Option<u8>) -> u8 { x." + "unwrap() }\n",
        encoding="utf-8",
    )
    bad_panic_rs = tmp_path / "crates" / "demo" / "src" / "boom.rs"
    bad_panic_rs.write_text(
        "pub fn crash() { " + "pan" + 'ic!("boom"); }\n',
        encoding="utf-8",
    )
    assert len(check_prod_rust_zero_unwrap(tmp_path)) == 1
    assert len(check_prod_rust_zero_panic_macros(tmp_path)) == 1

    bad_ts = tmp_path / "apps" / "desktop" / "src" / "DemoService.ts"
    bad_ts.parent.mkdir(parents=True)
    bad_ts.write_text("export " + "class DemoController {}\n", encoding="utf-8")
    assert len(check_ts_no_oop_classes(tmp_path)) == 1

    bad_py = tmp_path / "scripts" / "danger.py"
    bad_py.parent.mkdir(parents=True)
    bad_py.write_text(
        "def run() -> None:\n    sub_call('ls', shel" + "l=True)\n",
        encoding="utf-8",
    )
    assert len(check_python_no_unsafe_exec(tmp_path)) == 1

    big_blob = tmp_path / "assets" / "oversized_dump.bin"
    big_blob.parent.mkdir(parents=True)
    with big_blob.open("wb") as fh:
        fh.seek(1_048_576)
        fh.write(b"\x00")
    assert len(check_tracked_large_blobs(tmp_path, tracked_files=("assets/oversized_dump.bin",))) == 1
