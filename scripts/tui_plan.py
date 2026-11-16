"""Build DAG planning and inventory/cache telemetry formatting for the Panadiga TUI."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Mapping

from scripts.panadiga_cache import (
    WorkspaceInventory,
    frontend_digest_and_outputs,
    ghidra_digest_and_outputs,
    inspect_cache_entries,
    is_step_fresh,
    pico_digest_and_outputs,
    query_compiler_cache_stats,
)


@dataclass
class PipelineSelection:
    """User-selected targets and options for DAG execution."""

    ci: bool = False
    frontend: bool = True
    pico: bool = False
    desktop: bool = True
    android: bool = False
    use_cache: bool = True
    run_after: bool = False


@dataclass(frozen=True)
class DagWave:
    """Single dependency-ordered execution stage displayed before running."""

    title: str
    concurrency: str
    steps: tuple[str, ...]


def default_all_selection(inventory: WorkspaceInventory, *, include_ci: bool = True) -> PipelineSelection:
    """Construct full build selection respecting discovered cross-platform toolchains."""
    has_pico = all(k in inventory.tools for k in ("arm-none-eabi-gcc", "pico_sdk", "cmake"))
    has_android = all(k in inventory.tools for k in ("android_sdk", "android_ndk"))
    return PipelineSelection(
        ci=include_ci,
        frontend=True,
        pico=has_pico,
        desktop=True,
        android=has_android,
        use_cache=True,
        run_after=False,
    )


def _format_bytes(num_bytes: int) -> str:
    value = float(num_bytes)
    for unit in ("B", "KiB", "MiB", "GiB"):
        if value < 1024.0 or unit == "GiB":
            return f"{int(value)} {unit}" if unit == "B" else f"{value:.1f} {unit}"
        value /= 1024.0
    return f"{num_bytes} B"


def _step_cache_tag(
    step: str,
    selection: PipelineSelection,
    inventory: WorkspaceInventory,
    env: Mapping[str, str],
) -> str:
    if not selection.use_cache:
        return "[CACHE OFF]"
    if step == "frontend":
        digest, outs = frontend_digest_and_outputs(inventory.root)
        return "[CACHE HIT]" if is_step_fresh(inventory.root, "frontend", digest, outs) else "[BUILD]"
    if step == "pico":
        digest, outs = pico_digest_and_outputs(inventory.root, env)
        return "[CACHE HIT]" if is_step_fresh(inventory.root, "pico", digest, outs) else "[BUILD]"
    if step == "ghidra":
        digest, outs = ghidra_digest_and_outputs(inventory.root)
        return "[CACHE HIT]" if is_step_fresh(inventory.root, "ghidra", digest, outs) else "[EXPORT]"
    return "[INCREMENTAL]"


def build_execution_plan(
    selection: PipelineSelection,
    inventory: WorkspaceInventory,
    env: Mapping[str, str],
) -> list[DagWave]:
    """Compute dependency-ordered parallel/serial DAG waves for display and execution."""
    waves: list[DagWave] = []
    need_frontend = selection.frontend or selection.desktop or selection.android

    wave1_steps: list[str] = []
    if need_frontend:
        wave1_steps.append(f"Frontend Vite UI {_step_cache_tag('frontend', selection, inventory, env)}")
    if selection.pico:
        wave1_steps.append(
            f"Pico RP2040 + RP2350 (CMake/Ninja 2x paralelo) {_step_cache_tag('pico', selection, inventory, env)}"
        )
    if selection.android:
        wave1_steps.append("Android Prep (rustup target + local.properties)")
    if wave1_steps:
        mode = "PARALELO" if len(wave1_steps) > 1 or selection.pico else "DIRETO"
        waves.append(DagWave("Wave 1 - Preparo & Frontends Independentes", mode, tuple(wave1_steps)))

    if selection.pico:
        waves.append(
            DagWave(
                "Wave 2 - Barreira Serial Pico UF2",
                "SERIAL",
                ("Merge Unified UF2 + zero-touch sync (firmware_bins/)",),
            )
        )

    if selection.ci:
        waves.append(
            DagWave(
                "Wave 3 - Pipeline CI (9 Quality Gates)",
                "PARALELO (9 lanes + ProcessTracker)",
                ("Rust/Ghidra + TS + Vitest + jscpd + uxlint + Playwright + C11 + Mypy/Pytest",),
            )
        )

    if selection.desktop and selection.android:
        ghidra_tag = _step_cache_tag("ghidra", selection, inventory, env)
        waves.append(
            DagWave(
                "Wave 4 - Desktop Release + Android ARM64 APK",
                "PARALELO (target/ || target/android-cargo/)",
                (
                    f"Desktop: cargo build --release -p panadiga-cli -p panadiga-desktop + Ghidra {ghidra_tag}",
                    "Android: cargo aarch64 + Gradle APK + zipalign + apksigner",
                ),
            )
        )
    elif selection.desktop:
        ghidra_tag = _step_cache_tag("ghidra", selection, inventory, env)
        waves.append(
            DagWave(
                "Wave 4 - Desktop Release Packaging",
                "PARALELO (1 DAG Cargo unificado)",
                (
                    "cargo build --release -p panadiga-cli -p panadiga-desktop",
                    f"Ghidra SSOT {ghidra_tag} + Staging dist/ + Archive + SHA256SUMS.txt",
                ),
            )
        )
    elif selection.android:
        waves.append(
            DagWave(
                "Wave 4 - Android ARM64 APK",
                "DIRETO (target/android-cargo/)",
                ("tauri android build --apk --target aarch64 + zipalign + apksigner",),
            )
        )

    if selection.run_after and selection.desktop:
        waves.append(DagWave("Wave 6 - Launch", "DETACHED", ("Iniciar Panadiga Desktop GUI",)))

    return waves


def render_inventory_and_cache_report(root: Path, inventory: WorkspaceInventory, env: Mapping[str, str]) -> str:
    """Format recursive workspace discovery and cache metrics into readable lines."""
    lines = [
        "=" * 74,
        f"  PANADIGA RECURSIVE WORKSPACE & CACHE ({inventory.os_name} {inventory.arch} | {inventory.cpu_count} CPUs)",
        f"  Root: {root}",
        "=" * 74,
        f"  Crates ({len(inventory.crates)}): {', '.join(inventory.crates) or '-'}",
        f"  Apps   ({len(inventory.apps)}): {', '.join(inventory.apps) or '-'}",
        f"  FW     ({len(inventory.firmware_targets)}): {', '.join(inventory.firmware_targets) or '-'}",
        "-" * 74,
        "  Toolchains & Aceleradores:",
    ]
    for key in (
        "cargo",
        "rustc",
        "node",
        "npm",
        "arm-none-eabi-gcc",
        "pico_sdk",
        "cmake",
        "ninja",
        "c11_host",
        "android_sdk",
        "android_ndk",
        "sccache",
        "ccache",
    ):
        status = inventory.tools.get(key, "N/D")
        lines.append(f"    - {key:<18}: {status}")

    lines.append("-" * 74)
    lines.append("  Caches Locais:")
    for entry in inspect_cache_entries(root):
        state = _format_bytes(entry.bytes_total) if entry.exists else "vazio"
        lines.append(f"    - {entry.label:<22}: {state:>10}  ({entry.path})")

    cc_stats = query_compiler_cache_stats(env)
    if cc_stats:
        lines.append("-" * 74)
        for st in cc_stats:
            lines.append(f"  {st}")
    lines.append("=" * 74)
    return "\n".join(lines)
