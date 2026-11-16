"""Interactive cross-platform terminal UI rendering and keyboard loop for Panadiga."""

from __future__ import annotations

import os
import sys
from typing import Callable, Mapping, TextIO, cast

from scripts.panadiga_cache import WorkspaceInventory, clear_caches
from scripts.tui_plan import (
    PipelineSelection,
    build_execution_plan,
    default_all_selection,
    render_inventory_and_cache_report,
)


def _enable_ansi_console() -> None:
    if hasattr(sys.stdout, "reconfigure"):
        try:
            sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        except OSError:
            pass
    if sys.platform != "win32":
        return
    try:
        import ctypes

        kernel32 = ctypes.windll.kernel32
        handle = kernel32.GetStdHandle(-11)
        mode = ctypes.c_uint32()
        if kernel32.GetConsoleMode(handle, ctypes.byref(mode)):
            kernel32.SetConsoleMode(handle, mode.value | 0x0004)
    except Exception:
        pass


def _select_readable(timeout_s: float) -> list[TextIO]:
    import select

    readable, _, _ = select.select([sys.stdin], [], [], timeout_s)
    return cast(list[TextIO], readable)


def _read_keypress() -> str:
    """Read normalized navigation key cross-platform without third-party dependencies."""
    if sys.platform == "win32":
        import msvcrt

        getwch: Callable[[], str] = getattr(msvcrt, "getwch")
        ch = getwch()
        if ch in ("\x00", "\xe0"):
            ext = getwch()
            return {"H": "UP", "P": "DOWN", "K": "LEFT", "M": "RIGHT"}.get(ext, "")
        if ch in ("\r", "\n"):
            return "ENTER"
        if ch == " ":
            return "SPACE"
        if ch == "\x03":
            raise KeyboardInterrupt
        if ch == "\x1b":
            return "ESC"
        return ch.lower()

    import termios
    import tty

    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        ch = sys.stdin.read(1)
        if ch == "\x03":
            raise KeyboardInterrupt
        if ch == "\x1b":
            next_ch = sys.stdin.read(1) if sys.stdin in _select_readable(0.04) else ""
            if next_ch == "[":
                code = sys.stdin.read(1)
                return {"A": "UP", "B": "DOWN", "C": "RIGHT", "D": "LEFT"}.get(code, "")
            return "ESC"
        if ch in ("\r", "\n"):
            return "ENTER"
        if ch == " ":
            return "SPACE"
        return ch.lower()
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)


def _toggle_row(sel: PipelineSelection, row_idx: int) -> None:
    attrs = ("ci", "frontend", "pico", "desktop", "android", "use_cache", "run_after")
    if 0 <= row_idx < len(attrs):
        name = attrs[row_idx]
        setattr(sel, name, not getattr(sel, name))


def _render_tui_screen(
    sel: PipelineSelection,
    cursor: int,
    inventory: WorkspaceInventory,
    env: Mapping[str, str],
    notice: str,
) -> None:
    has_pico = all(k in inventory.tools for k in ("arm-none-eabi-gcc", "pico_sdk", "cmake"))
    has_android = all(k in inventory.tools for k in ("android_sdk", "android_ndk"))
    accel = []
    if "sccache" in inventory.tools:
        accel.append("sccache")
    if "ccache" in inventory.tools:
        accel.append("ccache")
    accel_str = ", ".join(accel) if accel else "hash steps.json + Cargo/Vite/Gradle"

    rows: tuple[tuple[str, bool, str], ...] = (
        ("Pipeline CI (9 Quality Gates em paralelo)", sel.ci, ""),
        ("Frontend Desktop UI (React 19 + Vite)", sel.frontend or sel.desktop or sel.android, ""),
        ("Pico Firmware (RP2040 + RP2350 + Unified UF2)", sel.pico, "" if has_pico else " [ARM GCC/SDK ausente]"),
        ("Desktop Release (panadiga CLI + Tauri GUI + dist/)", sel.desktop, ""),
        ("Android ARM64 APK (aarch64-linux-android)", sel.android, "" if has_android else " [SDK/NDK ausente]"),
        ("Usar Cache de Build (hash + sccache/ccache)", sel.use_cache, ""),
        ("Abrir Desktop GUI ao finalizar", sel.run_after, ""),
    )

    waves = build_execution_plan(sel, inventory, env)
    sys.stdout.write("\033[H\033[J")
    print("=" * 78)
    print(f"  PANADIGA INTERACTIVE TUI BUILD & CI  |  {inventory.os_name} {inventory.arch} ({inventory.cpu_count} cores)")
    print(f"  Root: {inventory.root}")
    print(f"  Cache/Accel: {accel_str}")
    print("=" * 78)
    print("  Selecione os alvos de execucao:")
    for idx, (label, checked, suffix) in enumerate(rows):
        pointer = ">" if idx == cursor else " "
        mark = "x" if checked else " "
        print(f"   {pointer} [{mark}] {idx + 1}. {label}{suffix}")

    print("-" * 78)
    print("  Plano DAG (Paralelo quando possivel, Serial quando necessario):")
    if not waves:
        print("    (Nenhum alvo marcado)")
    else:
        for w in waves:
            print(f"    * {w.title}  [{w.concurrency}]")
            for step in w.steps:
                print(f"        -> {step}")

    print("-" * 78)
    print("  Atalhos: Up/Down mover | Espaco/1-7 marcar | [a] Tudo+CI | [b] Build Tudo")
    print("           [d] Desktop  | [v] So CI | [c] Caches | [Enter] Executar | [q] Sair")
    if notice:
        print(f"  [!] {notice}")
    sys.stdout.flush()


