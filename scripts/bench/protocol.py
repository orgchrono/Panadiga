"""Non-destructive transport and session management for Pico P7501 bench probing."""

from __future__ import annotations

from dataclasses import dataclass, field
import importlib
import time
from typing import Final, Protocol, cast

from scripts.panadiga_cli_bridge import PanadigaCliError, run_panadiga_json

SAFE_BENCH_COMMANDS: Final[tuple[str, ...]] = (
    "PING",
    "HELP",
    "BANNER",
    "STATUS",
    "VDD",
    "LED",
    "PINCHECK",
    "BUSDIAG",
    "SNIFF",
    "ISOLATE",
    "UARTMON",
    "BOOTCAP",
    "JIGBOOT",
    "UARTPROBE",
    "UARTSEND",
    "BAUD",
    "FRAMING",
    "BAUDSCAN",
    "UARTSTREAM",
    "HEXCAP",
    "UARTTXHEX",
    "WAITTX",
    "PWRTRACE",
    "BOOTTRACE",
    "PIOLOGIC",
    "SDSNIFF",
    "JIGSWEEP",
    "DIAGWATCH",
    "PANARSP",
)

_PROMPT_MARKERS: Final[tuple[str, ...]] = ("P7501>", "PANADIGA>")


class BenchSafetyError(RuntimeError):
    """Raised when a command violates non-destructive bench safety rules."""


class BenchProtocolError(RuntimeError):
    """Raised on serial transport or prompt-framing failures."""


class _SerialPortLike(Protocol):
    """Minimal subset of pyserial.Serial used by SerialTransport."""

    timeout: float | None

    def write(self, data: bytes) -> int | None: ...
    def read(self, size: int = 1) -> bytes: ...
    def reset_input_buffer(self) -> None: ...
    def close(self) -> None: ...


class TransportProtocol(Protocol):
    """Abstract line-based command transport for live or mocked Pico probes."""

    def exchange(self, command: str, timeout_s: float = 4.0) -> str:
        """Send one validated bench command and collect output until prompt or timeout."""
        ...

    def close(self) -> None:
        """Release underlying communication resources."""
        ...


def validate_safe_bench_command(command: str) -> str:
    """Validate `command` against Rust `SAFE_BENCH_COMMANDS` via `panadiga pico validate-bench-cmd`."""
    try:
        res = run_panadiga_json(["pico", "validate-bench-cmd", "--cmd", command])
    except PanadigaCliError as exc:
        raise BenchSafetyError(str(exc)) from exc
    if not isinstance(res, dict):
        raise BenchSafetyError("Unexpected validation reply")
    norm = res.get("normalized")
    if not isinstance(norm, str):
        raise BenchSafetyError("Missing normalized command string")
    return norm


class SerialTransport:
    """PySerial-backed transport for a live Pico connected over USB CDC (e.g. COM10)."""

    def __init__(self, port: str, baudrate: int = 115200) -> None:
        serial_mod = importlib.import_module("serial")
        serial_cls = getattr(serial_mod, "Serial")
        self._port_name = port
        self._ser = cast(_SerialPortLike, serial_cls(port, baudrate=baudrate, timeout=0.1))
        self._ser.reset_input_buffer()

    def exchange(self, command: str, timeout_s: float = 4.0) -> str:
        """Transmit one validated command over USB CDC and read until prompt or deadline."""
        cmd = validate_safe_bench_command(command)
        self._ser.reset_input_buffer()
        self._ser.write((cmd + "\r\n").encode("ascii"))

        chunks: list[str] = []
        deadline = time.monotonic() + timeout_s
        while time.monotonic() < deadline:
            raw = self._ser.read(4096)
            if raw:
                text = raw.decode("utf-8", errors="replace")
                chunks.append(text)
                joined = "".join(chunks)
                if any(marker in joined for marker in _PROMPT_MARKERS):
                    return joined
            else:
                time.sleep(0.02)
        return "".join(chunks)

    def close(self) -> None:
        """Close the USB CDC serial handle."""
        self._ser.close()


def _empty_str_list() -> list[str]:
    return []


