"""C11 Pico firmware host test compiler and Gate 8 runner for Panadiga."""

from __future__ import annotations

from pathlib import Path
import shutil
import sys
import tempfile
from typing import Final, Mapping

from scripts.build_lanes.pico import compile_pico_boards_parallel, prepare_pico_env
from scripts.panadiga_ops import ROOT, load_msvc_env, print_banner, print_status, run_step

C11_HOST_SOURCES: Final[tuple[str, ...]] = (
    "firmware/pico-p7501/generated/p7501_ssot_tables.c",
    "firmware/pico-p7501/common/p7501_cmd_arena.c",
    "firmware/pico-p7501/common/p7501_hex.c",
    "firmware/pico-p7501/common/p7501_crc_csum.c",
    "firmware/pico-p7501/common/p7501_push_stream.c",
    "firmware/pico-p7501/common/p7501_spsc_ring.c",
    "firmware/pico-p7501/common/p7501_wdt_logic.c",
    "firmware/pico-p7501/crypto/p7501_feistel.c",
    "firmware/pico-p7501/crypto/p7501_compress.c",
    "firmware/pico-p7501/crypto/p7501_decomp.c",
    "firmware/pico-p7501/hal/p7501_bootsel_logic.c",
    "firmware/pico-p7501/hal/p7501_led_logic.c",
    "firmware/pico-p7501/hal/p7501_wireless_codec.c",
    "firmware/pico-p7501/gdb/gdb_rsp_codec.c",
    "firmware/pico-p7501/gdb/gdb_rsp_memory.c",
    "firmware/pico-p7501/gdb/gdb_rsp_cpu.c",
    "firmware/pico-p7501/uart/p7501_uart_autobaud.c",
    "firmware/pico-p7501/uart/p7501_uart_codec.c",
    "firmware/pico-p7501/uart/p7501_uart_args.c",
    "firmware/pico-p7501/uart/p7501_fmu_codec.c",
    "firmware/pico-p7501/flash/p7501_blackbox_codec.c",
    "firmware/pico-p7501/flash/p7501_bus_policy.c",
    "firmware/pico-p7501/flash/p7501_flash_geo.c",
    "firmware/pico-p7501/flash/p7501_fast_stream_codec.c",
    "firmware/pico-p7501/flash/p7501_normap_codec.c",
    "firmware/pico-p7501/flash/p7501_ftl_inspect.c",
    "firmware/pico-p7501/flash/p7501_job_codec.c",
    "firmware/pico-p7501/flash/p7501_payloads.c",
    "firmware/pico-p7501/flash/p7501_trap_guard.c",
    "firmware/pico-p7501/flash/p7501_probe_cfg_codec.c",
    "firmware/pico-p7501/flash/p7501_qspi_map.c",
    "firmware/pico-p7501/flash/p7501_sector_ops.c",
    "firmware/pico-p7501/flash/p7501_vault_codec.c",
    "firmware/pico-p7501/flash/p7501_target_bus.c",
    "firmware/pico-p7501/uart/p7501_pana_dumpnor.c",
    "firmware/pico-p7501/uart/p7501_pana_dumpnor_verify.c",
    "firmware/pico-p7501/uart/p7501_pana_jedec_stubs.c",
    "firmware/pico-p7501/uart/p7501_pana_jedec_exec.c",
    "firmware/pico-p7501/uart/p7501_pana_jedec_id.c",
    "firmware/pico-p7501/uart/p7501_pana_rsp_codec.c",
    "firmware/pico-p7501/uart/p7501_pana_rsp_guard.c",
    "firmware/pico-p7501/uart/p7501_pana_rsp_parse.c",
    "firmware/pico-p7501/uart/p7501_pana_rsp_tokens.c",
    "firmware/pico-p7501/uart/p7501_diag_watch_codec.c",
    "firmware/pico-p7501/uart/p7501_diag_watch_events.c",
    "firmware/pico-p7501/uart/p7501_drv_pana_codec.c",
    "firmware/pico-p7501/diag/p7501_i2c_codec.c",
    "firmware/pico-p7501/diag/p7501_identify_codec.c",
    "firmware/pico-p7501/diag/p7501_mmio_snapshot_codec.c",
    "firmware/pico-p7501/diag/p7501_piologic_codec.c",
    "firmware/pico-p7501/diag/p7501_pwrosc_codec.c",
    "firmware/pico-p7501/diag/p7501_sds_codec.c",
    "firmware/pico-p7501/diag/p7501_jigsweep_codec.c",
    "firmware/pico-p7501/diag/p7501_ir_codec.c",
    "firmware/pico-p7501/diag/p7501_serv_macro_codec.c",
    "firmware/pico-p7501/diag/p7501_tbus_codec.c",
    "firmware/pico-p7501/diag/p7501_boottrace_codec.c",
    "firmware/pico-p7501/diag/p7501_hotplug_codec.c",
    "firmware/pico-p7501/diag/p7501_bitwise_search.c",
    "firmware/pico-p7501/diag/p7501_hdd_unlock.c",
    "firmware/pico-p7501/diag/p7501_math_gps.c",
    "firmware/pico-p7501/diag/p7501_math_stats.c",
    "firmware/pico-p7501/diag/p7501_meihfs_probe.c",
    "firmware/pico-p7501/diag/p7501_microsurgery.c",
    "firmware/pico-p7501/diag/p7501_mn103_asm.c",
    "firmware/pico-p7501/diag/p7501_mn103_disasm.c",
    "firmware/pico-p7501/diag/p7501_pieos_xray.c",
    "firmware/pico-p7501/tests/test_uart_codec.c",
    "firmware/pico-p7501/tests/test_pana_logic.c",
    "firmware/pico-p7501/tests/test_pana_diag_drv.c",
    "firmware/pico-p7501/tests/test_pico_autonomous.c",
    "firmware/pico-p7501/tests/test_gdb_rsp_host.c",
)

