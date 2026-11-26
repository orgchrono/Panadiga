"""Android SDK/NDK setup and dual-ABI Tauri Mobile APK build lane."""

from __future__ import annotations

import shutil
from pathlib import Path
from typing import Callable, Mapping

from scripts.build_lanes.android_verify import (
    detect_android_sdk as detect_android_sdk,
    sign_and_align_apk,
    validate_apk_artifact,
)
from scripts.build_lanes.concurrency import flush_output, run_concurrent_tasks
from scripts.build_lanes.desktop import build_frontend
from scripts.panadiga_ops import ROOT, print_banner, print_status, run_step, update_sha256_manifest

ANDROID_RUST_TARGETS = ("aarch64-linux-android", "armv7-linux-androideabi")


def prepare_android_env(base_env: Mapping[str, str]) -> tuple[dict[str, str], Path, Path, Path]:
    """Configure Android SDK/NDK environment variables and Tauri `local.properties`."""
    sdk_dir, ndk_dir = detect_android_sdk(base_env)
    env = dict(base_env)
    env["ANDROID_HOME"] = str(sdk_dir)
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target" / "android-cargo"))
    gradle_opts = env.get("GRADLE_OPTS", "")
    if "org.gradle.daemon" not in gradle_opts:
        env["GRADLE_OPTS"] = f"{gradle_opts} -Dorg.gradle.daemon=false".strip()
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
    """Ensure both `aarch64-linux-android` and `armv7-linux-androideabi` Rust targets are installed."""
    out = run_step(
        ["rustup", "target", "add", *ANDROID_RUST_TARGETS],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )
    flush_output(out)


def _select_release_apk(apk_root: Path) -> Path:
    """Select the universal release APK produced by Tauri/Gradle."""
    apk_candidates = sorted(apk_root.rglob("*.apk"))
    if not apk_candidates:
        raise FileNotFoundError("Android build completed but no .apk artifact was produced.")
    universal_rel = [
        p
        for p in apk_candidates
        if "universal" in p.as_posix().lower() and "release" in p.name.lower()
    ]
    if universal_rel:
        return universal_rel[0]
    preferred = [p for p in apk_candidates if "release" in p.name.lower()]
    return preferred[0] if preferred else apk_candidates[0]


def build_android(
    base_env: Mapping[str, str],
    *,
    skip_frontend: bool = False,
    skip_prep: bool = False,
    use_cache: bool = True,
    capture_output: bool = False,
) -> Path:
    """Compile, zipalign, sign, and verify the dual-ABI Universal Android APK."""
    print_banner("BUILD ANDROID: Tauri 2 Mobile APK (ARM64 + ARMv7 Universal)")
    env, sdk_dir, desktop_dir, gen_android = prepare_android_env(base_env)

    prep_tasks: list[tuple[str, Callable[[], object]]] = []
    if not skip_prep:
        prep_tasks.append(("rustup", lambda: install_android_rust_target(env)))
    if not skip_frontend:
        prep_tasks.append(
            (
                "frontend",
                lambda: build_frontend(
                    env,
                    use_cache=use_cache,
                    capture_output=capture_output,
                    android=True,
                ),
            )
        )
    run_concurrent_tasks(prep_tasks)

    if not (gen_android / "app").is_dir() or not (gen_android / "tauri.settings.gradle").is_file():
        init_out = run_step(
            ["npx", "--yes", "@tauri-apps/cli@2.12.6", "android", "init", "--ci", "--skip-targets-install"],
            cwd=desktop_dir,
            env=env,
            capture_output=capture_output,
        )
        flush_output(init_out)

    if skip_frontend:
        skip_cfg = ROOT / "target" / ".panadiga-cache" / "tauri.android.skip_frontend.json"
        skip_cfg.parent.mkdir(parents=True, exist_ok=True)
        desired_cfg = '{"build":{"frontendDist":"../dist-android","beforeBuildCommand":""}}\n'
        if not skip_cfg.is_file() or skip_cfg.read_text(encoding="utf-8", errors="ignore") != desired_cfg:
            skip_cfg.write_text(desired_cfg, encoding="utf-8")
        cfg_path = skip_cfg
    else:
        cfg_path = desktop_dir / "src-tauri" / "tauri.android.conf.json"

    android_cmd = [
        "npx",
        "--yes",
        "@tauri-apps/cli@2.12.6",
        "android",
        "build",
        "--apk",
        "--config",
        str(cfg_path),
        "--target",
        "aarch64",
        "armv7",
    ]
    build_out = run_step(android_cmd, cwd=desktop_dir, env=env, capture_output=capture_output)
    flush_output(build_out)

    source_apk = _select_release_apk(gen_android / "app" / "build" / "outputs" / "apk")
    dist_dir = ROOT / "dist"
    releases_dir = dist_dir / "releases"
    releases_dir.mkdir(parents=True, exist_ok=True)
    dist_apk = dist_dir / "panadiga-android-arm64.apk"
    rel_apk = releases_dir / "panadiga-android-arm64.apk"

    sign_and_align_apk(sdk_dir, source_apk, dist_apk, env=env)
    summary = validate_apk_artifact(sdk_dir, dist_apk, require_dual_abi=True, env=env)
    shutil.copy2(dist_apk, rel_apk)
    update_sha256_manifest(releases_dir / "SHA256SUMS.txt", [rel_apk])
    print_status("OK", f"Verified signed Android APK ({summary['abis']}): {rel_apk}", "32")
    return rel_apk