@dataclass
class MockBenchTransport:
    """Deterministic offline simulator matching empirical COM10 DMR-EH55 telemetry."""

    vdd_volts: float = 2.70
    simulate_gp26_sticky_low: bool = False
    target_is_normal_bgo_brick: bool = True
    sd_status_code: int = -91006
    history: list[str] = field(default_factory=_empty_str_list)
    _gp26_latched_low: bool = field(init=False, default=False)

    def exchange(self, command: str, timeout_s: float = 4.0) -> str:
        """Simulate Pico P7501 command execution offline for CI and regression tests."""
        del timeout_s
        cmd = validate_safe_bench_command(command)
        self.history.append(cmd)
        parts = cmd.split()
        head = parts[0].upper()

        if head == "PING":
            return "PING\r\nPONG\r\nP7501> "
        if head in ("ISOLATE", "UARTPROBE"):
            if self.simulate_gp26_sticky_low and head == "UARTPROBE":
                self._gp26_latched_low = False
            if head == "ISOLATE":
                return (
                    "ISOLATE\r\n"
                    "ISOLATE: OK (All 6 pins in High-Impedance Hi-Z mode; unified JST ZR-6 plug is electrically invisible)\r\n"
                    "P7501> "
                )
            return "=== UARTPROBE MODE 8 CAPTURE (0 bytes) ===\r\nUARTPROBE: OK\r\nP7501> "
        if head in ("STATUS", "VDD"):
            vdd = 0.02 if self._gp26_latched_low else self.vdd_volts
            state = "LOW" if vdd < 1.5 else "OK"
            return (
                f"{head}\r\n"
                f"VDD: {vdd:.2f}V ({state})\r\n"
                "  [STATUS] Lines static: GP2=1, GP3=1, GP4=1, GP5=1.\r\n"
                "P7501> "
            )
        if head == "PINCHECK":
            return self._render_pincheck()
        if head == "BUSDIAG":
            return (
                "BUSDIAG\r\n"
                "GP2 (Pin6 MAX TXD / UART RX): 872 transitions (250 ms)\r\n"
                "GP3 (Pin5 NSW5V / Static Hi-Z): 0 transitions (5.15V rail idle)\r\n"
                "GP4 (Pin3 MAX RXD / UART TX): 0 transitions (R7570 10k idle HIGH)\r\n"
                "GP5 (Pin2 XRESET): STRONG HIGH, XRESET Release Rise Time (Hi-Z) : 20 us\r\n"
                "BUSDIAG: OK\r\n"
                "P7501> "
            )
        if head in ("BOOTCAP", "JIGBOOT", "UARTMON", "UARTSEND"):
            if self.simulate_gp26_sticky_low:
                self._gp26_latched_low = True
            return self._render_capture(head)
        if head == "PIOLOGIC":
            return self._render_piologic()
        if head == "PANARSP":
            return self._render_panarsp(cmd)
        return f"{cmd}\r\nOK\r\nP7501> "

    def _render_piologic(self) -> str:
        return (
            "PIOLOGIC\r\n"
            "=== PIOLOGIC PIO0 (4000ms @ 1000kHz, boot=2, hold=350ms) ===\r\n"
            "Initial: RXD=1 NSW5V=1 TXD=1 XRESET=0 | fifo_stall=NO | edges=28 | span=512000us\r\n"
            "  RXD   (GP2) : Final=1, Transitions=24 (rise=12 fall=12 minLow=8us minHigh=9us)\r\n"
            "  NSW5V (GP3) : Final=1, Transitions=0 (rise=0 fall=0 minLow=0us minHigh=0us)\r\n"
            "  TXD   (GP4) : Final=1, Transitions=2 (rise=1 fall=1 minLow=40us minHigh=100us)\r\n"
            "  XRESET(GP5) : Final=1, Transitions=2 (rise=1 fall=1 minLow=30000us minHigh=0us)\r\n"
            "[PIOLOGIC] XRESET_rise -> first_RXD_fall: 482 us\r\n"
            "[PIOLOGIC] TXD_fall -> RXD_fall turnaround: min=120 us max=120 us\r\n"
            "PIOLOGIC: OK\r\n"
            "P7501> "
        )

    def _render_panarsp(self, cmd: str) -> str:
        payload = "DEADBEEF" if "MEM:" in cmd.upper() else "OK"
        csum = sum(payload.encode("ascii")) & 0xFF
        return (
            f"{cmd}\r\n"
            "=== PANARSP PIO1 (baud=115200 ms=6000 boot=2 hold=350 action=0) ===\r\n"
            f"[PANARSP] RX ({len(payload) + 6} bytes, pretrig_dropped=0, inspect_from=0):\r\n"
            f"[PANARSP] Frame: ack=+ nak=no nak0x15=yes | payload=\"{payload}\" | csum=0x{csum:02X}/0x{csum:02X} (VALID)\r\n"
            f"[PANARSP] PayloadHex: {payload}\r\n"
            f"PANARSP: OK (rx={len(payload) + 6} bytes, rsp=VALID, fifo_ovf=NO)\r\n"
            "P7501> "
        )

    def _render_pincheck(self) -> str:
        if self._gp26_latched_low:
            gp26 = "GP26 (Pin1 SW_UART) | 0.02V | 0.02V | 0.02V | STICKY LOW BUG"
        else:
            gp26 = "GP26 (Pin1 SW_UART) | 3.25V | 2.69V | 3.27V | Weak pull-up (Q7510 B-C clamp)"
        return (
            "PINCHECK\r\n"
            "=== P7501 6-PIN ELECTRICAL MATRIX (PINCHECK) ===\r\n"
            "Pin               | Hi-Z | Pull-Down | Pull-Up | Electrical State\r\n"
            f"{gp26}\r\n"
            "GP3 (Pin5 NSW5V   ) | 3.29V | 3.28V | 3.30V | Hi-Z Sense ONLY (Static rail ~5.15V upstream)\r\n"
            "GP4 (Pin3 MAX RXD ) | 3.28V | 2.91V | 3.29V | Idle HIGH (R7570 10k / Q7514)\r\n"
            "GP5 (Pin2 XRESET  ) | 3.29V | 2.85V | 3.30V | STRONG HIGH (Open-Drain control)\r\n"
            "PINCHECK: OK\r\n"
            "P7501> "
        )

    def _render_capture(self, head: str) -> str:
        if head == "JIGBOOT":
            return (
                "JIGBOOT\r\n"
                "DDR CHECK: OK\r\n"
                "Chip select = 1\r\n"
                "***** Loader ver0.61 [06/03/06] *****\r\n"
                "***** serial boot mode *****\r\n"
                "!!! start fmu to read & exec fvu. !!!\r\n"
                "wait 1 second.\r\n"
                "*** timer correspond initialize ***\r\n"
                "startup reason = [70]\r\n"
                "Main hung-up time changed 255[s].\r\n"
                "SD init start...\r\n"
                f"[SDCD]SetBusWidth:SD Status ERROR {self.sd_status_code}\r\n"
                "SD init failed[-90002].\r\n"
                "*** CD diag boot mode ***\r\n"
                "[RTSD] ATAPI Software Reset \r\n"
                "X(80062900)X(80023a00)** startup drive **\r\n"
                "X(80020401)59,[RTSD] === DRIVE ACCESS ERROR === ( 80040904 )\r\n"
                "*** CDB  [00 00 00 00 00 00 00 00 00 00 00 00] \r\n"
                "*** SENSE[70 00 04 00 00 00 00 0A 00 00 83 A7 09 04 00 00 00 00] \r\n"
                "X(80040904)0,Drive start failed[-2147219196].\r\n"
                f"{head}: OK\r\n"
                "P7501> "
            )
        bgo_line = "BGO driver init failed[1].\r\n" if self.target_is_normal_bgo_brick else "BGO driver init OK\r\n"
        return (
            f"{head}\r\n"
            "DDR CHECK: OK\r\n"
            "***** Loader ver0.61 [06/03/06] *****\r\n"
            "***** normal mode *****\r\n"
            f"{bgo_line}"
            f"{head}: OK\r\n"
            "P7501> "
        )

    def close(self) -> None:
        """No-op close for mock transport."""


class BenchSession:
    """Context manager guaranteeing bus Hi-Z isolation (`ISOLATE`) upon exit."""

    def __init__(self, transport: TransportProtocol) -> None:
        self._transport = transport

    def __enter__(self) -> BenchSession:
        return self

    def send(self, command: str, timeout_s: float = 4.0) -> str:
        """Execute a validated safe bench command."""
        return self._transport.exchange(command, timeout_s=timeout_s)

    def __exit__(self, exc_type: object, exc: object, tb: object) -> None:
        del exc_type, exc, tb
        try:
            self._transport.exchange("ISOLATE", timeout_s=2.0)
        finally:
            self._transport.close()