OPENOS_HOST_SOURCES: tuple[str, ...] = (
    "firmware/panadiga-openos/drivers/uart_sc0.c",
    "firmware/panadiga-openos/drivers/tbus_ic7501.c",
    "firmware/panadiga-openos/drivers/aiof_video.c",
    "firmware/panadiga-openos/drivers/avdd_iic_video.c",
    "firmware/panadiga-openos/drivers/davis_sata.c",
    "firmware/panadiga-openos/drivers/storage_provisioner.c",
    "firmware/panadiga-openos/kernel/openos_main.c",
    "firmware/panadiga-openos/tests/test_openos_host.c",
)

C11_HW_HOST_SOURCES: Final[tuple[str, ...]] = (
    "firmware/pico-p7501/diag/p7501_hotplug_codec.c",
    "firmware/pico-p7501/hal/p7501_hw.c",
    "firmware/pico-p7501/tests/test_p7501_hw_host.c",
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
    """Compile C11 Pico RSP host test suite into an isolated temp directory with strict warnings."""
    _compile_c11_target(
        exe_path,
        work_dir,
        env,
        sources=C11_HOST_SOURCES,
        include_dir="firmware/pico-p7501",
        define_macro="PICO_HOST_TEST",
        capture_output=capture_output,
    )


def _gate_c11_host(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    """Compile & execute the C11 host test suites and optional RP2040/RP2350 ARM compilation."""
    if not capture_output:
        print_banner("8/9. Compiling & Executing C11 Pico & OpenOS Host Tests (+ ARM Check)")
    ext = ".exe" if sys.platform == "win32" else ""
    targets: tuple[tuple[str, tuple[str, ...], str, tuple[str, ...], str], ...] = (
        ("test_gdb_rsp_host", C11_HOST_SOURCES, "firmware/pico-p7501", (), "PICO_HOST_TEST"),
        (
            "test_p7501_hw_host",
            C11_HW_HOST_SOURCES,
            "firmware/pico-p7501",
            ("firmware/pico-p7501/tests/mock_pico_sdk",),
            "PICO_HOST_TEST",
        ),
        ("test_openos_host", OPENOS_HOST_SOURCES, "firmware/panadiga-openos", (), "PANADIGA_OPENOS_HOST_TEST"),
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
        print_status("PASS", "All C11 Pico firmware host tests (+ ARM compile check) passed!", "32")
