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
    "firmware/pico-p7501/common/p7501_spsc_ring.c",
    "firmware/pico-p7501/common/p7501_wdt_logic.c",
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
    "firmware/pico-p7501/flash/p7501_probe_cfg_codec.c",
    "firmware/pico-p7501/flash/p7501_qspi_map.c",
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
    "firmware/pico-p7501/tests/test_uart_codec.c",
    "firmware/pico-p7501/tests/test_pana_logic.c",
    "firmware/pico-p7501/tests/test_pana_diag_drv.c",
    "firmware/pico-p7501/tests/test_gdb_rsp_host.c",
)


def _compile_c11_host_test(
    exe_path: Path,
    work_dir: Path,
    env: Mapping[str, str],
    *,
    capture_output: bool = False,
) -> None:
    """Compile C11 Pico RSP host test suite into an isolated temp directory with strict warnings."""
    for compiler in ("gcc", "clang"):
        if shutil.which(compiler, path=env.get("PATH")):
            cmd = [
                compiler,
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-DPICO_HOST_TEST=1",
                "-I",
                "firmware/pico-p7501",
                *C11_HOST_SOURCES,
                "-o",
                str(exe_path),
            ]
            run_step(cmd, cwd=ROOT, env=env, capture_output=capture_output)
            return

    if sys.platform == "win32":
        msvc_env = load_msvc_env(env)
        if shutil.which("cl", path=msvc_env.get("PATH")):
            obj_dir = work_dir.as_posix().rstrip("/") + "/"
            run_step(
                [
                    "cl",
                    "/nologo",
                    "/MP",
                    "/std:c11",
                    "/W4",
                    "/WX",
                    "/DPICO_HOST_TEST=1",
                    "/D_CRT_SECURE_NO_WARNINGS",
                    "/Ifirmware/pico-p7501",
                    *C11_HOST_SOURCES,
                    f"/Fo:{obj_dir}",
                    f"/Fe:{exe_path}",
                ],
                cwd=ROOT,
                env=msvc_env,
                capture_output=capture_output,
            )
            return

    raise RuntimeError("No C11 compiler (gcc, clang, or MSVC cl) found for Pico firmware host tests.")


def _gate_c11_host(env: Mapping[str, str], *, capture_output: bool = False) -> None:
    """Compile & execute the C11 host test suite and optional RP2040/RP2350 ARM compilation."""
    if not capture_output:
        print_banner("8/9. Compiling & Executing C11 Pico Firmware Host Tests (+ ARM Check)")
    exe_name = "test_gdb_rsp_host.exe" if sys.platform == "win32" else "test_gdb_rsp_host"
    with tempfile.TemporaryDirectory(prefix="panadiga_c11_") as tmpdir_str:
        work_dir = Path(tmpdir_str)
        exe_path = work_dir / exe_name
        _compile_c11_host_test(exe_path, work_dir, env, capture_output=capture_output)
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
