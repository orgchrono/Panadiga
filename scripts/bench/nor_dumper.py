"""Thin typed Python facade delegating MN103S UART NOR dumper to `panadiga unbrick --json`."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final

from scripts.panadiga_cli_bridge import (
    PanadigaCliError,
    as_dict,
    as_list,
    req_int,
    req_str,
    run_panadiga_json,
)

NOR_MAGIC_HEADER: Final[bytes] = b"PANADMP1"
NOR_MAGIC_FOOTER: Final[bytes] = b"PANADONE"
NOR_ALLOWED_WINDOWS: Final[tuple[tuple[int, int], ...]] = (
    (0x80000000, 0x80800000),
    (0x84000000, 0x84800000),
)
DEFAULT_NOR_STUB_LOAD_ADDR: Final[int] = 0x51800000
DIGADIAG_STUB_LOAD_ADDR: Final[int] = 0x50400000


@dataclass(frozen=True, slots=True)
class NorDumpResult:
    """Verified binary NOR dump recovered from a DIGADIAG UART stream."""

    start_addr: int
    total_len: int
    block_size: int
    data: bytes
    sha256: str
    block_checksums: tuple[int, ...]


def resolve_nor_stub_load_addr(
    load_addr: int | None = None,
    *,
    wrap_digadiag: bool = False,
) -> int:
    """Select `0x50400000` for DIGADIAG.FRM payloads or `0x51800000` for raw RSP RAM dumps."""
    if load_addr is not None:
        return load_addr & 0xFFFFFFFF
    return DIGADIAG_STUB_LOAD_ADDR if wrap_digadiag else DEFAULT_NOR_STUB_LOAD_ADDR


def validate_nor_dump_range(start_addr: int, total_len: int, block_size: int) -> None:
    """Validate strictly read-only NOR bounds and alignment via native Rust `panadiga unbrick nor-stub`."""
    _ = build_readonly_nor_dumper_stub(start_addr, total_len, block_size)


def build_readonly_nor_dumper_stub(
    start_addr: int,
    total_len: int,
    block_size: int = 4096,
    load_addr: int = DEFAULT_NOR_STUB_LOAD_ADDR,
) -> bytes:
    """Assemble position-coupled read-only MN103S NOR dumper via `panadiga unbrick nor-stub --json`."""
    try:
        rep = as_dict(
            run_panadiga_json(
                [
                    "unbrick",
                    "nor-stub",
                    "--start",
                    f"0x{start_addr & 0xFFFFFFFF:08X}",
                    "--len",
                    f"0x{max(0, total_len):X}" if total_len >= 0 else str(total_len),
                    "--block-size",
                    str(max(0, block_size)),
                    "--load-addr",
                    f"0x{load_addr & 0xFFFFFFFF:08X}",
                ]
            )
        )
    except PanadigaCliError as exc:
        raise ValueError(str(exc)) from exc
    return bytes.fromhex(req_str(rep, "data_hex"))


def decode_nor_dump_stream(stream: bytes) -> NorDumpResult:
    """Parse and checksum-verify a `PANADMP1..PANADONE` stream via `panadiga unbrick decode-nor-dump`."""
    try:
        rep = as_dict(
            run_panadiga_json(["unbrick", "decode-nor-dump", "--input", "-"], stdin_bytes=stream)
        )
    except PanadigaCliError as exc:
        raise ValueError(str(exc)) from exc
    return NorDumpResult(
        start_addr=req_int(rep, "start_addr"),
        total_len=req_int(rep, "total_len"),
        block_size=req_int(rep, "block_size"),
        data=bytes.fromhex(req_str(rep, "data_hex")),
        sha256=req_str(rep, "sha256"),
        block_checksums=tuple(
            int(v) for v in as_list(rep.get("block_checksums")) if isinstance(v, int)
        ),
    )
