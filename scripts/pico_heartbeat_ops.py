"""PicoFW (P7501) heartbeat probe and read-only guided command dispatcher."""

from __future__ import annotations

import os
import sys

if __package__ in (None, ""):
    _here_hb = os.path.dirname(os.path.abspath(__file__))
    sys.path = [os.path.dirname(_here_hb), *(p for p in sys.path if os.path.abspath(p or ".") != _here_hb)]

import time
import subprocess
import re
import json
import argparse
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Final, Sequence

from scripts.bench.protocol import SAFE_BENCH_COMMANDS, validate_safe_bench_command
from scripts.panadiga_cli_bridge import JsonDict, JsonValue, resolve_panadiga_cli_prefix
from scripts.panadiga_ops import ROOT, build_env
from scripts.profiles_types import FORBIDDEN_BENCH_TOKENS

FORBIDDEN_TOKENS: Final[tuple[str, ...]] = tuple(
    dict.fromkeys(
        (
            *FORBIDDEN_BENCH_TOKENS,
            "CONFIRM_UPPER_NOR",
            "0XC000100C",
            "0XC000_100C",
            "0X50409DF2",
            "0X50409E65",
            "BGO_WRITE",
            "NVRAM_WRITE",
            "M80",
            "X80",
            "MEMW",
            "W32",
            "W16",
            "W8",
            "JOB",
        )
    )
)

SAFE_GUIDED_PROFILES: Final[dict[str, tuple[str, ...]]] = {
    "passive_triage": ("PING", "STATUS", "PINCHECK", "BUSDIAG"),
    "status_dcsr": ("PING", "STATUS"),
    "bus_health": ("PING", "PINCHECK", "BUSDIAG"),
}

_LIST_PORT_RE: Final[re.Pattern[str]] = re.compile(
    r"^\s*Port:\s*(\S+)\s*\|\s*([^|]+?)\s*\|\s*VID:([0-9A-Fa-f]{4})\s+PID:([0-9A-Fa-f]{4})\s*\|\s*(.*)$",
    re.MULTILINE,
)
_VDD_RE: Final[re.Pattern[str]] = re.compile(r"VDD[:=]\s*([0-9]+(?:\.[0-9]+)?)\s*V", re.IGNORECASE)
_DCSR_RE: Final[re.Pattern[str]] = re.compile(r"DCSR:\s*(0x[0-9A-Fa-f]{8})", re.IGNORECASE)
_WDT_RECOVERY_NONZERO_RE: Final[re.Pattern[str]] = re.compile(r"\bWDT_RECOVERIES=[1-9]\d*\b")

CommandRunner = Callable[[Sequence[str]], tuple[int, str, str]]
BenchValidator = Callable[[str], str]


@dataclass(frozen=True)
class DetectedSerialPort:
    """One attached serial controller parsed from `panadiga pico list`."""

    port: str
    architecture: str
    vid: str
    pid: str
    description: str


@dataclass(frozen=True)
class HeartbeatSample:
    """Single heartbeat telemetry probe result with deterministic health grade."""

    sample_index: int
    port: str | None
    port_source: str
    ping_ok: bool
    vdd_volts: float | None
    dcsr_hex: str | None
    health: str
    detail: str
    ping_raw: str
    status_raw: str
    ping_lines: list[str]
    status_lines: list[str]


def _split_lines(raw: str) -> list[str]:
    """Normalize CRLF/CR line endings into clean individual lines."""
    if not raw:
        return []
    return raw.replace("\r\n", "\n").replace("\r", "\n").splitlines()


def default_cli_runner(args: Sequence[str]) -> tuple[int, str, str]:
    """Run a `panadiga` CLI subcommand and return `(returncode, stdout, stderr)`."""
    cmd = [*resolve_panadiga_cli_prefix(), *args]
    proc = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=str(ROOT),
        env=build_env(),
        check=False,
    )
    return proc.returncode, proc.stdout, proc.stderr


def parse_pico_list_output(output: str) -> list[DetectedSerialPort]:
    """Parse `panadiga pico list` stdout into structured serial port records."""
    ports: list[DetectedSerialPort] = []
    for port, arch, vid, pid, desc in _LIST_PORT_RE.findall(output):
        ports.append(
            DetectedSerialPort(
                port=port.strip(),
                architecture=arch.strip(),
                vid=vid.upper(),
                pid=pid.upper(),
                description=desc.strip(),
            )
        )
    return ports


