"""Unit tests for cross-platform scripts/ helpers (ops, build, validation)."""

from __future__ import annotations

from pathlib import Path
import subprocess
import sys
import threading
from typing import Mapping
import zipfile
import pytest

from scripts.build import detect_android_sdk, parse_args
import scripts.ops.toolchain as toolchain
import scripts.validate_gates as validate_gates
from scripts.build_lanes.android_verify import validate_apk_artifact
from scripts.panadiga_cache import (
    compute_tree_digest,
    copy_if_changed,
    discover_workspace_inventory,
    find_repo_root,
    is_step_fresh,
    record_step_cache,
)
from scripts.tui import PipelineSelection, build_execution_plan
from scripts.validate import parse_args as parse_validate_args
from scripts.panadiga_ops import (
    ProcessAbortedError,
    ProcessTracker,
    ROOT,
    active_process_tracker,
    find_msvc_vcvars64,
    load_msvc_env,
    locate_panadiga_binary,
    locate_panadiga_desktop_binary,
    panadiga_binary_name,
    panadiga_desktop_binary_name,
    resolve_executable,
    sha256_file,
    update_sha256_manifest,
    use_process_tracker,
)


def test_sha256_manifest_is_idempotent(tmp_path: Path) -> None:
    artifact = tmp_path / "panadiga-test.bin"
    artifact.write_bytes(b"PANASONIC-AM34-TEST")
    digest = sha256_file(artifact).upper()

    manifest = tmp_path / "SHA256SUMS.txt"
    manifest.write_text("0000  other.zip\nDEADBEEF  panadiga-test.bin\n", encoding="utf-8")

    update_sha256_manifest(manifest, [artifact])
    lines = manifest.read_text(encoding="utf-8").splitlines()
    assert lines == ["0000  other.zip", f"{digest}  panadiga-test.bin"]


def test_detect_android_sdk_from_environment(tmp_path: Path) -> None:
    sdk_dir = tmp_path / "Android" / "Sdk"
    ndk_v1 = sdk_dir / "ndk" / "26.1.10909125"
    ndk_v2 = sdk_dir / "ndk" / "27.0.12077973"
    ndk_v1.mkdir(parents=True)
    ndk_v2.mkdir(parents=True)

    detected_sdk, detected_ndk = detect_android_sdk({"ANDROID_HOME": str(sdk_dir)})
    assert detected_sdk == sdk_dir
    assert detected_ndk == ndk_v2


def test_resolve_executable_raises_for_missing_command() -> None:
    with pytest.raises(FileNotFoundError):
        resolve_executable("__panadiga_nonexistent_tool_xyz__")


def test_build_cli_arg_parsing() -> None:
    args = parse_args(["--desktop", "--pico"])
    assert args.desktop is True
    assert args.pico is True
    assert args.android is False


def test_workspace_binary_names_and_locators() -> None:
    cli_name = panadiga_binary_name()
    gui_name = panadiga_desktop_binary_name()
    assert cli_name.startswith("panadiga")
    assert gui_name.startswith("panadiga-desktop")
    assert callable(locate_panadiga_binary)
    assert callable(locate_panadiga_desktop_binary)
    vcvars = find_msvc_vcvars64()
    assert vcvars is None or vcvars.name.lower() == "vcvars64.bat"
    augmented = load_msvc_env({"PATH": ""})
    assert "PATH" in augmented


def test_process_tracker_and_validate_args() -> None:
    parsed = parse_validate_args(["--sequential", "--jobs", "4"])
    assert parsed.sequential is True
    assert parsed.jobs == 4

    tracker = ProcessTracker()
    assert active_process_tracker() is None
    with use_process_tracker(tracker):
        assert active_process_tracker() is tracker
        assert tracker.is_aborted() is False
        tracker.terminate_all()
        assert tracker.is_aborted() is True
    assert active_process_tracker() is None
    assert issubclass(ProcessAbortedError, RuntimeError)