def _handle_cache_submenu(inventory: WorkspaceInventory, env: Mapping[str, str]) -> str:
    sys.stdout.write("\033[H\033[J")
    print(render_inventory_and_cache_report(inventory.root, inventory, env))
    print("  [l] Limpar cache incremental Panadiga   [x] Limpar TUDO (+ Cargo target)")
    print("  [qualquer outra tecla] Voltar ao menu principal")
    sys.stdout.flush()
    key = _read_keypress()
    if key == "l":
        removed = clear_caches(inventory.root, include_cargo=False)
        return f"Limpo cache Panadiga ({len(removed)} diretorios)."
    if key == "x":
        removed = clear_caches(inventory.root, include_cargo=True)
        return f"Limpo cache completo incluindo Cargo target ({len(removed)} diretorios)."
    return ""


def _fallback_numeric_prompt(inventory: WorkspaceInventory, env: Mapping[str, str]) -> PipelineSelection | None:
    print(render_inventory_and_cache_report(inventory.root, inventory, env))
    print("Menu rapido:")
    print("  1) Pipeline Completo (CI + Pico* + Frontend + Desktop + Android*)")
    print("  2) Build Tudo (sem CI)")
    print("  3) Desktop Release (Frontend + CLI + Desktop)")
    print("  4) Somente CI (9 Quality Gates em paralelo)")
    print("  5) Somente Pico Firmware")
    print("  6) Limpar Cache Panadiga")
    print("  q) Sair")
    choice = input("Escolha [1-6/q]: ").strip().lower()
    if choice == "1":
        return default_all_selection(inventory, include_ci=True)
    if choice == "2":
        return default_all_selection(inventory, include_ci=False)
    if choice == "3":
        return PipelineSelection(frontend=True, desktop=True)
    if choice == "4":
        return PipelineSelection(ci=True, frontend=False, desktop=False)
    if choice == "5":
        return PipelineSelection(frontend=False, pico=True, desktop=False)
    if choice == "6":
        clear_caches(inventory.root, include_cargo=False)
        print("[OK] Caches Panadiga removidos.")
        return None
    return None


def launch_tui(inventory: WorkspaceInventory, env: Mapping[str, str]) -> PipelineSelection | None:
    """Run interactive terminal selector returning chosen targets, or None on exit."""
    _enable_ansi_console()
    if not (sys.stdin.isatty() and sys.stdout.isatty()):
        return _fallback_numeric_prompt(inventory, env)

    sel = default_all_selection(inventory, include_ci=True)
    cursor = 0
    notice = ""
    row_count = 7

    while True:
        _render_tui_screen(sel, cursor, inventory, env, notice)
        notice = ""
        try:
            key = _read_keypress()
        except (OSError, ImportError):
            return _fallback_numeric_prompt(inventory, env)

        if key in ("UP", "k"):
            cursor = (cursor - 1) % row_count
        elif key in ("DOWN", "j"):
            cursor = (cursor + 1) % row_count
        elif key == "SPACE":
            _toggle_row(sel, cursor)
        elif key in ("1", "2", "3", "4", "5", "6", "7"):
            idx = int(key) - 1
            cursor = idx
            _toggle_row(sel, idx)
        elif key == "a":
            sel = default_all_selection(inventory, include_ci=True)
            notice = "Preset selecionado: Pipeline Completo (CI + Build Tudo)."
        elif key == "b":
            sel = default_all_selection(inventory, include_ci=False)
            notice = "Preset selecionado: Build Tudo (sem CI)."
        elif key == "d":
            sel = PipelineSelection(ci=False, frontend=True, desktop=True, use_cache=sel.use_cache)
            notice = "Preset selecionado: Desktop Release Rapido."
        elif key == "v":
            sel = PipelineSelection(ci=True, frontend=False, desktop=False, use_cache=sel.use_cache)
            notice = "Preset selecionado: Somente Quality Gates CI."
        elif key == "c":
            notice = _handle_cache_submenu(inventory, env)
        elif key in ("q", "ESC"):
            print("\nCancelado pelo usuario.")
            return None
        elif key == "ENTER":
            if not (sel.ci or sel.frontend or sel.pico or sel.desktop or sel.android):
                notice = "Marque ao menos um alvo antes de executar!"
                continue
            print()
            return sel
        else:
            if key == "" and os.name == "nt":
                continue
            notice = f"Tecla '{key}' sem acao. Use Setas, Espaco, Enter ou q."
