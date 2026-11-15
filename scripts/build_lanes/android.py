"""Android SDK/NDK detection, APK alignment/signing, and Tauri Mobile APK build lane."""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable, Mapping

from scripts.build_lanes.concurrency import flush_output, run_concurrent_tasks
from scripts.build_lanes.desktop import build_frontend
from scripts.panadiga_ops import ROOT, print_banner, print_status, run_step, update_sha256_manifest


def detect_android_sdk(env: Mapping[str, str]) -> tuple[Path, Path | None]:
    """Detect Android SDK and latest NDK directories cross-platform."""
    sdk_env = env.get("ANDROID_HOME") or env.get("ANDROID_SDK_ROOT")
    if sdk_env and Path(sdk_env).is_dir():
        sdk_dir = Path(sdk_env)
    else:
        fallback = (
            Path(env.get("LOCALAPPDATA", "")) / "Android" / "Sdk"
            if sys.platform == "win32"
            else Path.home() / "Android" / "Sdk"
        )
        if not fallback.is_dir():
            raise FileNotFoundError(
                "ANDROID_HOME not set and default Android SDK directory not found."
            )
        sdk_dir = fallback

    ndk_env = env.get("NDK_HOME") or env.get("ANDROID_NDK_ROOT")
    ndk_dir: Path | None = Path(ndk_env) if ndk_env and Path(ndk_env).is_dir() else None
    if ndk_dir is None and (sdk_dir / "ndk").is_dir():
        candidates = sorted((p for p in (sdk_dir / "ndk").iterdir() if p.is_dir()))
        if candidates:
            ndk_dir = candidates[-1]
    return sdk_dir, ndk_dir


def _find_build_tool(sdk_dir: Path, tool_stem: str) -> Path:
    """Resolve the newest Android SDK build-tools executable (zipalign / apksigner)."""
    bt_dir = sdk_dir / "build-tools"
    if not bt_dir.is_dir():
        raise FileNotFoundError(f"Android build-tools directory missing under {sdk_dir}")
    exts = (".bat", ".exe", "") if sys.platform == "win32" else ("",)
    for ver_dir in sorted((p for p in bt_dir.iterdir() if p.is_dir()), reverse=True):
        for ext in exts:
            candidate = ver_dir / f"{tool_stem}{ext}"
            if candidate.is_file():
                return candidate
    raise FileNotFoundError(f"Android SDK tool '{tool_stem}' not found under {bt_dir}")


def _ensure_signed_apk(
    sdk_dir: Path,
    source_apk: Path,
    dest_apk: Path,
    env: Mapping[str, str],
) -> None:
    """Align and sign an Android APK with debug keystore so it is directly sideloadable."""
    apksigner = _find_build_tool(sdk_dir, "apksigner")
    zipalign = _find_build_tool(sdk_dir, "zipalign")

    if "unsigned" not in source_apk.name.lower():
        verify_proc = subprocess.run(
            [str(apksigner), "verify", str(source_apk)],
            cwd=ROOT,
            env=dict(env),
            capture_output=True,
            check=False,
        )
        if verify_proc.returncode == 0:
            shutil.copy2(source_apk, dest_apk)
            return

    keystore = Path.home() / ".android" / "debug.keystore"
    if not keystore.is_file():
        keystore.parent.mkdir(parents=True, exist_ok=True)
        run_step(
            [
                "keytool",
                "-genkey",
                "-v",
                "-keystore",
                str(keystore),
                "-alias",
                "androiddebugkey",
                "-keyalg",
                "RSA",
                "-keysize",
                "2048",
                "-validity",
                "10000",
                "-storepass",
                "android",
                "-keypass",
                "android",
                "-dname",
                "CN=Android Debug,O=Android,C=US",
            ],
            cwd=ROOT,
            env=env,
        )

    aligned_tmp = dest_apk.with_suffix(".aligned.apk")
    run_step([str(zipalign), "-p", "-f", "4", str(source_apk), str(aligned_tmp)], cwd=ROOT, env=env)
    try:
        run_step(
            [
                str(apksigner),
                "sign",
                "--ks",
                str(keystore),
                "--ks-key-alias",
                "androiddebugkey",
                "--ks-pass",
                "pass:android",
                "--key-pass",
                "pass:android",
                "--out",
                str(dest_apk),
                str(aligned_tmp),
            ],
            cwd=ROOT,
            env=env,
        )
    finally:
        aligned_tmp.unlink(missing_ok=True)

    run_step([str(apksigner), "verify", str(dest_apk)], cwd=ROOT, env=env)