def discover_pico_port(
    explicit_port: str | None,
    *,
    runner: CommandRunner | None = None,
) -> tuple[str | None, str, list[DetectedSerialPort]]:
    """Resolve Pico serial port without hardcoding `COM10`, preferring VID `2E8A`."""
    if explicit_port is not None and explicit_port.strip():
        return explicit_port.strip(), "explicit", []
    active_runner = runner if runner is not None else default_cli_runner
    rc, stdout, _ = active_runner(["pico", "list"])
    if rc != 0:
        return None, "discovery_failed", []
    detected = parse_pico_list_output(stdout)
    if not detected:
        return None, "none_detected", []
    for entry in detected:
        if entry.vid == "2E8A":
            return entry.port, "auto_vid_2e8a", detected
    return detected[0].port, "auto_first_port", detected


_WRITE_VERB_WORDS: Final[frozenset[str]] = frozenset(
    {"MW", "MB", "MH", "WB", "WH", "WW", "PW", "PB", "PH", "PO", "EB", "EW", "SW"}
)
_LETHAL_MMIO_RANGES: Final[tuple[tuple[int, int], ...]] = (
    (0xC000_100C, 0xC000_1010),
    (0xDC0F_0000, 0xDC0F_0010),
)


def _parse_hex_addr_token(tok: str) -> int | None:
    clean = tok.strip().replace("_", "")
    if clean.startswith("0X"):
        raw = clean[2:]
        if not raw or len(raw) > 8 or any(c not in "0123456789ABCDEF" for c in raw):
            return None
        return int(raw, 16)
    if 6 <= len(clean) <= 8 and all(c in "0123456789ABCDEF" for c in clean):
        return int(clean, 16)
    return None


def _parse_span_len_token(tok: str) -> int | None:
    clean = tok.strip().replace("_", "")
    if not clean:
        return None
    if clean.startswith("0X"):
        raw = clean[2:]
        if not raw or len(raw) > 8 or any(c not in "0123456789ABCDEF" for c in raw):
            return None
        return min(int(raw, 16), 0x10000)
    if clean.isdigit():
        return min(int(clean, 10), 0x10000)
    if len(clean) <= 4 and all(c in "0123456789ABCDEF" for c in clean):
        return min(int(clean, 16), 0x10000)
    return None


def _contains_lethal_am34_address(tokens: Sequence[str]) -> bool:
    for idx, tok in enumerate(tokens):
        addr = _parse_hex_addr_token(tok)
        if addr is None:
            continue
        span_len = 4
        if idx + 1 < len(tokens):
            parsed_len = _parse_span_len_token(tokens[idx + 1])
            if parsed_len is not None and parsed_len > 0:
                span_len = max(span_len, parsed_len)
        end = addr + span_len
        for lo, hi in _LETHAL_MMIO_RANGES:
            if addr < hi and end > lo:
                return True
    return False


def _decode_uart_hex_upper(head: str, tokens: list[str]) -> str | None:
    if head not in {"UARTTXHEX", "WAITTX"}:
        return None
    raw_hex = ""
    if head == "UARTTXHEX" and len(tokens) >= 2:
        raw_hex = tokens[1].removeprefix("LOW:")
    if not raw_hex or raw_hex in {"BRK", "BREAK"} or len(raw_hex) % 2 != 0:
        return None
    try:
        return bytes.fromhex(raw_hex).decode("ascii", errors="ignore").upper()
    except ValueError:
        return None


def check_command_safety(cmd: str) -> tuple[bool, str]:
    """Hard-block destructive/MMIO-unsafe tokens with explicit `SAFETY_DENIED` verdict."""
    normalized = cmd.strip()
    if not normalized:
        return False, "SAFETY_DENIED: empty command"
    upper_full = normalized.upper()
    upper_tokens = [tok for tok in re.split(r"[\s,;:=]+", upper_full) if tok]
    head = upper_full.split()[0]
    if head not in SAFE_BENCH_COMMANDS:
        return False, f"SAFETY_DENIED: command '{head}' is outside SAFE_BENCH_COMMANDS"
    decoded_upper = _decode_uart_hex_upper(head, upper_full.split())
    for candidate_full, candidate_toks in [
        (upper_full, upper_tokens),
        *((decoded_upper, [t for t in re.split(r"[\s,;:=]+", decoded_upper) if t]) for _ in [0] if decoded_upper),
    ]:
        for bad in FORBIDDEN_TOKENS:
            if bad in candidate_toks or (bad.startswith("0X") and bad in candidate_full) or bad in candidate_full:
                return False, f"SAFETY_DENIED: forbidden token '{bad}' in command '{normalized}'"
        if _contains_lethal_am34_address(candidate_toks):
            return False, f"SAFETY_DENIED: lethal AM34 MMIO span in command '{normalized}'"
        if head in {"UARTSEND", "UARTTXHEX", "WAITTX", "PANARSP"} and any(
            t in _WRITE_VERB_WORDS or t.startswith(("$M", "$X")) for t in candidate_toks
        ):
            return False, f"SAFETY_DENIED: write-intent payload in '{normalized}'"
    return True, f"ALLOWED:{normalized}"


