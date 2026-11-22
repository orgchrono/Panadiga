"""Android APK alignment, deterministic signing, and manifest/ABI/signature validator."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import zipfile
from pathlib import Path
from typing import Mapping

from scripts.panadiga_ops import ROOT, run_step

DEFAULT_BENCH_KEYSTORE = (
    ROOT / "apps" / "desktop" / "src-tauri" / "gen" / "android" / "keystore" / "panadiga-bench.keystore"
)
BENCH_KEY_ALIAS = "panadigabench"
BENCH_KEY_PASS = "panadiga"
REQUIRED_APK_ENTRIES = (
    "META-INF/MANIFEST.MF",
    "AndroidManifest.xml",
    "classes.dex",
    "lib/arm64-v8a/libpanadiga_lib.so",
)
DUAL_ABI_ENTRY = "lib/armeabi-v7a/libpanadiga_lib.so"
REQUIRED_SIGN_SCHEMES = (
    "Verified using v1 scheme (JAR signing): true",
    "Verified using v2 scheme (APK Signature Scheme v2): true",
    "Verified using v3 scheme (APK Signature Scheme v3): true",
)


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


def find_build_tool(sdk_dir: Path, tool_stem: str) -> Path:
    """Resolve the newest Android SDK build-tools executable (zipalign / apksigner / aapt2)."""
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


def ensure_bench_keystore(
    keystore_path: Path = DEFAULT_BENCH_KEYSTORE,
    env: Mapping[str, str] | None = None,
) -> Path:
    """Return existing bench keystore or synthesize a standard RSA-2048 keystore."""
    if keystore_path.is_file():
        return keystore_path
    keystore_path.parent.mkdir(parents=True, exist_ok=True)
    run_step(
        [
            "keytool",
            "-genkey",
            "-v",
            "-keystore",
            str(keystore_path),
            "-alias",
            BENCH_KEY_ALIAS,
            "-keyalg",
            "RSA",
            "-keysize",
            "2048",
            "-validity",
            "10000",
            "-storepass",
            BENCH_KEY_PASS,
            "-keypass",
            BENCH_KEY_PASS,
            "-dname",
            "CN=Panadiga Bench,O=Panadiga Engineering,C=BR",
        ],
        cwd=ROOT,
        env=env or os.environ,
    )
    return keystore_path


def _run_tool_checked(cmd: list[str], env: Mapping[str, str], label: str) -> str:
    """Run an Android SDK verification tool and return stdout, raising on failure."""
    proc = subprocess.run(
        cmd,
        cwd=ROOT,
        env=dict(env),
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        detail = proc.stderr.strip() or proc.stdout.strip() or f"exit code {proc.returncode}"
        raise RuntimeError(f"APK verification step '{label}' failed: {detail}")
    return proc.stdout


def _parse_manifest_int(xmltree: str, attr_name: str) -> int:
    """Parse an integer attribute from `aapt2 dump xmltree` decimal or hex output."""
    pattern = rf"{re.escape(attr_name)}\([^)]*\)=([^\s]+)"
    match = re.search(pattern, xmltree)
    if not match:
        raise RuntimeError(f"AndroidManifest.xml missing attribute '{attr_name}'")
    raw = match.group(1)
    hex_tail = re.search(r"(0x[0-9a-fA-F]+)$", raw)
    return int(hex_tail.group(1), 16) if hex_tail else int(raw, 10)


def sign_and_align_apk(
    sdk_dir: Path,
    unsigned_apk: Path,
    output_apk: Path,
    keystore_path: Path = DEFAULT_BENCH_KEYSTORE,
    *,
    env: Mapping[str, str] | None = None,
) -> Path:
    """Zipalign an APK cleanly and sign with v1 + v2 + v3 schemes using the bench keystore."""
    if not unsigned_apk.is_file():
        raise FileNotFoundError(f"Source APK not found: {unsigned_apk}")
    run_env = env or os.environ
    zipalign = find_build_tool(sdk_dir, "zipalign")
    apksigner = find_build_tool(sdk_dir, "apksigner")
    ks = ensure_bench_keystore(keystore_path, run_env)

    output_apk.parent.mkdir(parents=True, exist_ok=True)
    aligned_tmp = output_apk.with_suffix(".aligned.tmp.apk")
    run_step([str(zipalign), "-p", "-f", "4", str(unsigned_apk), str(aligned_tmp)], cwd=ROOT, env=run_env)
    try:
        run_step(
            [
                str(apksigner),
                "sign",
                "--v1-signing-enabled",
                "true",
                "--v2-signing-enabled",
                "true",
                "--v3-signing-enabled",
                "true",
                "--ks",
                str(ks),
                "--ks-key-alias",
                BENCH_KEY_ALIAS,
                "--ks-pass",
                f"pass:{BENCH_KEY_PASS}",
                "--key-pass",
                f"pass:{BENCH_KEY_PASS}",
                "--out",
                str(output_apk),
                str(aligned_tmp),
            ],
            cwd=ROOT,
            env=run_env,
        )
    finally:
        aligned_tmp.unlink(missing_ok=True)
    return output_apk


def validate_apk_artifact(
    sdk_dir: Path,
    apk_path: Path,
    *,
    require_dual_abi: bool = True,
    env: Mapping[str, str] | None = None,
) -> dict[str, object]:
    """Validate APK structure, zipalign, v1/v2/v3 signatures, ABIs, and SDK metadata."""
    if not apk_path.is_file():
        raise FileNotFoundError(f"APK artifact not found: {apk_path}")
    size_bytes = apk_path.stat().st_size
    if size_bytes <= 1_000_000:
        raise RuntimeError(f"APK artifact unexpectedly small ({size_bytes} bytes): {apk_path}")

    required_entries = list(REQUIRED_APK_ENTRIES)
    if require_dual_abi:
        required_entries.append(DUAL_ABI_ENTRY)

    with zipfile.ZipFile(apk_path, "r") as zf:
        entries = {info.filename: info.file_size for info in zf.infolist()}
        for required in required_entries:
            if entries.get(required, 0) <= 0:
                raise RuntimeError(f"APK missing required non-empty ZIP entry: {required}")
        abis = sorted(
            name.split("/")[1]
            for name in entries
            if name.startswith("lib/") and name.endswith("/libpanadiga_lib.so")
        )

    run_env = env or os.environ
    zipalign = find_build_tool(sdk_dir, "zipalign")
    apksigner = find_build_tool(sdk_dir, "apksigner")
    aapt2 = find_build_tool(sdk_dir, "aapt2")

    _run_tool_checked([str(zipalign), "-c", "4", str(apk_path)], run_env, "zipalign -c 4")
    verify_out = _run_tool_checked(
        [str(apksigner), "verify", "-v", "--min-sdk-version", "21", str(apk_path)],
        run_env,
        "apksigner verify -v",
    )
    for marker in REQUIRED_SIGN_SCHEMES:
        if marker not in verify_out:
            raise RuntimeError(f"APK signature verification missing '{marker}'")

    xmltree = _run_tool_checked(
        [str(aapt2), "dump", "xmltree", str(apk_path), "--file", "AndroidManifest.xml"],
        run_env,
        "aapt2 dump xmltree",
    )
    codename_match = re.search(r'compileSdkVersionCodename(?:\([^)]*\))?="([^"]+)"', xmltree)
    if codename_match:
        codename = codename_match.group(1)
        if not (codename.isdigit() and int(codename) <= 16):
            raise RuntimeError(
                f"APK AndroidManifest.xml contains preview compileSdkVersionCodename='{codename}'."
            )

    min_sdk = _parse_manifest_int(xmltree, "minSdkVersion")
    target_sdk = _parse_manifest_int(xmltree, "targetSdkVersion")
    if min_sdk != 24:
        raise RuntimeError(f"Unexpected minSdkVersion={min_sdk} (expected 24)")
    if not (24 <= target_sdk <= 35):
        raise RuntimeError(f"Unexpected targetSdkVersion={target_sdk} (expected 24..35)")

    return {
        "apk": str(apk_path),
        "size_bytes": size_bytes,
        "abis": abis,
        "min_sdk": min_sdk,
        "target_sdk": target_sdk,
        "v1": True,
        "v2": True,
        "v3": True,
    }


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint for signing, aligning, and validating Android APKs."""
    parser = argparse.ArgumentParser(description="Sign, zipalign, and verify Panadiga Android APK.")
    parser.add_argument("--sign-and-verify", nargs=2, metavar=("INPUT_APK", "OUTPUT_APK"))
    parser.add_argument("--verify-only", metavar="APK_PATH")
    parser.add_argument("--single-abi", action="store_true")
    parser.add_argument("--keystore", default=str(DEFAULT_BENCH_KEYSTORE))
    args = parser.parse_args(argv)

    sdk_dir, _ = detect_android_sdk(os.environ)
    require_dual = not args.single_abi
    if args.sign_and_verify:
        src_apk, dst_apk = (Path(args.sign_and_verify[0]), Path(args.sign_and_verify[1]))
        sign_and_align_apk(sdk_dir, src_apk, dst_apk, Path(args.keystore), env=os.environ)
        summary = validate_apk_artifact(sdk_dir, dst_apk, require_dual_abi=require_dual, env=os.environ)
        print(f"VERIFIED SIGNED APK: {summary}")
        return 0
    if args.verify_only:
        summary = validate_apk_artifact(
            sdk_dir, Path(args.verify_only), require_dual_abi=require_dual, env=os.environ
        )
        print(f"VERIFIED APK: {summary}")
        return 0
    parser.error("Provide --sign-and-verify INPUT_APK OUTPUT_APK or --verify-only APK_PATH")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
