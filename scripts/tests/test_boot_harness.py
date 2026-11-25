"""Unit tests for the automated A/B XRESET + SW_UART boot harness."""

from __future__ import annotations

import csv
import io
from pathlib import Path

from scripts.bench.boot_harness import (
    format_ab_csv,
    run_ab_boot_harness,
)
from scripts.bench.protocol import BenchSafetyError, TransportProtocol
from scripts.bench.runner import main as bench_main


class _SequenceBootTransport(TransportProtocol):
    """Deterministic multi-run transport exercising intermittent SD/ODD faults."""

    def __init__(self, jig_responses: list[str]) -> None:
        self.commands: list[str] = []
        self._jig_responses = jig_responses
        self._jig_idx = 0

    def exchange(self, command: str, timeout_s: float = 4.0) -> str:
        del timeout_s
        self.commands.append(command)
        head = command.split()[0].upper()
        if head == "ISOLATE":
            return "ISOLATE: OK\r\nPICO> "
        if head == "BOOTCAP":
            return (
                "BOOTCAP\r\n"
                "DDR CHECK: OK\r\n"
                "***** Loader ver0.61 [06/03/06] *****\r\n"
                "***** normal mode *****\r\n"
                "BGO driver init failed[1].\r\n"
                "PICO> "
            )
        if head == "JIGBOOT":
            resp = self._jig_responses[self._jig_idx % len(self._jig_responses)]
            self._jig_idx += 1
            return resp
        raise BenchSafetyError(f"Unexpected command {command}")

    def close(self) -> None:
        return None


def test_ab_harness_tracks_intermittent_sd_and_83a7_counts() -> None:
    """Alternate NORMAL/JIG trials cleanly, classify counts, and emit valid CSV."""
    tr = _SequenceBootTransport(
        jig_responses=[
            (
                "DDR CHECK: OK\r\n"
                "***** serial boot mode *****\r\n"
                "[SDCD]SetBusWidth:SD Status ERROR -91006\r\n"
                "SD init failed[-90002].\r\n"
                "*** SENSE[70 00 04 00 00 00 00 0A 00 00 83 A7 09 04 00 00 00 00]\r\n"
                "PICO> "
            ),
            (
                "DDR CHECK: OK\r\n"
                "***** serial boot mode *****\r\n"
                "SDCD:sdcd_insrmv_check:No Card[-90001]\r\n"
                "SD init failed[-90002].\r\n"
                "*** SENSE[70 00 04 00 00 00 00 0A 00 00 83 A7 09 04 00 00 00 00]\r\n"
                "PICO> "
            ),
        ]
    )

    summary = run_ab_boot_harness(tr, pairs=2, window_ms=2000)
    assert len(summary.runs) == 4
    assert [c.split()[0] for c in tr.commands] == [
        "BOOTCAP",
        "JIGBOOT",
        "BOOTCAP",
        "JIGBOOT",
        "ISOLATE",
    ]
    assert summary.by_mode["NORMAL"].bgo_fail_count == 2
    assert summary.by_mode["NORMAL"].sd_91006_count == 0
    assert summary.by_mode["JIG"].runs == 2
    assert summary.by_mode["JIG"].sd_91006_count == 1
    assert summary.by_mode["JIG"].sd_90001_count == 1
    assert summary.by_mode["JIG"].sd_90002_count == 2
    assert summary.by_mode["JIG"].odd_83a7_count == 2

    csv_text = format_ab_csv(summary.runs)
    rows = list(csv.DictReader(io.StringIO(csv_text)))
    assert len(rows) == 4
    assert rows[1]["mode"] == "JIG"
    assert rows[1]["sd_91006"] == "True"
    assert rows[3]["sd_90001"] == "True"


def test_runner_ab_boot_cli_mock_writes_csv(tmp_path: Path) -> None:
    """Exercise CLI `runner ab-boot --mock --csv` subcommand cleanly."""
    csv_out = tmp_path / "ab_runs.csv"
    rc = bench_main(["ab-boot", "--mock", "--pairs", "2", "--window-ms", "1500", "--csv", str(csv_out)])
    assert rc == 0
    assert csv_out.exists()
    rows = list(csv.DictReader(io.StringIO(csv_out.read_text(encoding="utf-8"))))
    assert len(rows) == 4
