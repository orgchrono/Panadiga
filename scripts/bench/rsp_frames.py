"""GDB RSP frame codec, MN103S register decoder, and read-only safety validation helpers."""

from __future__ import annotations

from dataclasses import dataclass
import struct
from typing import Final, Protocol

_E0X_ERRORS: Final[dict[str, str]] = {
    "E00": "RSP malformed/bad length",
    "E01": "RSP invalid address or bad argument",
    "E02": "RSP bus/alignment fault",
    "E03": "RSP unsupported command",
    "E04": "RSP overrun / buffer limit",
}
_E0X_BYTES: Final[dict[bytes, str]] = {k.encode("ascii"): v for k, v in _E0X_ERRORS.items()}
MAX_M_CHUNK: Final[int] = 512
MAX_X_CHUNK: Final[int] = 0x440
G_REGISTER_BYTES: Final[int] = 0x70
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


_IPL_READONLY_RANGES: Final[tuple[tuple[int, int], ...]] = (
    (0x80000000, 0x807F0000),
    (0x84000000, 0x847F0000),
)


def is_ipl_readonly_address(addr: int, length: int = 1) -> bool:
    """Return True if [addr, addr + length) touches read-only IPL NOR flash (0x80000000..0x807EFFFF or 0x84..)."""
    u = addr & 0xFFFFFFFF
    end = (u + max(1, length)) & 0xFFFFFFFF
    if end < u:
        end = 0x100000000
    return any(u < hi and end > lo for lo, hi in _IPL_READONLY_RANGES)


def build_rsp_frame(payload: str | bytes, *, allow_write: bool = False) -> bytes:
    """Encode a read-only (or explicitly guarded write) RSP command payload as `$<payload>#<xx>`."""
    raw = payload.encode("ascii") if isinstance(payload, str) else payload
    if raw[:1] in (b"M", b"X", b"c", b"s", b"k"):
        if not allow_write:
            raise ValueError(
                f"Mutating or execution RSP command '{raw[:1].decode('ascii')}' requires allow_write=True"
            )
    csum = rsp_checksum(raw)
    return b"$" + raw + f"#{csum:02x}".encode("ascii")


def build_rsp_write_frame(addr: int, data: bytes, *, use_binary_x: bool = False) -> bytes:
    """Build a hardware-guarded RSP memory write ($M or $X), rejecting IPL NOR regions."""
    if not data:
        raise ValueError("Write payload cannot be empty")
    u = addr & 0xFFFFFFFF
    if is_ipl_readonly_address(u, len(data)):
        raise ValueError(
            f"Hardware Safety Lockout (E05): refusing RSP write to read-only IPL NOR 0x{u:08X}"
        )
    if use_binary_x:
        escaped = bytearray()
        for b in data:
            if b in (0x23, 0x24, 0x2A, 0x7D):
                escaped.append(0x7D)
                escaped.append(b ^ 0x20)
            else:
                escaped.append(b)
        header = f"X{u:x},{len(data):x}:".encode("ascii")
        return build_rsp_frame(header + bytes(escaped), allow_write=True)
    header_hex = f"M{u:x},{len(data):x}:{data.hex()}"
    return build_rsp_frame(header_hex, allow_write=True)


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


def raise_if_e0x(payload: bytes) -> None:
    """Raise a descriptive RuntimeError when the AM34 RSP target replies with `E00`..`E04`."""
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
    raise_if_e0x(payload)
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
    if len(raw) != G_REGISTER_BYTES:
        raise ValueError(
            f"Expected {G_REGISTER_BYTES}-byte MN103S $g register frame, got {len(raw)} bytes"
        )
    words = struct.unpack(f"<{_G_REGISTER_WORDS}I", raw)
    return {f"w{idx:02d}": int(val) for idx, val in enumerate(words)}


def validate_dump_range(
    start_addr: int,
    total_bytes: int,
    chunk_size: int,
    *,
    max_chunk_size: int = MAX_M_CHUNK,
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
