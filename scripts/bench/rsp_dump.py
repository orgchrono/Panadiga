"""Read-only GDB RSP ($m) chunked memory dumper and bridge stream helpers for AM34."""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import importlib
import time
from typing import Final, Protocol

_E0X_ERRORS: Final[dict[str, str]] = {
    "E00": "RSP malformed/bad length",
    "E01": "RSP invalid address or bad argument",
    "E02": "RSP bus/alignment fault",
    "E03": "RSP unsupported command",
    "E04": "RSP overrun / buffer limit",
}


class RawByteStream(Protocol):
    """Minimal byte-oriented transport contract for UART BRIDGE / mock RSP targets."""

    def write(self, data: bytes) -> int | None: ...
    def read(self, size: int = 1) -> bytes: ...
    def close(self) -> None: ...


@dataclass(frozen=True)
class ParsedRspFrame:
    """Decoded RSP reply frame extracted from raw UART byte stream."""

    payload: bytes
    expected_csum: int
    actual_csum: int
    checksum_ok: bool
    ack_plus: bool
    nak_minus: bool
    raw_nak_0x15: bool


@dataclass(frozen=True)
class DumpExtractionResult:
    """Verified full or slice read-only memory dump extracted via `_dev_SERIAL` RSP."""

    start_addr: int
    data: bytes
    sha256: str
    chunks_ok: int
    retries: int
    exit_ok: bool


def rsp_checksum(payload: bytes) -> int:
    """Compute standard 8-bit modulo-256 RSP checksum."""
    return sum(payload) & 0xFF


def build_rsp_frame(payload: str | bytes) -> bytes:
    """Encode a read-only RSP command payload as `$<payload>#<xx>`."""
    raw = payload.encode("ascii") if isinstance(payload, str) else payload
    if raw[:1] in (b"M", b"X", b"c", b"s", b"k"):
        raise ValueError(f"Mutating or execution RSP command '{raw[:1].decode('ascii')}' is forbidden")
    csum = rsp_checksum(raw)
    return b"$" + raw + f"#{csum:02x}".encode("ascii")


def parse_rsp_reply(stream: bytes) -> ParsedRspFrame | None:
    """Find the first syntactically complete `$<payload>#<xx>` frame and validate checksum."""
    ack_plus = b"+" in stream
    nak_minus = b"-" in stream
    raw_nak = b"\x15" in stream

    search_idx = 0
    while True:
        dollar = stream.find(b"$", search_idx)
        if dollar < 0:
            return None
        hash_pos = stream.find(b"#", dollar + 1)
        if hash_pos < 0 or (hash_pos + 2) >= len(stream):
            return None
        hex_pair = stream[hash_pos + 1 : hash_pos + 3]
        try:
            actual = int(hex_pair.decode("ascii"), 16)
        except (UnicodeDecodeError, ValueError):
            search_idx = dollar + 1
            continue
        payload = stream[dollar + 1 : hash_pos]
        expected = rsp_checksum(payload)
        return ParsedRspFrame(
            payload=payload,
            expected_csum=expected,
            actual_csum=actual,
            checksum_ok=(expected == actual),
            ack_plus=ack_plus,
            nak_minus=nak_minus,
            raw_nak_0x15=raw_nak,
        )


def decode_rsp_hex_memory(payload: bytes) -> bytes:
    """Decode ASCII hex memory payload returned by `$m<addr>,<len>`, raising on `E0x` errors."""
    text = payload.decode("ascii", errors="strict")
    if text in _E0X_ERRORS:
        raise RuntimeError(f"Target returned {text}: {_E0X_ERRORS[text]}")
    if len(text) == 0 or (len(text) % 2) != 0:
        raise ValueError(f"Invalid even hex payload length: {len(text)}")
    return bytes.fromhex(text)


def validate_dump_range(start_addr: int, total_bytes: int, chunk_size: int) -> None:
    """Validate non-destructive RSP memory read bounds."""
    if start_addr < 0 or start_addr > 0xFFFFFFFF:
        raise ValueError(f"Invalid 32-bit target address: 0x{start_addr:X}")
    if total_bytes <= 0:
        raise ValueError("total_bytes must be positive")
    if chunk_size <= 0 or chunk_size > 512:
        raise ValueError("chunk_size must be in 1..512 for reliable UART RSP framing")