def grade_heartbeat(
    *,
    ping_ok: bool,
    vdd_volts: float | None,
    dcsr_hex: str | None,
    ping_rc: int,
    status_rc: int,
    combined_text: str,
) -> tuple[str, str]:
    """Classify target bench telemetry into deterministic health state."""
    if vdd_volts is not None and vdd_volts > 3.65:
        return "OVERVOLTAGE_ABORT", f"Target VDD {vdd_volts:.2f}V exceeds 3.65V safety ceiling"
    upper = combined_text.upper()
    wdt_fault = (
        ("STALL" in upper)
        or ("WDT_STALL" in upper)
        or ("WDT_TRIP" in upper)
        or ("WATCHDOG TIMEOUT" in upper)
        or ("REBOOT_SRC=WDT" in upper)
        or (_WDT_RECOVERY_NONZERO_RE.search(upper) is not None)
        or ("TIMEOUT" in upper and not ping_ok)
    )
    if wdt_fault:
        return "WDT_STALL", "Probe or target reported stall/watchdog timeout"
    if ping_rc != 0 or not ping_ok:
        return "DISCONNECTED", "Pico did not reply with PONG"
    if "NSW5V=0" in upper or "FAULT_LATCH=1" in upper:
        return "TARGET_OFF_OR_STANDBY", "STANDBY_GATE_STOP: NSW5V=0 or fault_latch=1 in STATUS"
    if dcsr_hex is not None and dcsr_hex.upper() == "0XFFFFFFFF":
        return "TARGET_OFF_OR_STANDBY", "DCSR=0xFFFFFFFF (target off or STANDBY / UART disconnected)"
    if vdd_volts is not None and vdd_volts < 2.50:
        return "TARGET_OFF_OR_STANDBY", f"VDD={vdd_volts:.2f}V (< 2.50V Standby/off)"
    if status_rc != 0 and vdd_volts is None:
        return "WDT_STALL", "PONG ok but STATUS/DCSR query returned non-zero without VDD"
    if vdd_volts is not None and 2.95 <= vdd_volts <= 3.60:
        return "ONLINE_HEALTHY", f"PONG ok, VDD={vdd_volts:.2f}V in 2.95..3.60V LVTTL window"
    return "TARGET_OFF_OR_STANDBY", "PONG ok without healthy 3.3V VDD sense"


