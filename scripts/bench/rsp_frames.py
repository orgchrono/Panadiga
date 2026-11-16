"""Thin typed Python facade delegating GDB RSP framing & safety checks to `panadiga pico rsp-codec`."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final, Protocol

from scripts.panadiga_cli_bridge import (
    PanadigaCliError,
    as_dict,
    req_bool,
    req_int,
    req_str,
    run_panadiga_json,
)

MAX_M_CHUNK: Final[int] = 512
MAX_X_CHUNK: Final[int] = 0x440
G_REGISTER_BYTES: Final[int] = 0x70


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
    """Compute standard 8-bit modulo-256 RSP checksum via `panadiga pico rsp-codec`."""
    res = run_panadiga_json(["pico", "rsp-codec", "--kind", "checksum", "--input", "-"], stdin_bytes=payload)
    if isinstance(res, bool) or not isinstance(res, int):
        raise RuntimeError(f"Unexpected checksum reply: {res!r}")
    return res


def is_ipl_readonly_address(addr: int, length: int = 1) -> bool:
    """Return True when [addr, addr + length) touches read-only IPL NOR flash."""
    res = run_panadiga_json(
        [
            "pico",
            "rsp-codec",
            "--kind",
            "is-ipl-readonly",
            "--addr",
            f"0x{addr & 0xFFFFFFFF:08X}",
            "--len",
            str(max(1, length)),
        ]
    )
    return req_bool({"ok": res}, "ok")


def build_rsp_frame(payload: str | bytes, *, allow_write: bool = False) -> bytes:
    """Encode `$<payload>#<xx>` via `panadiga pico rsp-codec --kind build-frame`."""
    raw = payload.encode("ascii") if isinstance(payload, str) else payload
    args = ["pico", "rsp-codec", "--kind", "build-frame", "--input", "-"]
    if allow_write:
        args.append("--allow-write")
    try:
        rep = as_dict(run_panadiga_json(args, stdin_bytes=raw))
    except PanadigaCliError as exc:
        raise ValueError(str(exc)) from exc
    return bytes.fromhex(req_str(rep, "data_hex"))


def build_rsp_write_frame(addr: int, data: bytes, *, use_binary_x: bool = False) -> bytes:
    """Build hardware-guarded `$M`/`$X` write frame via `panadiga pico rsp-codec --kind build-write`."""
    args = [
        "pico",
        "rsp-codec",
        "--kind",
        "build-write",
        "--addr",
        f"0x{addr & 0xFFFFFFFF:08X}",
        "--input",
        "-",
    ]
    if use_binary_x:
        args.append("--binary-x")
    try:
        rep = as_dict(run_panadiga_json(args, stdin_bytes=data))
    except PanadigaCliError as exc:
        raise ValueError(str(exc)) from exc
    return bytes.fromhex(req_str(rep, "data_hex"))


def raise_if_e0x(payload: bytes) -> None:
    """Raise RuntimeError if RSP payload matches `E00`..`E04`."""
    if len(payload) == 3 and payload[:2] == b"E0" and payload[2:3] in (b"0", b"1", b"2", b"3", b"4"):
        try:
            _ = decode_rsp_hex_memory(payload)
        except ValueError as exc:
            raise RuntimeError(str(exc)) from exc


def parse_rsp_reply(stream: bytes, *, expected_payload_len: int | None = None) -> ParsedRspFrame | None:
    """Scan a `$<payload>#<xx>` frame via `panadiga pico rsp-codec --kind parse-reply`."""
    args = ["pico", "rsp-codec", "--kind", "parse-reply", "--input", "-"]
    if expected_payload_len is not None:
        args.extend(["--expected-len", str(expected_payload_len)])
    raw = run_panadiga_json(args, stdin_bytes=stream)
    if raw is None:
        return None
    d = as_dict(raw)
    return ParsedRspFrame(
        payload=bytes.fromhex(req_str(d, "payload_hex")),
        expected_csum=req_int(d, "expected_csum"),
        actual_csum=req_int(d, "actual_csum"),
        checksum_ok=req_bool(d, "checksum_ok"),
        ack_plus=req_bool(d, "ack_plus"),
        nak_minus=req_bool(d, "nak_minus"),
        raw_nak_0x15=req_bool(d, "raw_nak_0x15"),
    )


def decode_rsp_hex_memory(payload: bytes) -> bytes:
    """Decode ASCII hex payload returned by `$m` via `panadiga pico rsp-codec --kind decode-hex`."""
    try:
        rep = as_dict(
            run_panadiga_json(["pico", "rsp-codec", "--kind", "decode-hex", "--input", "-"], stdin_bytes=payload)
        )
    except PanadigaCliError as exc:
        msg = str(exc)
        if "Target returned E0" in msg:
            raise RuntimeError(msg) from exc
        raise ValueError(msg) from exc
    return bytes.fromhex(req_str(rep, "data_hex"))


def decode_mn103s_g_registers(payload: bytes) -> dict[str, int]:
    """Decode 112-byte MN103S `$g` frame (`w00`..`w27`) via `panadiga pico rsp-codec --kind decode-g`."""
    try:
        rep = as_dict(
            run_panadiga_json(["pico", "rsp-codec", "--kind", "decode-g", "--input", "-"], stdin_bytes=payload)
        )
    except PanadigaCliError as exc:
        raise ValueError(str(exc)) from exc
    return {k: int(v) for k, v in rep.items() if isinstance(v, int) and not isinstance(v, bool)}


def validate_dump_range(
    start_addr: int,
    total_bytes: int,
    chunk_size: int,
    *,
    max_chunk_size: int = MAX_M_CHUNK,
) -> None:
    """Validate non-destructive RSP memory read bounds via `panadiga pico rsp-codec --kind validate-dump`."""
    if start_addr < 0 or start_addr > 0xFFFFFFFF:
        raise ValueError(f"Invalid 32-bit target address: 0x{start_addr:X}")
    if total_bytes < 0 or chunk_size < 0:
        raise ValueError("total_bytes and chunk_size must be positive")
    try:
        _ = run_panadiga_json(
            [
                "pico",
                "rsp-codec",
                "--kind",
                "validate-dump",
                "--addr",
                f"0x{start_addr & 0xFFFFFFFF:08X}",
                "--len",
                str(total_bytes),
                "--chunk-size",
                str(chunk_size),
                "--max-chunk-size",
                str(max_chunk_size),
            ]
        )
    except PanadigaCliError as exc:
        raise ValueError(str(exc)) from exc
