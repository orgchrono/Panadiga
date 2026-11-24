"""Unit tests for Pico heartbeat discovery, grading, and SAFETY_DENIED guided guards."""

from __future__ import annotations

from typing import Sequence

import pytest

from scripts.pico_heartbeat_ops import (
    SAFE_GUIDED_PROFILES,
    check_command_safety,
    discover_pico_port,
    format_report_human,
    main,
    run_guided,
    run_heartbeat,
)


def _mock_list_runner(args: Sequence[str]) -> tuple[int, str, str]:
    if list(args[:2]) == ["pico", "list"]:
        return (
            0,
            "\n".join(
                [
                    "Port: COM3 | Generic USB Serial | VID:1A86 PID:7523 | CH340",
                    "Port: /dev/ttyACM0 | RP2350 (Pico 2) | VID:2E8A PID:000F | Panadiga P7501",
                ]
            ),
            "",
        )
    if list(args[:2]) == ["pico", "ping"]:
        return 0, "PONG fw=P7501_ZERO_GADGET", ""
    if list(args[:2]) == ["pico", "status"]:
        return 0, "PING: PONG OK\nVDD: 3.31 V\nDCSR: 0x00000001 -> HALTED", ""
    return 1, "", "unexpected cmd"


def test_discover_prefers_vid_2e8a_and_allows_explicit_override() -> None:
    port, source, detected = discover_pico_port(None, runner=_mock_list_runner)
    assert port == "/dev/ttyACM0"
    assert source == "auto_vid_2e8a"
    assert len(detected) == 2

    explicit_port, explicit_source, _ = discover_pico_port("COM9", runner=_mock_list_runner)
    assert explicit_port == "COM9"
    assert explicit_source == "explicit"


def test_heartbeat_grades_healthy_standby_overvoltage_and_missing() -> None:
    rc_ok, rep_ok = run_heartbeat(runner=_mock_list_runner)
    assert rc_ok == 0
    assert rep_ok["overall_health"] == "ONLINE_HEALTHY"

    def standby_runner(args: Sequence[str]) -> tuple[int, str, str]:
        if args[1] == "status":
            return 0, "VDD: 0.12 V\nDCSR: 0xFFFFFFFF -> FLOATING HIGH", ""
        return _mock_list_runner(args)

    _, rep_stby = run_heartbeat(port="/dev/ttyACM0", runner=standby_runner)
    assert rep_stby["overall_health"] == "TARGET_OFF_OR_STANDBY"

    def overvolt_runner(args: Sequence[str]) -> tuple[int, str, str]:
        if args[1] == "status":
            return 0, "VDD: 4.92 V\nDCSR: 0x00000000", ""
        return _mock_list_runner(args)

    rc_ov, rep_ov = run_heartbeat(port="COM7", runner=overvolt_runner)
    assert rc_ov == 2
    assert rep_ov["overall_health"] == "OVERVOLTAGE_ABORT"

    rc_empty, rep_empty = run_heartbeat(runner=lambda _a: (0, "No USB serial devices detected.", ""))
    assert rc_empty == 2
    assert rep_empty["overall_health"] == "DISCONNECTED"


def test_heartbeat_accepts_healthy_wdt_active_and_flags_real_wdt_faults() -> None:
    def live_healthy_runner(args: Sequence[str]) -> tuple[int, str, str]:
        if args[1] == "ping":
            return (
                0,
                "VDD: 3.26V (OK)\n[WDT] wdt=ACTIVE(4000ms,NON_OVERRIDABLE) seq=1750 "
                "wdt_recoveries=0 reboot_src=CLEAN fault_slot=255 fault_stage=IDLE\n",
                "",
            )
        return _mock_list_runner(args)

    rc_live, rep_live = run_heartbeat(port="COM10", runner=live_healthy_runner)
    assert rc_live == 0
    assert rep_live["overall_health"] == "ONLINE_HEALTHY"

    for fault_banner in (
        "VDD: 3.26V\nwdt_recoveries=1 reboot_src=WDT\n",
        "VDD: 3.26V\nWDT_TRIP detected\n",
        "VDD: 3.26V\nSTALL in flash task\n",
    ):
        def fault_runner(args: Sequence[str], banner: str = fault_banner) -> tuple[int, str, str]:
            if args[1] == "ping":
                return 0, banner, ""
            return _mock_list_runner(args)

        rc_fault, rep_fault = run_heartbeat(port="COM10", runner=fault_runner)
        assert rc_fault == 1
        assert rep_fault["overall_health"] == "WDT_STALL"


def test_guided_hard_blocks_forbidden_tokens_with_safety_denied() -> None:
    dangerous = [
        "ERASE 0x807F0000",
        "WRITE 0x80040000 0x1234",
        "FLASH program.bin",
        "POKE 0xC000100C 0",
        "PATCH BGO",
        "MEMW 0x50400000 1",
        "W32 0x807F0000 0xFFFFFFFF",
        "CONFIRM_UPPER_NOR",
        "SNIFF 0xC000_100C",
        "UARTSEND 0x50409DF2",
        "UARTSEND 0x50409E65",
        "BGO_WRITE sector 26",
        "NVRAM_WRITE",
        "M807F0000,4:FFFFFFFF",
        "X807F0000,2:AA",
    ]
    for bad in dangerous:
        allowed, reason = check_command_safety(bad)
        assert not allowed
        assert "SAFETY_DENIED" in reason
        rc, rep = run_guided(profile="status_dcsr", cmd=bad, dry_run=True, validator=lambda s: s)
        assert rc == 2
        assert rep["status"] == "SAFETY_DENIED"


def test_guided_dry_run_validates_safe_profiles() -> None:
    for profile in SAFE_GUIDED_PROFILES:
        rc, rep = run_guided(
            profile=profile,
            cmd="STATUS",
            port="COM12",
            dry_run=True,
            validator=lambda s: s.strip().upper(),
        )
        assert rc == 0
        assert rep["status"] == "DRY_RUN_READY"
        assert rep["port"] == "COM12"


def test_heartbeat_human_multiline_and_json_cli_modes(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    rc, rep = run_heartbeat(runner=_mock_list_runner)
    assert rc == 0
    samples = rep["samples"]
    assert isinstance(samples, list) and isinstance(samples[0], dict)
    assert samples[0]["status_lines"] == [
        "PING: PONG OK",
        "VDD: 3.31 V",
        "DCSR: 0x00000001 -> HALTED",
    ]
    human = format_report_human(rep)
    assert "\\n" not in human
    assert "PING: PONG OK\n  VDD: 3.31 V\n  DCSR: 0x00000001 -> HALTED" in human

    monkeypatch.setattr("scripts.pico_heartbeat_ops.default_cli_runner", _mock_list_runner)
    assert main(["--mode", "heartbeat"]) == 0
    out_text = capsys.readouterr().out
    assert "[HEARTBEAT] overall_health=ONLINE_HEALTHY" in out_text
    assert "\\n" not in out_text

    assert main(["--mode", "heartbeat", "--json", "-"]) == 0
    out_json = capsys.readouterr().out
    assert '"status_lines"' in out_json
    assert '"overall_health": "ONLINE_HEALTHY"' in out_json