def run_heartbeat(
    *,
    port: str | None = None,
    baud: int = 115200,
    count: int = 1,
    interval_s: float = 0.0,
    runner: CommandRunner | None = None,
    sleep_fn: Callable[[float], None] = time.sleep,
) -> tuple[int, JsonDict]:
    """Execute 1..N Pico heartbeat samples and return `(exit_code, report_dict)`."""
    active_runner = runner if runner is not None else default_cli_runner
    samples: list[HeartbeatSample] = []
    worst_exit = 0
    iterations = max(1, count)

    for idx in range(iterations):
        resolved_port, port_source, detected = discover_pico_port(port, runner=active_runner)
        if resolved_port is None:
            samples.append(
                HeartbeatSample(
                    sample_index=idx + 1,
                    port=None,
                    port_source=port_source,
                    ping_ok=False,
                    vdd_volts=None,
                    dcsr_hex=None,
                    health="DISCONNECTED",
                    detail=f"No serial Pico port detected (candidates={len(detected)})",
                    ping_raw="",
                    status_raw="",
                    ping_lines=[],
                    status_lines=[],
                )
            )
            worst_exit = 2
        else:
            p_rc, p_out, p_err = active_runner(["pico", "ping", "-p", resolved_port, "--baud", str(baud)])
            s_rc, s_out, s_err = active_runner(["pico", "status", "-p", resolved_port, "--baud", str(baud)])
            ping_raw = (p_out or p_err).strip()
            status_raw = (s_out or s_err).strip()
            combined = f"{ping_raw}\n{status_raw}"
            ping_ok = "PONG" in combined.upper()
            vdd_match = _VDD_RE.search(combined)
            vdd_val = float(vdd_match.group(1)) if vdd_match else None
            dcsr_match = _DCSR_RE.search(combined)
            dcsr_val = dcsr_match.group(1).upper().replace("0X", "0x") if dcsr_match else None
            health, detail = grade_heartbeat(
                ping_ok=ping_ok,
                vdd_volts=vdd_val,
                dcsr_hex=dcsr_val,
                ping_rc=p_rc,
                status_rc=s_rc,
                combined_text=combined,
            )
            if health in {"DISCONNECTED", "OVERVOLTAGE_ABORT", "WDT_STALL"}:
                worst_exit = max(worst_exit, 2 if health == "OVERVOLTAGE_ABORT" else 1)
            samples.append(
                HeartbeatSample(
                    sample_index=idx + 1,
                    port=resolved_port,
                    port_source=port_source,
                    ping_ok=ping_ok,
                    vdd_volts=vdd_val,
                    dcsr_hex=dcsr_val,
                    health=health,
                    detail=detail,
                    ping_raw=ping_raw,
                    status_raw=status_raw,
                    ping_lines=_split_lines(ping_raw),
                    status_lines=_split_lines(status_raw),
                )
            )
        if idx + 1 < iterations and interval_s > 0.0:
            sleep_fn(interval_s)

    report: JsonDict = {
        "mode": "heartbeat",
        "baud": baud,
        "sample_count": len(samples),
        "overall_health": samples[-1].health if samples else "DISCONNECTED",
        "samples": [asdict(item) for item in samples],
    }
    return worst_exit, report


def run_guided(
    *,
    profile: str,
    cmd: str | None = None,
    port: str | None = None,
    baud: int = 115200,
    dry_run: bool = False,
    runner: CommandRunner | None = None,
    validator: BenchValidator = validate_safe_bench_command,
) -> tuple[int, JsonDict]:
    """Validate and dispatch a read-only guided bench profile or single safe command."""
    active_runner = runner if runner is not None else default_cli_runner
    if profile not in SAFE_GUIDED_PROFILES:
        return 2, {
            "mode": "guided",
            "status": "INVALID_PROFILE",
            "profile": profile,
            "allowed_profiles": list[JsonValue](sorted(SAFE_GUIDED_PROFILES)),
        }

    steps_raw = list(SAFE_GUIDED_PROFILES[profile])
    if cmd is not None and cmd.strip():
        steps_raw.append(cmd.strip())

    validated_steps: list[str] = []
    for step in steps_raw:
        safe_ok, reason = check_command_safety(step)
        if not safe_ok:
            return 2, {
                "mode": "guided",
                "status": "SAFETY_DENIED",
                "profile": profile,
                "rejected_command": step,
                "reason": reason,
            }
        try:
            normalized = validator(step)
        except Exception as exc:
            return 2, {
                "mode": "guided",
                "status": "SAFETY_DENIED",
                "profile": profile,
                "rejected_command": step,
                "reason": f"SAFETY_DENIED: {exc}",
            }
        validated_steps.append(normalized)

    if dry_run:
        return 0, {
            "mode": "guided",
            "status": "DRY_RUN_READY",
            "profile": profile,
            "port": port,
            "baud": baud,
            "steps": list(validated_steps),
        }

    resolved_port, port_source, _ = discover_pico_port(port, runner=active_runner)
    if resolved_port is None:
        return 2, {
            "mode": "guided",
            "status": "DISCONNECTED",
            "profile": profile,
            "port": None,
            "port_source": port_source,
            "steps": list(validated_steps),
        }

    rc, out, err = active_runner(["pico", "status", "-p", resolved_port, "--baud", str(baud)])
    return (0 if rc == 0 else 1), {
        "mode": "guided",
        "status": "EXECUTED" if rc == 0 else "CLI_ERROR",
        "profile": profile,
        "port": resolved_port,
        "port_source": port_source,
        "steps": list(validated_steps),
        "cli_output": (out or err).strip(),
    }


