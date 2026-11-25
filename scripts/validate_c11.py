"""C11 PicoFW and DigaOS firmware host test compiler and Gate 8 runner for Panadiga."""

from __future__ import annotations

from pathlib import Path
import shutil
import sys
import tempfile
from typing import Final, Mapping

from scripts.build_lanes.pico import compile_pico_boards_parallel, prepare_pico_env
from scripts.panadiga_ops import ROOT, load_msvc_env, print_banner, print_status, run_step

C11_HOST_SOURCES: Final[tuple[str, ...]] = (
    "firmware/picofw/generated/picofw_ssot_tables.c",
    "firmware/picofw/common/picofw_cmd_arena.c",
    "firmware/picofw/common/picofw_hex.c",
    "firmware/picofw/common/picofw_crc_csum.c",
    "firmware/picofw/common/picofw_probe_fsm.c",
    "firmware/picofw/common/picofw_push_stream.c",
    "firmware/picofw/common/picofw_spsc_ring.c",
    "firmware/picofw/common/picofw_wdt_logic.c",
    "firmware/picofw/crypto/picofw_feistel.c",
    "firmware/picofw/crypto/picofw_compress.c",
    "firmware/picofw/crypto/picofw_decomp.c",
    "firmware/picofw/hal/picofw_bootsel_logic.c",
    "firmware/picofw/hal/picofw_led_logic.c",
    "firmware/picofw/hal/picofw_wireless_codec.c",
    "firmware/picofw/gdb/gdb_rsp_codec.c",
    "firmware/picofw/gdb/gdb_rsp_memory.c",
    "firmware/picofw/gdb/gdb_rsp_cpu.c",
    "firmware/picofw/uart/picofw_uart_autobaud.c",
    "firmware/picofw/uart/picofw_uart_codec.c",
    "firmware/picofw/uart/picofw_uart_args.c",
    "firmware/picofw/uart/picofw_fmu_codec.c",
    "firmware/picofw/flash/picofw_blackbox_codec.c",
    "firmware/picofw/flash/picofw_bus_policy.c",
    "firmware/picofw/flash/picofw_bus_tx_guard.c",
    "firmware/picofw/flash/picofw_flash_geo.c",
    "firmware/picofw/flash/picofw_fast_stream_codec.c",
    "firmware/picofw/flash/picofw_normap_codec.c",
    "firmware/picofw/flash/picofw_ftl_inspect.c",
    "firmware/picofw/flash/picofw_job_codec.c",
    "firmware/picofw/flash/picofw_payloads.c",
    "firmware/picofw/flash/picofw_trap_guard.c",
    "firmware/picofw/flash/picofw_probe_cfg_codec.c",
    "firmware/picofw/flash/picofw_qspi_map.c",
    "firmware/picofw/flash/picofw_sector_ops.c",
    "firmware/picofw/flash/picofw_vault_codec.c",
    "firmware/picofw/flash/picofw_target_bus.c",
    "firmware/picofw/uart/picofw_pana_dumpnor.c",
    "firmware/picofw/uart/picofw_pana_dumpnor_verify.c",
    "firmware/picofw/uart/picofw_pana_jedec_stubs.c",
    "firmware/picofw/uart/picofw_pana_jedec_exec.c",
    "firmware/picofw/uart/picofw_pana_jedec_id.c",
    "firmware/picofw/uart/picofw_pana_rsp_codec.c",
    "firmware/picofw/uart/picofw_pana_rsp_guard.c",
    "firmware/picofw/uart/picofw_pana_rsp_parse.c",
    "firmware/picofw/uart/picofw_pana_rsp_tokens.c",
    "firmware/picofw/uart/picofw_diag_watch_codec.c",
    "firmware/picofw/uart/picofw_diag_watch_events.c",
    "firmware/picofw/uart/picofw_drv_pana_codec.c",
    "firmware/picofw/diag/picofw_i2c_codec.c",
    "firmware/picofw/diag/picofw_identify_codec.c",
    "firmware/picofw/diag/picofw_mmio_snapshot_codec.c",
    "firmware/picofw/diag/picofw_piologic_codec.c",
    "firmware/picofw/diag/picofw_pwrosc_codec.c",
    "firmware/picofw/diag/picofw_sds_codec.c",
    "firmware/picofw/diag/picofw_jigsweep_codec.c",
    "firmware/picofw/diag/picofw_ir_codec.c",
    "firmware/picofw/diag/picofw_serv_macro_codec.c",
    "firmware/picofw/diag/picofw_tbus_codec.c",
    "firmware/picofw/diag/picofw_boottrace_codec.c",
    "firmware/picofw/diag/picofw_hotplug_codec.c",
    "firmware/picofw/diag/picofw_bitwise_search.c",
    "firmware/picofw/diag/picofw_hdd_unlock.c",
    "firmware/picofw/diag/picofw_math_gps.c",
    "firmware/picofw/diag/picofw_math_stats.c",
    "firmware/picofw/diag/picofw_meihfs_probe.c",
    "firmware/picofw/diag/picofw_microsurgery.c",
    "firmware/picofw/diag/picofw_mn103_asm.c",
    "firmware/picofw/diag/picofw_mn103_disasm.c",
    "firmware/picofw/diag/picofw_pieos_xray.c",
    "firmware/picofw/tests/test_uart_codec.c",
    "firmware/picofw/tests/test_pana_logic.c",
    "firmware/picofw/tests/test_pana_diag_drv.c",
    "firmware/picofw/tests/test_pico_autonomous.c",
    "firmware/picofw/tests/test_gdb_rsp_host.c",
)


