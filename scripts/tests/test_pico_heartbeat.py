"""Unit tests for Pico heartbeat discovery, grading, and SAFETY_DENIED guided guards."""

from __future__ import annotations

from typing import Sequence

from scripts.pico_heartbeat_ops import (
    SAFE_GUIDED_PROFILES,
    check_command_safety,
    discover_pico_port,
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