def test_recursive_root_cache_roundtrip_and_tui_plan(tmp_path: Path) -> None:
    nested = ROOT / "apps" / "desktop" / "src"
    assert find_repo_root(nested) == ROOT

    src_dir = tmp_path / "src"
    src_dir.mkdir()
    input_file = src_dir / "main.c"
    input_file.write_text("int main(void) { return 0; }\n", encoding="utf-8")
    out_file = tmp_path / "out.bin"
    out_file.write_bytes(b"PANADIGA-BIN")

    cache_dir = tmp_path / ".cache"
    digest = compute_tree_digest(tmp_path, [src_dir])
    assert not is_step_fresh(tmp_path, "demo", digest, [out_file], cache_dir=cache_dir)
    record_step_cache(tmp_path, "demo", digest, [out_file], cache_dir=cache_dir)
    assert is_step_fresh(tmp_path, "demo", digest, [out_file], cache_dir=cache_dir)

    out_file.write_bytes(b"MODIFIED")
    assert not is_step_fresh(tmp_path, "demo", digest, [out_file], cache_dir=cache_dir)

    mirror = tmp_path / "mirror.bin"
    assert copy_if_changed(out_file, mirror) is True
    assert copy_if_changed(out_file, mirror) is False

    inventory = discover_workspace_inventory(ROOT, {})
    assert "crates/panadiga-core" in inventory.crates
    assert "apps/desktop" in inventory.apps
    assert "firmware/picofw" in inventory.firmware_targets

    plan = build_execution_plan(
        PipelineSelection(ci=True, frontend=True, pico=True, desktop=True, android=True),
        inventory,
        {},
    )
    assert any("PARALELO" in wave.concurrency for wave in plan)
    assert any("SERIAL" in wave.concurrency for wave in plan)

    args_tui = parse_args(["--tui", "--ci", "--no-cache"])
    assert args_tui.tui is True
    assert args_tui.ci is True
    assert args_tui.no_cache is True


def _make_fake_sdk_and_apk(tmp_path: Path, *, include_armeabi: bool = True) -> tuple[Path, Path]:
    sdk_dir = tmp_path / "Sdk"
    bt_dir = sdk_dir / "build-tools" / "36.0.0"
    bt_dir.mkdir(parents=True)
    ext_map = {"zipalign": ".exe" if sys.platform == "win32" else "", "apksigner": ".bat" if sys.platform == "win32" else "", "aapt2": ".exe" if sys.platform == "win32" else ""}
    for stem, ext in ext_map.items():
        (bt_dir / f"{stem}{ext}").write_text("", encoding="utf-8")

    apk_path = tmp_path / "panadiga-test.apk"
    with zipfile.ZipFile(apk_path, "w", compression=zipfile.ZIP_STORED) as zf:
        zf.writestr("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\n")
        zf.writestr("AndroidManifest.xml", b"manifest")
        zf.writestr("classes.dex", b"dex\n035\x00")
        zf.writestr("lib/arm64-v8a/libpanadiga_lib.so", b"\x7fELF" + b"\x00" * 1_100_000)
        if include_armeabi:
            zf.writestr("lib/armeabi-v7a/libpanadiga_lib.so", b"\x7fELF" + b"\x00" * 600_000)
    return sdk_dir, apk_path


