"""Thin typed Python facade delegating A/B cold-boot harness classification & CSV to `panadiga pico ab-boot`."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import json
import time
from typing import Sequence

from scripts.bench.protocol import BenchSession, TransportProtocol
from scripts.panadiga_cli_bridge import (
    JsonDict,
    PanadigaCliError,
    as_dict,
    as_list,
    opt_float,
    opt_int,
    opt_str,
    req_bool,
    req_int,
    req_str,
    run_panadiga_json,
)


@dataclass(frozen=True)
class AbBootRun:
    """Single cold-boot trial outcome decoded from AM34 SC0 UART capture."""

    run_index: int
    mode: str
    command: str
    ddr_check_ok: bool
    boot_mode: str | None
    bgo_failed_code: int | None
    sd_91006: bool
    sd_90002: bool
    sd_90001: bool
    odd_83a7: bool
    duration_ms: float
    sd_hw_ok: bool = False
    sd_diag_load_fail: bool = False
    sd_uncompress_fail: bool = False
    odd_tray_fail: bool = False
    diagnosis: str = ""


@dataclass(frozen=True)
class AbModeStats:
    """Aggregated error/mode frequencies for one arm of the A/B experiment."""

    mode: str
    runs: int
    ddr_ok_count: int
    sd_91006_count: int
    sd_90002_count: int
    sd_90001_count: int
    odd_83a7_count: int
    bgo_fail_count: int
    sd_hw_ok_count: int = 0
    sd_diag_load_fail_count: int = 0


@dataclass(frozen=True)
class AbHarnessSummary:
    """Complete A/B boot campaign report comparing NORMAL vs JIG boots."""

    runs: tuple[AbBootRun, ...]
    by_mode: dict[str, AbModeStats]


def build_ab_boot_command(mode: str, window_ms: int = 6000) -> str:
    """Build the safe read-only Pico command via `panadiga pico ab-boot --kind build-cmd`."""
    try:
        rep = as_dict(
            run_panadiga_json(["pico", "ab-boot", "--kind", "build-cmd", "--mode", mode, "--window-ms", str(window_ms)])
        )
    except PanadigaCliError as exc:
        raise ValueError(str(exc)) from exc
    return req_str(rep, "text")


def _parse_ab_boot_run(jd: JsonDict) -> AbBootRun:
    return AbBootRun(
        run_index=req_int(jd, "run_index"),
        mode=req_str(jd, "mode"),
        command=req_str(jd, "command"),
        ddr_check_ok=req_bool(jd, "ddr_check_ok"),
        boot_mode=opt_str(jd, "boot_mode"),
        bgo_failed_code=opt_int(jd, "bgo_failed_code"),
        sd_91006=req_bool(jd, "sd_91006"),
        sd_90002=req_bool(jd, "sd_90002"),
        sd_90001=req_bool(jd, "sd_90001"),
        odd_83a7=req_bool(jd, "odd_83a7"),
        duration_ms=opt_float(jd, "duration_ms") or 0.0,
        sd_hw_ok=req_bool(jd, "sd_hw_ok"),
        sd_diag_load_fail=req_bool(jd, "sd_diag_load_fail"),
        sd_uncompress_fail=req_bool(jd, "sd_uncompress_fail"),
        odd_tray_fail=req_bool(jd, "odd_tray_fail"),
        diagnosis=req_str(jd, "diagnosis"),
    )


def classify_ab_boot_log(
    run_index: int,
    mode: str,
    command: str,
    raw_log: str,
    duration_ms: float = 0.0,
) -> AbBootRun:
    """Parse one raw Pico boot capture log via `panadiga pico ab-boot --kind classify`."""
    rep = as_dict(
        run_panadiga_json(
            [
                "pico",
                "ab-boot",
                "--kind",
                "classify",
                "--run-index",
                str(run_index),
                "--mode",
                mode,
                "--command",
                command,
                "--duration-ms",
                f"{duration_ms:.4f}",
                "--input",
                "-",
            ],
            stdin_bytes=raw_log.encode("utf-8"),
        )
    )
    return _parse_ab_boot_run(rep)


def summarize_ab_runs(runs: Sequence[AbBootRun]) -> AbHarnessSummary:
    """Aggregate per-mode counters via `panadiga pico ab-boot --kind summarize`."""
    payload = json.dumps([asdict(r) for r in runs]).encode("utf-8")
    rep = as_dict(
        run_panadiga_json(["pico", "ab-boot", "--kind", "summarize", "--input", "-"], stdin_bytes=payload)
    )
    raw_runs = as_list(rep.get("runs"))
    parsed_runs = tuple(_parse_ab_boot_run(as_dict(item)) for item in raw_runs)
    raw_by_mode = as_dict(rep.get("by_mode"))
    by_mode: dict[str, AbModeStats] = {}
    for k, v in raw_by_mode.items():
        mv = as_dict(v)
        by_mode[k] = AbModeStats(
            mode=req_str(mv, "mode"),
            runs=req_int(mv, "runs"),
            ddr_ok_count=req_int(mv, "ddr_ok_count"),
            sd_91006_count=req_int(mv, "sd_91006_count"),
            sd_90002_count=req_int(mv, "sd_90002_count"),
            sd_90001_count=req_int(mv, "sd_90001_count"),
            odd_83a7_count=req_int(mv, "odd_83a7_count"),
            bgo_fail_count=req_int(mv, "bgo_fail_count"),
            sd_hw_ok_count=req_int(mv, "sd_hw_ok_count"),
            sd_diag_load_fail_count=req_int(mv, "sd_diag_load_fail_count"),
        )
    return AbHarnessSummary(runs=parsed_runs, by_mode=by_mode)


def format_ab_csv(runs: Sequence[AbBootRun]) -> str:
    """Render structured CSV table via `panadiga pico ab-boot --kind csv`."""
    payload = json.dumps([asdict(r) for r in runs]).encode("utf-8")
    rep = as_dict(
        run_panadiga_json(["pico", "ab-boot", "--kind", "csv", "--input", "-"], stdin_bytes=payload)
    )
    return req_str(rep, "text")


def run_ab_boot_harness(
    transport: TransportProtocol,
    pairs: int = 3,
    window_ms: int = 6000,
) -> AbHarnessSummary:
    """Execute alternating NORMAL (SW_UART Hi-Z) vs JIG (SW_UART LOW) XRESET boot pairs."""
    if pairs <= 0:
        raise ValueError("pairs must be >= 1")
    timeout_s = max(4.0, (window_ms / 1000.0) + 3.0)
    results: list[AbBootRun] = []

    with BenchSession(transport) as session:
        run_idx = 0
        for _ in range(pairs):
            for mode in ("NORMAL", "JIG"):
                cmd = build_ab_boot_command(mode, window_ms=window_ms)
                t0 = time.monotonic()
                raw_log = session.send(cmd, timeout_s=timeout_s)
                elapsed_ms = (time.monotonic() - t0) * 1000.0
                results.append(
                    classify_ab_boot_log(
                        run_index=run_idx,
                        mode=mode,
                        command=cmd,
                        raw_log=raw_log,
                        duration_ms=elapsed_ms,
                    )
                )
                run_idx += 1

    return summarize_ab_runs(results)
