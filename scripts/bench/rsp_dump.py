"""Read-only GDB RSP (`$m` / `$x` / `$g`) memory dumper and bridge stream helpers for AM34."""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import importlib
import struct
import time
from typing import Final, Protocol

_E0X_ERRORS: Final[dict[str, str]] = {
    "E00": "RSP malformed/bad length",
    "E01": "RSP invalid address or bad argument",
    "E02": "RSP bus/alignment fault",
    "E03": "RSP unsupported command",
    "E04": "RSP overrun / buffer limit",
}
_E0X_BYTES: Final[dict[bytes, str]] = {k.encode("ascii"): v for k, v in _E0X_ERRORS.items()}
_MAX_M_CHUNK: Final[int] = 512
_MAX_X_CHUNK: Final[int] = 0x440
_G_REGISTER_BYTES: Final[int] = 0x70
_G_REGISTER_WORDS: Final[int] = 28


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


def _try_build_parsed_frame(
    payload: bytes,
    hex_pair: bytes,
    *,
    ack_plus: bool,
    nak_minus: bool,
    raw_nak: bool,
) -> ParsedRspFrame | None:
    if len(hex_pair) != 2:
        return None
    try:
        actual = int(hex_pair.decode("ascii"), 16)
    except (UnicodeDecodeError, ValueError):
        return None
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


def _raise_if_e0x(payload: bytes) -> None:
    if payload in _E0X_BYTES:
        code = payload.decode("ascii")
        raise RuntimeError(f"Target returned {code}: {_E0X_BYTES[payload]}")


def parse_rsp_reply(stream: bytes, *, expected_payload_len: int | None = None) -> ParsedRspFrame | None:
    """Find a complete `$<payload>#<xx>` frame, preferring checksum-verified `#<hh>` for `$x` binary data."""
    search_idx = 0
    first_mismatch: ParsedRspFrame | None = None

    while True:
        dollar = stream.find(b"$", search_idx)
        if dollar < 0:
            return first_mismatch

        prefix = stream[:dollar]
        ack_plus = b"+" in prefix
        nak_minus = b"-" in prefix
        raw_nak = b"\x15" in prefix
        start = dollar + 1

        if expected_payload_len is not None:
            if (start + 6) <= len(stream) and stream[start : start + 3] in _E0X_BYTES and stream[start + 3 : start + 4] == b"#":
                err_frame = _try_build_parsed_frame(
                    stream[start : start + 3],
                    stream[start + 4 : start + 6],
                    ack_plus=ack_plus,
                    nak_minus=nak_minus,
                    raw_nak=raw_nak,
                )
                if err_frame is not None and err_frame.checksum_ok:
                    return err_frame

            exact_hash = start + expected_payload_len
            if (exact_hash + 3) > len(stream):
                return first_mismatch
            if stream[exact_hash : exact_hash + 1] == b"#":
                exact_frame = _try_build_parsed_frame(
                    stream[start:exact_hash],
                    stream[exact_hash + 1 : exact_hash + 3],
                    ack_plus=ack_plus,
                    nak_minus=nak_minus,
                    raw_nak=raw_nak,
                )
                if exact_frame is not None:
                    return exact_frame

        pos = start
        while True:
            hash_pos = stream.find(b"#", pos)
            if hash_pos < 0 or (hash_pos + 2) >= len(stream):
                break
            cand = _try_build_parsed_frame(
                stream[start:hash_pos],
                stream[hash_pos + 1 : hash_pos + 3],
                ack_plus=ack_plus,
                nak_minus=nak_minus,
                raw_nak=raw_nak,
            )
            if cand is not None:
                if cand.checksum_ok:
                    return cand
                if first_mismatch is None:
                    first_mismatch = cand
            pos = hash_pos + 1

        search_idx = dollar + 1


def decode_rsp_hex_memory(payload: bytes) -> bytes:
    """Decode ASCII hex memory payload returned by `$m<addr>,<len>`, raising on `E0x` errors."""
    _raise_if_e0x(payload)
    text = payload.decode("ascii", errors="strict")
    if len(text) == 0 or (len(text) % 2) != 0:
        raise ValueError(f"Invalid even hex payload length: {len(text)}")
    return bytes.fromhex(text)


def decode_mn103s_g_registers(payload: bytes) -> dict[str, int]:
    """Decode an AM34 112-byte (`0x70`, `28 x <I` LE) `$g` register frame (`w00`..`w27`).

    Word indices `[25]` (`0x64`), `[26]` (`0x68`), and `[27]` (`0x6c`) correspond
    to the tail fields patched by `FUN_574f0da3` in `EH55_MAIN`.
    """
    raw = decode_rsp_hex_memory(payload)
    if len(raw) != _G_REGISTER_BYTES:
        raise ValueError(
            f"Expected {_G_REGISTER_BYTES}-byte MN103S $g register frame, got {len(raw)} bytes"
        )
    words = struct.unpack(f"<{_G_REGISTER_WORDS}I", raw)
    return {f"w{idx:02d}": int(val) for idx, val in enumerate(words)}