def test_validate_apk_artifact_checks_signatures_abis_and_sdk(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    sdk_dir, apk_path = _make_fake_sdk_and_apk(tmp_path, include_armeabi=True)
    xmltree_good = (
        'A: http://schemas.android.com/apk/res/android:minSdkVersion(0x0101020c)=24\n'
        'A: http://schemas.android.com/apk/res/android:targetSdkVersion(0x01010270)=34\n'
    )
    apksig_good = (
        "Verifies\n"
        "Verified using v1 scheme (JAR signing): true\n"
        "Verified using v2 scheme (APK Signature Scheme v2): true\n"
        "Verified using v3 scheme (APK Signature Scheme v3): true\n"
    )

    state = {"xml": xmltree_good, "sig": apksig_good}

    def fake_run(cmd: list[str], **_kwargs: object) -> subprocess.CompletedProcess[str]:
        exe = Path(cmd[0]).name.lower()
        if exe.startswith("zipalign"):
            return subprocess.CompletedProcess(cmd, 0, "", "")
        if exe.startswith("apksigner"):
            return subprocess.CompletedProcess(cmd, 0, state["sig"], "")
        return subprocess.CompletedProcess(cmd, 0, state["xml"], "")

    monkeypatch.setattr("scripts.build_lanes.android_verify.subprocess.run", fake_run)
    summary = validate_apk_artifact(sdk_dir, apk_path, require_dual_abi=True, env={})
    assert summary["abis"] == ["arm64-v8a", "armeabi-v7a"]
    assert summary["min_sdk"] == 24
    assert summary["target_sdk"] == 34

    state["xml"] = xmltree_good + 'A: compileSdkVersionCodename(0x01010573)="16"\n'
    assert validate_apk_artifact(sdk_dir, apk_path, require_dual_abi=True, env={})["target_sdk"] == 34

    state["xml"] = xmltree_good + 'A: compileSdkVersionCodename="17"\n'
    with pytest.raises(RuntimeError, match="compileSdkVersionCodename"):
        validate_apk_artifact(sdk_dir, apk_path, require_dual_abi=True, env={})

    state["xml"] = xmltree_good
    state["sig"] = apksig_good.replace("v1 scheme (JAR signing): true", "v1 scheme (JAR signing): false")
    with pytest.raises(RuntimeError, match="v1 scheme"):
        validate_apk_artifact(sdk_dir, apk_path, require_dual_abi=True, env={})

    _, single_apk = _make_fake_sdk_and_apk(tmp_path / "single", include_armeabi=False)
    state["sig"] = apksig_good
    with pytest.raises(RuntimeError, match="armeabi-v7a"):
        validate_apk_artifact(sdk_dir, single_apk, require_dual_abi=True, env={})


def test_validate_parallel_lanes_schedules_playwright_after_ssot(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    lock = threading.Lock()
    order: list[str] = []

    def _rec(name: str) -> object:
        def _fake(_env: Mapping[str, str], *, capture_output: bool = False) -> None:
            _ = capture_output
            with lock:
                order.append(name)

        return _fake

    for attr, key in (
        ("_gate_rust", "rust"),
        ("_gate_typescript", "ts"),
        ("_gate_vitest", "vitest"),
        ("_gate_jscpd", "jscpd"),
        ("_gate_uxlint", "uxlint"),
        ("_gate_playwright", "playwright"),
        ("_gate_ghidra", "ghidra"),
        ("_gate_c11_host", "c11"),
        ("_gate_python_mypy", "mypy"),
        ("_gate_python_compile_and_pytest", "pytest"),
    ):
        monkeypatch.setattr(validate_gates, attr, _rec(key))

    def _fake_load_msvc(base: Mapping[str, str]) -> dict[str, str]:
        with lock:
            order.append("msvc_prewarm")
        return dict(base)

    monkeypatch.setattr(validate_gates, "load_msvc_env", _fake_load_msvc)

    validate_gates.run_parallel_lanes({}, max_jobs=4, ghidra_smoke=False)

    assert order[0] == "msvc_prewarm"
    assert set(order) == {
        "msvc_prewarm",
        "rust",
        "ts",
        "vitest",
        "jscpd",
        "uxlint",
        "c11",
        "mypy",
        "ghidra",
        "playwright",
        "pytest",
    }
    for lane in ("rust", "ts", "vitest", "jscpd", "uxlint", "c11", "mypy"):
        assert order.index(lane) < order.index("ghidra")
    assert order.index("ghidra") < order.index("playwright")
    assert order.index("ghidra") < order.index("pytest")


def test_discover_msvc_delta_does_not_cache_transient_timeout(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(toolchain, "_MSVC_DELTA_CACHE", None)
    vcvars = tmp_path / "vcvars64.bat"
    vcvars.write_text("", encoding="utf-8")
    monkeypatch.setattr(toolchain, "find_msvc_vcvars64", lambda: vcvars)

    calls = 0

    def fake_run(cmd: str, **_kwargs: object) -> subprocess.CompletedProcess[str]:
        nonlocal calls
        calls += 1
        if calls == 1:
            raise subprocess.TimeoutExpired(cmd=cmd, timeout=120)
        if calls == 2:
            return subprocess.CompletedProcess(
                cmd,
                0,
                "INCLUDE=C:\\inc\nPATH=C:\\msvc\\bin\n",
                "",
            )
        raise AssertionError("MSVC probe should not run after positive cache hit")

    monkeypatch.setattr("scripts.ops.toolchain.subprocess.run", fake_run)

    assert toolchain._discover_msvc_delta() is None
    assert toolchain._discover_msvc_delta() == {"INCLUDE": "C:\\inc", "PATH": "C:\\msvc\\bin"}
    assert toolchain._discover_msvc_delta() == {"INCLUDE": "C:\\inc", "PATH": "C:\\msvc\\bin"}
    assert calls == 2