class BridgeByteStream:
    """Context-managed Pico `BRIDGE` raw passthrough ending deterministically with `~~~` + `ISOLATE`."""

    def __init__(self, port: str, baud: int = 115200, boot_mode: str = "") -> None:
        serial_mod = importlib.import_module("serial")
        serial_cls = getattr(serial_mod, "Serial")
        self._ser: RawByteStream = serial_cls(port, baudrate=115200, timeout=0.05)
        cmd = f"BRIDGE {baud} {boot_mode}".strip() + "\r\n"
        self._ser.write(cmd.encode("ascii"))
        self._drain_until(b"active", timeout_s=2.0)

    def _drain_until(self, marker: bytes, timeout_s: float) -> bytes:
        buf = bytearray()
        deadline = time.monotonic() + timeout_s
        while time.monotonic() < deadline:
            chunk = self._ser.read(1024)
            if chunk:
                buf.extend(chunk)
                if marker in buf:
                    break
            else:
                time.sleep(0.01)
        return bytes(buf)

    def write(self, data: bytes) -> int | None:
        """Forward raw bytes into active Pico `BRIDGE` session."""
        return self._ser.write(data)

    def read(self, size: int = 1) -> bytes:
        """Read raw UART bytes streamed by P7501 SC0 through `BRIDGE`."""
        return self._ser.read(size)

    def close(self) -> None:
        """Exit `BRIDGE` via `~~~`, enforce `ISOLATE`, and close serial device."""
        try:
            self._ser.write(b"~~~")
            self._drain_until(b"BRIDGE: OK", timeout_s=1.5)
            self._ser.write(b"ISOLATE\r\n")
            self._drain_until(b"ISOLATE: OK", timeout_s=1.0)
        finally:
            self._ser.close()


class MockAm34RspStream:
    """Deterministic AM34 `_dev_SERIAL` simulator (`FUN_574f0590`) for CI and dry-runs."""

    def __init__(
        self,
        base_addr: int,
        memory_image: bytes,
        *,
        boot_noise: bytes = b"DDR CHECK: OK\r\nwait 1 second.\r\n",
        corrupt_chunk_index: int | None = None,
    ) -> None:
        self._base = base_addr
        self._mem = memory_image
        self._rx = bytearray(boot_noise)
        self._corrupt_idx = corrupt_chunk_index
        self._chunk_seen = 0
        self._in_loop = False
        self._closed = False
        self.tx_history: list[bytes] = []

    def write(self, data: bytes) -> int | None:
        """Simulate AM34 RSP frame parsing and reply generation."""
        self.tx_history.append(data)
        if data == b"\x15":
            self._in_loop = True
            self._rx.extend(b"+" + build_rsp_frame(b"OK"))
            return len(data)
        if not self._in_loop:
            return len(data)
        idx = 0
        while idx < len(data):
            if data[idx : idx + 1] in (b"+", b"-", b"\x0e", b"\x04", b"\x02"):
                idx += 1
                continue
            frame = parse_rsp_reply(data[idx:])
            if frame is None:
                break
            self._handle_payload(frame.payload)
            hash_rel = data[idx:].find(b"#")
            idx += (hash_rel + 3) if hash_rel >= 0 else len(data)
        return len(data)

    def _handle_payload(self, payload: bytes) -> None:
        text = payload.decode("ascii", errors="replace")
        if text == "g":
            regs = "00" * 32
            self._rx.extend(b"+" + build_rsp_frame(regs))
        elif text in ("Z0", "Z1"):
            self._rx.extend(b"+" + build_rsp_frame(b"OK"))
            if text == "Z0":
                self._in_loop = False
        elif text.startswith("m") and "," in text:
            addr_s, len_s = text[1:].split(",", 1)
            addr = int(addr_s, 16)
            length = int(len_s, 16)
            off = addr - self._base
            if off < 0 or (off + length) > len(self._mem):
                self._rx.extend(b"+" + build_rsp_frame(b"E01"))
                return
            self._chunk_seen += 1
            raw_reply = build_rsp_frame(self._mem[off : off + length].hex())
            if self._corrupt_idx == self._chunk_seen:
                self._corrupt_idx = None
                raw_reply = raw_reply[:-2] + b"ff"
            self._rx.extend(b"+" + raw_reply)
        else:
            self._rx.extend(b"+" + build_rsp_frame(b"E03"))

    def read(self, size: int = 1) -> bytes:
        """Pop simulated RX bytes."""
        out = bytes(self._rx[:size])
        del self._rx[:size]
        return out

    def close(self) -> None:
        """Mark simulator stream closed."""
        self._closed = True


