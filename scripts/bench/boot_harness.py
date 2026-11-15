"""Automated A/B cold-boot statistical harness (`NORMAL` vs `JIG`) over Pico P7501."""

from __future__ import annotations

import csv
from dataclasses import asdict, dataclass
import io
import time
from typing import Sequence

from scripts.bench.decoders import parse_am34_uart_log
from scripts.bench.protocol import BenchSession, TransportProtocol

_ALLOWED_AB_MODES = ("NORMAL", "JIG")


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


@dataclass(frozen=True)
class AbHarnessSummary:
    """Complete A/B boot campaign report comparing NORMAL vs JIG boots."""

    runs: tuple[AbBootRun, ...]
    by_mode: dict[str, AbModeStats]


def build_ab_boot_command(mode: str, window_ms: int = 6000) -> str:
    """Build the safe read-only Pico command for one A/B trial arm."""
    norm = mode.strip().upper()
    if norm not in _ALLOWED_AB_MODES:
        raise ValueError(f"Unsupported A/B boot mode '{mode}'; expected NORMAL or JIG")
    clamped_ms = max(500, min(60000, int(window_ms)))
    return f"BOOTCAP {clamped_ms}" if norm == "NORMAL" else f"JIGBOOT {clamped_ms}"


def classify_ab_boot_log(
    run_index: int,
    mode: str,
    command: str,
    raw_log: str,
    duration_ms: float = 0.0,
) -> AbBootRun:
    """Parse one raw Pico boot capture log deterministically into structured counts."""
    report = parse_am34_uart_log(raw_log)
    upper = raw_log.upper()
    odd_83a7 = (
        any(pkt.vendor_mecha_code == 0x83A7 for pkt in report.sense_packets)
        or "83 A7" in upper
        or "83A7" in upper
    )
    return AbBootRun(
        run_index=run_index,
        mode=mode,
        command=command,
        ddr_check_ok=report.ddr_check_ok,
        boot_mode=report.boot_mode,
        bgo_failed_code=report.bgo_failed_code,
        sd_91006=(-91006 in report.sdcd_errors),
        sd_90002=(-90002 in report.sdcd_errors),
        sd_90001=(-90001 in report.sdcd_errors),
        odd_83a7=odd_83a7,
        duration_ms=round(duration_ms, 2),
    )


def summarize_ab_runs(runs: Sequence[AbBootRun]) -> AbHarnessSummary:
    """Aggregate per-mode counters across all recorded A/B trials."""
    by_mode: dict[str, AbModeStats] = {}
    for mode in _ALLOWED_AB_MODES:
        group = [r for r in runs if r.mode == mode]
        by_mode[mode] = AbModeStats(
            mode=mode,
            runs=len(group),
            ddr_ok_count=sum(1 for r in group if r.ddr_check_ok),
            sd_91006_count=sum(1 for r in group if r.sd_91006),
            sd_90002_count=sum(1 for r in group if r.sd_90002),
            sd_90001_count=sum(1 for r in group if r.sd_90001),
            odd_83a7_count=sum(1 for r in group if r.odd_83a7),
            bgo_fail_count=sum(1 for r in group if r.bgo_failed_code is not None),
        )
    return AbHarnessSummary(runs=tuple(runs), by_mode=by_mode)


def format_ab_csv(runs: Sequence[AbBootRun]) -> str:
    """Render structured CSV table for spreadsheet/statistical offline inspection."""
    fieldnames = (
        "run_index",
        "mode",
        "command",
        "ddr_check_ok",
        "boot_mode",
        "bgo_failed_code",
        "sd_91006",
        "sd_90002",
        "sd_90001",
        "odd_83a7",
        "duration_ms",
    )
    buf = io.StringIO()
    writer = csv.DictWriter(buf, fieldnames=fieldnames, lineterminator="\n")
    writer.writeheader()
    for r in runs:
        writer.writerow(asdict(r))
    return buf.getvalue()


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
            for mode in _ALLOWED_AB_MODES:
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