def emit_json_payload(payload: JsonDict, json_target: str | None) -> None:
    """Emit structured JSON payload cleanly to stdout (`-`) or a destination file."""
    text = json.dumps(payload, indent=2, ensure_ascii=False)
    if json_target == "-":
        print(text)
    elif json_target is not None:
        out_path = Path(json_target)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(text + "\n", encoding="utf-8")
    else:
        print(text)


def _format_raw_block(lines: list[str], fallback_raw: str) -> list[str]:
    """Render raw multiline probe output with indentation and real line breaks."""
    source = lines if lines else _split_lines(fallback_raw)
    return [f"  {line}" for line in source] if source else ["  (empty)"]


def format_report_human(payload: JsonDict) -> str:
    """Render heartbeat or guided telemetry as clean multiline human-readable text."""
    mode = str(payload.get("mode", "heartbeat"))
    out: list[str] = []
    if mode == "heartbeat":
        out.append(
            f"[HEARTBEAT] overall_health={payload.get('overall_health', 'UNKNOWN')} "
            f"baud={payload.get('baud', 115200)} samples={payload.get('sample_count', 0)}"
        )
        raw_samples = payload.get("samples", [])
        if isinstance(raw_samples, list):
            for entry in raw_samples:
                if not isinstance(entry, dict):
                    continue
                idx = entry.get("sample_index", "?")
                port = entry.get("port") or "(none)"
                vdd = entry.get("vdd_volts")
                vdd_str = f"{vdd:.2f}V" if isinstance(vdd, (int, float)) else "N/A"
                out.append(
                    f"\n--- Sample #{idx} port={port} ({entry.get('port_source', '')}) "
                    f"health={entry.get('health', '')} VDD={vdd_str} ---"
                )
                out.append(f"Detail: {entry.get('detail', '')}")
                ping_l = entry.get("ping_lines")
                ping_lines = [str(x) for x in ping_l] if isinstance(ping_l, list) else []
                out.append("PING:")
                out.extend(_format_raw_block(ping_lines, str(entry.get("ping_raw", ""))))
                st_l = entry.get("status_lines")
                st_lines = [str(x) for x in st_l] if isinstance(st_l, list) else []
                out.append("STATUS:")
                out.extend(_format_raw_block(st_lines, str(entry.get("status_raw", ""))))
        return "\n".join(out)

    out.append(
        f"[GUIDED] status={payload.get('status', '')} profile={payload.get('profile', '')} "
        f"port={payload.get('port') or '(none)'}"
    )
    if "reason" in payload:
        out.append(f"Reason: {payload['reason']}")
    steps_raw = payload.get("steps")
    if isinstance(steps_raw, list) and steps_raw:
        out.append(f"Steps: {', '.join(str(s) for s in steps_raw)}")
    if "cli_output" in payload:
        out.append("Output:")
        out.extend(_format_raw_block([], str(payload.get("cli_output", ""))))
    return "\n".join(out)


def main(argv: Sequence[str] | None = None) -> int:
    """CLI entrypoint for PicoFW (P7501) heartbeat probing and safe guided execution."""
    parser = argparse.ArgumentParser(
        description="Panadiga PicoFW (P7501) heartbeat monitor and read-only guided ops."
    )
    parser.add_argument("--mode", choices=("heartbeat", "guided"), default="heartbeat")
    parser.add_argument("--port", default=None, help="Explicit serial port override (e.g. /dev/ttyACM0)")
    parser.add_argument("--baud", type=int, default=115200)
    parser.add_argument("--count", type=int, default=1)
    parser.add_argument("--interval-s", type=float, default=0.0)
    parser.add_argument(
        "--profile",
        choices=tuple(sorted(SAFE_GUIDED_PROFILES)),
        default="passive_triage",
    )
    parser.add_argument("--cmd", default=None, help="Additional read-only bench command to validate")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--format",
        choices=("text", "json"),
        default="text",
        help="Console output format when --json path is not given (default: text)",
    )
    parser.add_argument(
        "--json",
        default=None,
        help="Optional JSON output file path or '-' for JSON stdout",
    )
    args = parser.parse_args(argv)

    if args.mode == "heartbeat":
        code, payload = run_heartbeat(
            port=args.port,
            baud=args.baud,
            count=args.count,
            interval_s=args.interval_s,
        )
    else:
        code, payload = run_guided(
            profile=args.profile,
            cmd=args.cmd,
            port=args.port,
            baud=args.baud,
            dry_run=args.dry_run,
        )
    if args.json is not None or args.format == "json":
        emit_json_payload(payload, args.json or "-")
    else:
        print(format_report_human(payload))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