def validate_dump_range(
    start_addr: int,
    total_bytes: int,
    chunk_size: int,
    *,
    max_chunk_size: int = _MAX_M_CHUNK,
) -> None:
    """Validate non-destructive RSP memory read bounds."""
    if start_addr < 0 or start_addr > 0xFFFFFFFF:
        raise ValueError(f"Invalid 32-bit target address: 0x{start_addr:X}")
    if total_bytes <= 0:
        raise ValueError("total_bytes must be positive")
    if chunk_size <= 0 or chunk_size > max_chunk_size:
        raise ValueError(
            f"chunk_size must be in 1..{max_chunk_size} for reliable UART RSP framing"
        )


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
            regs = "00" * _G_REGISTER_BYTES
            self._rx.extend(b"+" + build_rsp_frame(regs))
        elif text in ("Z0", "Z1"):
            self._rx.extend(b"+" + build_rsp_frame(b"OK"))
            if text == "Z0":
                self._in_loop = False
        elif text[:1] in ("m", "x") and "," in text:
            addr_s, len_s = text[1:].split(",", 1)
            addr = int(addr_s, 16)
            length = int(len_s, 16)
            off = addr - self._base
            max_chunk = _MAX_X_CHUNK if text[0] == "x" else _MAX_M_CHUNK
            if off < 0 or length <= 0 or length > max_chunk or (off + length) > len(self._mem):
                self._rx.extend(b"+" + build_rsp_frame(b"E01"))
                return
            self._chunk_seen += 1
            mem_slice = self._mem[off : off + length]
            reply_payload: str | bytes = mem_slice if text[0] == "x" else mem_slice.hex()
            raw_reply = build_rsp_frame(reply_payload)
            if self._corrupt_idx == self._chunk_seen:
                self._corrupt_idx = None
                bad_csum = b"00" if raw_reply.endswith(b"ff") else b"ff"
                raw_reply = raw_reply[:-2] + bad_csum
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

    def _read_frame(self, *, expected_payload_len: int | None = None) -> ParsedRspFrame:
        buf = bytearray()
        deadline = time.monotonic() + self._timeout_s
        while time.monotonic() < deadline:
            chunk = self._stream.read(512)
            if chunk:
                buf.extend(chunk)
                parsed = parse_rsp_reply(bytes(buf), expected_payload_len=expected_payload_len)
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

    def read_registers(self, *, max_retries: int = 3) -> dict[str, int]:
        """Query `$g#67` within an active stub session and decode 28 MN103S register words."""
        reply = self._exchange_with_retry("g", max_retries=max_retries)
        return decode_mn103s_g_registers(reply.payload)

    def read_memory(
        self,
        start_addr: int,
        total_bytes: int,
        chunk_size: int = 512,
        *,
        use_magic: bool = False,
        use_binary_x: bool = False,
        exit_stub: bool = True,
        max_retries: int = 3,
    ) -> DumpExtractionResult:
        """Read `[start_addr .. start_addr + total_bytes)` via `$m` or binary `$x` chunks with CRC retries."""
        max_chunk = _MAX_X_CHUNK if use_binary_x else _MAX_M_CHUNK
        validate_dump_range(start_addr, total_bytes, chunk_size, max_chunk_size=max_chunk)
        self.enter_stub(use_magic=use_magic)

        opcode = "x" if use_binary_x else "m"
        assembled = bytearray()
        chunks_ok = 0
        retries = 0
        exit_ok = False
        try:
            offset = 0
            while offset < total_bytes:
                req_len = min(chunk_size, total_bytes - offset)
                req_payload = f"{opcode}{start_addr + offset:x},{req_len:x}"
                req_bytes = build_rsp_frame(req_payload)
                chunk_bytes: bytes | None = None
                for attempt in range(max_retries + 1):
                    self._stream.write(req_bytes)
                    reply = self._read_frame(expected_payload_len=req_len if use_binary_x else None)
                    if reply.checksum_ok:
                        self._stream.write(b"+")
                        if use_binary_x:
                            if len(reply.payload) != req_len:
                                _raise_if_e0x(reply.payload)
                                raise RuntimeError(
                                    f"Unexpected $x payload length at 0x{start_addr + offset:08X}: "
                                    f"expected {req_len}, got {len(reply.payload)}"
                                )
                            chunk_bytes = reply.payload
                        else:
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