def prepare_android_env(base_env: Mapping[str, str]) -> tuple[dict[str, str], Path, Path, Path]:
    """Configure Android SDK/NDK environment variables and Tauri `local.properties`."""
    sdk_dir, ndk_dir = detect_android_sdk(base_env)
    env = dict(base_env)
    env["ANDROID_HOME"] = str(sdk_dir)
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target" / "android-cargo"))
    if ndk_dir is not None:
        env["NDK_HOME"] = str(ndk_dir)
        env["ANDROID_NDK_ROOT"] = str(ndk_dir)

    desktop_dir = ROOT / "apps" / "desktop"
    gen_android = desktop_dir / "src-tauri" / "gen" / "android"
    gen_android.mkdir(parents=True, exist_ok=True)
    props_lines = [f"sdk.dir={sdk_dir.as_posix()}"]
    if ndk_dir is not None:
        props_lines.append(f"ndk.dir={ndk_dir.as_posix()}")
    props_file = gen_android / "local.properties"
    desired = "\n".join(props_lines) + "\n"
    if not props_file.is_file() or props_file.read_text(encoding="utf-8", errors="ignore") != desired:
        props_file.write_text(desired, encoding="utf-8")
    return env, sdk_dir, desktop_dir, gen_android


def install_android_rust_target(env: Mapping[str, str]) -> None:
    """Ensure the `aarch64-linux-android` Rust compilation target is installed."""
    out = run_step(
        ["rustup", "target", "add", "aarch64-linux-android"],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )
    flush_output(out)


def build_android(
    base_env: Mapping[str, str],
    *,
    skip_frontend: bool = False,
    skip_prep: bool = False,
    use_cache: bool = True,
    capture_output: bool = False,
) -> Path:
    """Compile, zipalign, and sign Android ARM64 APK directly from panadiga-desktop workspace."""
    print_banner("BUILD ANDROID: Tauri 2 Mobile APK (AArch64)")
    env, sdk_dir, desktop_dir, gen_android = prepare_android_env(base_env)

    prep_tasks: list[tuple[str, Callable[[], Any]]] = []
    if not skip_prep:
        prep_tasks.append(("rustup", lambda: install_android_rust_target(env)))
    if not skip_frontend:
        prep_tasks.append(("frontend", lambda: build_frontend(env, use_cache=use_cache, capture_output=capture_output)))
    run_concurrent_tasks(prep_tasks)

    if not (gen_android / "app").is_dir():
        init_out = run_step(
            ["npx", "@tauri-apps/cli", "android", "init", "--ci", "--skip-targets-install"],
            cwd=desktop_dir,
            env=env,
            capture_output=capture_output,
        )
        flush_output(init_out)

    android_cmd = ["npx", "@tauri-apps/cli", "android", "build", "--apk", "--target", "aarch64"]
    if skip_frontend:
        skip_cfg = ROOT / "target" / ".panadiga-cache" / "tauri.skip_frontend.json"
        skip_cfg.parent.mkdir(parents=True, exist_ok=True)
        desired_cfg = '{"build":{"beforeBuildCommand":""}}\n'
        if not skip_cfg.is_file() or skip_cfg.read_text(encoding="utf-8", errors="ignore") != desired_cfg:
            skip_cfg.write_text(desired_cfg, encoding="utf-8")
        android_cmd.extend(["--config", str(skip_cfg)])

    build_out = run_step(android_cmd, cwd=desktop_dir, env=env, capture_output=capture_output)
    flush_output(build_out)

    apk_root = gen_android / "app" / "build" / "outputs" / "apk"
    apk_candidates = sorted(apk_root.rglob("*.apk"))
    if not apk_candidates:
        raise FileNotFoundError("Android build completed but no .apk artifact was produced.")
    preferred = [p for p in apk_candidates if "release" in p.name.lower()]
    source_apk = preferred[0] if preferred else apk_candidates[0]

    dist_dir = ROOT / "dist"
    releases_dir = dist_dir / "releases"
    releases_dir.mkdir(parents=True, exist_ok=True)
    dist_apk = dist_dir / "panadiga-android-arm64.apk"
    rel_apk = releases_dir / "panadiga-android-arm64.apk"

    _ensure_signed_apk(sdk_dir, source_apk, dist_apk, env)
    shutil.copy2(dist_apk, rel_apk)
    update_sha256_manifest(releases_dir / "SHA256SUMS.txt", [rel_apk])
    print_status("OK", f"Signed Android APK packaged at: {rel_apk}", "32")
    return rel_apk
