"""Thin CLI runner delegating Pico P7501 bench diagnostics & AM34 decoding to `panadiga` CLI."""

from __future__ import annotations

import argparse
from dataclasses import asdict
import json
from pathlib import Path
from typing import cast

from scripts.bench.boot_harness import format_ab_csv, run_ab_boot_harness
from scripts.bench.decoders import (
    parse_am34_uart_log,
    parse_busdiag_output,
    parse_pincheck_output,
    parse_status_output,
    read_captured_log,
)
from scripts.bench.protocol import BenchSession, MockBenchTransport, SerialTransport, TransportProtocol
from scripts.panadiga_cli_bridge import JsonDict, JsonValue, as_dict, req_bool


def _make_transport(port: str | None, simulate_bug: bool = False) -> TransportProtocol:
    if port:
        return SerialTransport(port)
    return MockBenchTransport(simulate_gp26_sticky_low=simulate_bug)


def run_health(port: str | None) -> JsonDict:
    """Execute read-only electrical health verification (`PING`, `STATUS`, `PINCHECK`, `BUSDIAG`, `ISOLATE`)."""
    with BenchSession(_make_transport(port)) as session:
        ping = session.send("PING", timeout_s=2.0)
        status = parse_status_output(session.send("STATUS", timeout_s=2.0))
        pins = parse_pincheck_output(session.send("PINCHECK", timeout_s=3.0))
        bus = parse_busdiag_output(session.send("BUSDIAG", timeout_s=3.0))
    return {
        "pong": "PONG" in ping,
        "status": {
            "vdd_volts": status.vdd_volts,
            "vdd_status": status.vdd_status,
            "gp26_sticky_low_suspected": status.gp26_sticky_low_suspected,
        },
        "pincheck_rows": [cast(JsonValue, asdict(r)) for r in pins],
        "busdiag": cast(JsonValue, asdict(bus)),
    }


def run_capture(command: str, port: str | None, out_path: Path | None) -> JsonDict:
    """Execute `BOOTCAP`, `JIGBOOT`, or `UARTMON`, decode via native Rust CLI, and isolate bus."""
    with BenchSession(_make_transport(port)) as session:
        raw_log = session.send(command, timeout_s=6.0)
    if out_path is not None:
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(raw_log, encoding="utf-8")
    decoded = parse_am34_uart_log(raw_log)
    return {"command": command, "bytes": len(raw_log), "decoded": cast(JsonValue, asdict(decoded))}


def run_smoke_suite() -> JsonDict:
    """Run offline self-check verifying health, sticky-LOW bug detection, and JIGBOOT decoding."""
    health_res = run_health(None)
    jig_res = run_capture("JIGBOOT", None, None)

    bug_tr = MockBenchTransport(simulate_gp26_sticky_low=True)
    with BenchSession(bug_tr) as session:
        _ = session.send("UARTSEND VER")
        bug_status = parse_status_output(session.send("STATUS"))
        _ = session.send("UARTPROBE 8")
        fixed_status = parse_status_output(session.send("STATUS"))

    assert req_bool(health_res, "pong") is True
    assert bug_status.gp26_sticky_low_suspected is True
    assert fixed_status.gp26_sticky_low_suspected is False
    decoded_dict = as_dict(jig_res.get("decoded"))
    assert decoded_dict.get("boot_mode") == "serial boot mode"

    status_dict = as_dict(health_res.get("status"))
    return {
        "smoke_ok": True,
        "health_vdd": status_dict.get("vdd_volts"),
        "gp26_bug_detected": bug_status.gp26_sticky_low_suspected,
        "gp26_recovered": not fixed_status.gp26_sticky_low_suspected,
        "jigboot_notes": decoded_dict.get("notes"),
    }


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint for `python -m scripts.bench.runner` (backed by native `panadiga` CLI)."""
    parser = argparse.ArgumentParser(description="Panadiga P7501 Non-Destructive Bench Runner")
    sub = parser.add_subparsers(dest="subcommand", required=True)

    p_health = sub.add_parser("health", help="Run PING/STATUS/PINCHECK/BUSDIAG/ISOLATE")
    p_health.add_argument("--port", default=None, help="Serial port (e.g. COM10); omit for --mock")
    p_health.add_argument("--mock", action="store_true", help="Force offline deterministic mock transport")

    p_cap = sub.add_parser("capture", help="Run BOOTCAP/JIGBOOT/UARTMON and decode UART stream")
    p_cap.add_argument("cmd", choices=["BOOTCAP", "JIGBOOT", "UARTMON"])
    p_cap.add_argument("--port", default=None)
    p_cap.add_argument("--mock", action="store_true")
    p_cap.add_argument("--out", type=Path, default=None)

    p_dec = sub.add_parser("decode-log", help="Decode an existing captured UART log file")
    p_dec.add_argument("path", type=Path)

    sub.add_parser("smoke-suite", help="Run offline end-to-end bench verification suite")

    p_ab = sub.add_parser("ab-boot", help="Run alternating NORMAL vs JIG boot pairs and tally faults")
    p_ab.add_argument("--port", default=None)
    p_ab.add_argument("--mock", action="store_true")
    p_ab.add_argument("--pairs", type=int, default=3)
    p_ab.add_argument("--window-ms", type=int, default=6000)
    p_ab.add_argument("--csv", type=Path, default=None)

    args = parser.parse_args(argv)
    result: JsonDict
    if args.subcommand == "health":
        result = run_health(None if args.mock else args.port)
    elif args.subcommand == "capture":
        result = run_capture(args.cmd, None if args.mock else args.port, args.out)
    elif args.subcommand == "decode-log":
        text = read_captured_log(args.path)
        result = cast(JsonDict, asdict(parse_am34_uart_log(text)))
    elif args.subcommand == "ab-boot":
        summary = run_ab_boot_harness(
            _make_transport(None if args.mock else args.port),
            pairs=args.pairs,
            window_ms=args.window_ms,
        )
        if args.csv is not None:
            args.csv.parent.mkdir(parents=True, exist_ok=True)
            args.csv.write_text(format_ab_csv(summary.runs), encoding="utf-8")
        result = {
            "runs": [cast(JsonValue, asdict(r)) for r in summary.runs],
            "by_mode": {k: cast(JsonValue, asdict(v)) for k, v in summary.by_mode.items()},
        }
    else:
        result = run_smoke_suite()

    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