class ReadOnlyRspDumper:
    """Strictly read-only GDB RSP client extracting full AM34 memory ranges reliably."""

    def __init__(self, stream: RawByteStream, *, reply_timeout_s: float = 1.0) -> None:
        self._stream = stream
        self._timeout_s = reply_timeout_s

    def _read_frame(self) -> ParsedRspFrame:
        buf = bytearray()
        deadline = time.monotonic() + self._timeout_s
        while time.monotonic() < deadline:
            chunk = self._stream.read(512)
            if chunk:
                buf.extend(chunk)
                parsed = parse_rsp_reply(bytes(buf))
                if parsed is not None:
                    return parsed
            else:
                time.sleep(0.002)
        raise TimeoutError(f"RSP reply timeout after {len(buf)} RX bytes")

    def enter_stub(self, *, use_magic: bool = False) -> ParsedRspFrame:
        """Drain banner chatter and send optional `0E0402` + `0x15` NAK to enter `_dev_SERIAL` RSP loop."""
        while self._stream.read(1024):
            pass
        if use_magic:
            self._stream.write(bytes((0x0E, 0x04, 0x02)))
            time.sleep(0.02)
            while self._stream.read(1024):
                pass
        self._stream.write(b"\x15")
        return self._read_frame()

    def _exchange_with_retry(self, payload: str, *, max_retries: int) -> ParsedRspFrame:
        req = build_rsp_frame(payload)
        for attempt in range(max_retries + 1):
            self._stream.write(req)
            frame = self._read_frame()
            if frame.checksum_ok:
                self._stream.write(b"+")
                return frame
            self._stream.write(b"-")
            if attempt >= max_retries:
                raise RuntimeError(f"RSP checksum failure on '{payload}' after {max_retries + 1} attempts")
        raise AssertionError("unreachable")

    def read_memory(
        self,
        start_addr: int,
        total_bytes: int,
        chunk_size: int = 256,
        *,
        use_magic: bool = False,
        exit_stub: bool = True,
        max_retries: int = 3,
    ) -> DumpExtractionResult:
        """Read `[start_addr .. start_addr + total_bytes)` via `$m` chunks with CRC retries and SHA-256."""
        validate_dump_range(start_addr, total_bytes, chunk_size)
        self.enter_stub(use_magic=use_magic)

        assembled = bytearray()
        chunks_ok = 0
        retries = 0
        exit_ok = False
        try:
            offset = 0
            while offset < total_bytes:
                req_len = min(chunk_size, total_bytes - offset)
                req_payload = f"m{start_addr + offset:x},{req_len:x}"
                req_bytes = build_rsp_frame(req_payload)
                chunk_bytes: bytes | None = None
                for attempt in range(max_retries + 1):
                    self._stream.write(req_bytes)
                    reply = self._read_frame()
                    if reply.checksum_ok:
                        self._stream.write(b"+")
                        chunk_bytes = decode_rsp_hex_memory(reply.payload)
                        break
                    retries += 1
                    self._stream.write(b"-")
                    if attempt >= max_retries:
                        raise RuntimeError(f"Chunk failure at 0x{start_addr + offset:08X}")
                assert chunk_bytes is not None
                assembled.extend(chunk_bytes)
                offset += len(chunk_bytes)
                chunks_ok += 1
            if exit_stub:
                z_reply = self._exchange_with_retry("Z0", max_retries=max_retries)
                exit_ok = z_reply.payload == b"OK"
        finally:
            self._stream.close()

        raw_out = bytes(assembled)
        digest = hashlib.sha256(raw_out).hexdigest()
        return DumpExtractionResult(
            start_addr=start_addr,
            data=raw_out,
            sha256=digest,
            chunks_ok=chunks_ok,
            retries=retries,
            exit_ok=exit_ok,
        )