def _discover_digaos_host_sources(root: Path = ROOT) -> tuple[str, ...]:
    """Collect all host-testable DigaOS C11 sources (generated SSOT, multi-SoC HAL, drivers, kernel, test entry)."""
    digaos_dir = root / "firmware" / "digaos"
    if not digaos_dir.is_dir():
        return ()
    sources: list[str] = []
    for subdir in ("generated", "hal", "drivers", "kernel"):
        sub_path = digaos_dir / subdir
        if sub_path.is_dir():
            sources.extend(
                p.relative_to(root).as_posix()
                for p in sorted(sub_path.glob("*.c"))
            )
    test_entry = digaos_dir / "tests" / "test_digaos_host.c"
    if test_entry.is_file():
        sources.append(test_entry.relative_to(root).as_posix())
    return tuple(sources)


DIGAOS_HOST_SOURCES: Final[tuple[str, ...]] = _discover_digaos_host_sources()

C11_HW_HOST_SOURCES: Final[tuple[str, ...]] = (
    "firmware/picofw/common/picofw_probe_fsm.c",
    "firmware/picofw/diag/picofw_hotplug_codec.c",
    "firmware/picofw/hal/picofw_hw.c",
    "firmware/picofw/tests/test_picofw_hw_host.c",
)


def _compile_c11_target(
    exe_path: Path,
    work_dir: Path,
    env: Mapping[str, str],
    *,
    sources: tuple[str, ...],
    include_dir: str,
    define_macro: str,
    extra_include_dirs: tuple[str, ...] = (),
    capture_output: bool = False,
) -> None:
    """Compile a C11 host test target into an isolated temp directory with strict warnings."""
    all_includes = (include_dir, *extra_include_dirs)
    for compiler in ("gcc", "clang"):
        if shutil.which(compiler, path=env.get("PATH")):
            inc_flags = [arg for inc in all_includes for arg in ("-I", inc)]
            cmd = [
                compiler,
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                f"-D{define_macro}=1",
                *inc_flags,
                *sources,
                "-o",
                str(exe_path),
            ]
            run_step(cmd, cwd=ROOT, env=env, capture_output=capture_output)
            return

    if sys.platform == "win32":
        msvc_env = load_msvc_env(env)
        if shutil.which("cl", path=msvc_env.get("PATH")):
            obj_dir = work_dir.as_posix().rstrip("/") + "/"
            inc_flags = [f"/I{inc}" for inc in all_includes]
            run_step(
                [
                    "cl",
                    "/nologo",
                    "/MP",
                    "/std:c11",
                    "/W4",
                    "/WX",
                    f"/D{define_macro}=1",
                    "/D_CRT_SECURE_NO_WARNINGS",
                    *inc_flags,
                    *sources,
                    f"/Fo:{obj_dir}",
                    f"/Fe:{exe_path}",
                ],
                cwd=ROOT,
                env=msvc_env,
                capture_output=capture_output,
            )
            return

    raise RuntimeError(f"No C11 compiler (gcc, clang, or MSVC cl) found for {define_macro} host tests.")


def _compile_c11_host_test(
    exe_path: Path,
    work_dir: Path,
    env: Mapping[str, str],
    *,
    capture_output: bool = False,
) -> None:
    """Compile C11 PicoFW RSP host test suite into an isolated temp directory with strict warnings."""
    _compile_c11_target(
        exe_path,
        work_dir,
        env,
        sources=C11_HOST_SOURCES,
        include_dir="firmware/picofw",
        define_macro="PICO_HOST_TEST",
        capture_output=capture_output,
    )


def _gate_c11_host(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    """Compile & execute the C11 host test suites and optional RP2040/RP2350 ARM compilation."""
    if not capture_output:
        print_banner("8/9. Compiling & Executing C11 PicoFW & DigaOS Host Tests (+ ARM Check)")
    digaos_sources = _discover_digaos_host_sources(ROOT) or DIGAOS_HOST_SOURCES
    if not digaos_sources:
        raise FileNotFoundError("Missing firmware/digaos C11 host sources")
    ext = ".exe" if sys.platform == "win32" else ""
    targets: tuple[tuple[str, tuple[str, ...], str, tuple[str, ...], str], ...] = (
        ("test_gdb_rsp_host", C11_HOST_SOURCES, "firmware/picofw", (), "PICO_HOST_TEST"),
        (
            "test_picofw_hw_host",
            C11_HW_HOST_SOURCES,
            "firmware/picofw",
            ("firmware/picofw/tests/mock_pico_sdk",),
            "PICO_HOST_TEST",
        ),
        ("test_digaos_host", digaos_sources, "firmware/digaos", (), "PANADIGA_DIGAOS_HOST_TEST"),
    )
    with tempfile.TemporaryDirectory(prefix="panadiga_c11_") as tmpdir_str:
        base_dir = Path(tmpdir_str)
        for name, sources, include_dir, extra_includes, macro in targets:
            work_dir = base_dir / name
            work_dir.mkdir(parents=True, exist_ok=True)
            exe_path = work_dir / f"{name}{ext}"
            _compile_c11_target(
                exe_path,
                work_dir,
                env,
                sources=sources,
                include_dir=include_dir,
                define_macro=macro,
                extra_include_dirs=extra_includes,
                capture_output=capture_output,
            )
            run_step([str(exe_path)], cwd=ROOT, env=env, capture_output=capture_output)
    pico_env = prepare_pico_env(env)
    arm_gcc = shutil.which("arm-none-eabi-gcc", path=pico_env.get("PATH"))
    sdk_path_str = pico_env.get("PICO_SDK_PATH", "")
    if arm_gcc is not None and bool(sdk_path_str) and Path(sdk_path_str).is_dir():
        compile_pico_boards_parallel(pico_env)
    elif not capture_output:
        print_status(
            "SKIP",
            "ARM GCC or Pico SDK not found; skipping RP2040/RP2350 ARM compile in Gate 8.",
            "33",
        )
    if not capture_output:
        print_status("PASS", "All C11 PicoFW & DigaOS firmware host tests (+ ARM compile check) passed!", "32")
