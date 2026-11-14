"""Read-only MN103S DIGADIAG UART NOR flash dumper stub builder and stream decoder."""

from __future__ import annotations

import hashlib
import struct
from dataclasses import dataclass
from typing import Final

NOR_MAGIC_HEADER: Final[bytes] = b"PANADMP1"
NOR_MAGIC_FOOTER: Final[bytes] = b"PANADONE"
NOR_ALLOWED_WINDOWS: Final[tuple[tuple[int, int], ...]] = (
    (0x80000000, 0x80800000),
    (0x84000000, 0x84800000),
)
DEFAULT_NOR_STUB_LOAD_ADDR: Final[int] = 0x51800000

# Verified MN103S Sleigh subroutine: polls SC0STR (0xD400200C) bit mask 0xA0, writes D3 to SC0TXB (0xD4002008).
_PUTC_D3_BYTES: Final[bytes] = bytes.fromhex(
    "fcac0c2000d4"
    "1c"
    "f8e0a0"
    "a000"
    "c9f4"
    "8c"
    "fc82082000d4"
    "df0000"
)
_HEADER_PREFIX_LEN: Final[int] = len(NOR_MAGIC_HEADER) + 12


@dataclass(frozen=True, slots=True)
class NorDumpResult:
    """Verified binary NOR dump recovered from a DIGADIAG UART stream."""

    start_addr: int
    total_len: int
    block_size: int
    data: bytes
    sha256: str
    block_checksums: tuple[int, ...]


def validate_nor_dump_range(start_addr: int, total_len: int, block_size: int) -> None:
    """Enforce strictly read-only uncached/cached NOR bounds and alignment."""
    if total_len <= 0:
        raise ValueError(f"total_len must be > 0 (got {total_len})")
    if block_size <= 0:
        raise ValueError(f"block_size must be > 0 (got {block_size})")
    if total_len % block_size != 0:
        raise ValueError(
            f"total_len ({total_len}) must be an exact multiple of block_size ({block_size})"
        )
    end_addr = start_addr + total_len
    if not any(lo <= start_addr and end_addr <= hi for lo, hi in NOR_ALLOWED_WINDOWS):
        raise ValueError(
            f"Read-only NOR guard rejected [0x{start_addr:08X}, 0x{end_addr:08X}); "
            "allowed windows: 0x80000000..0x80800000 or 0x84000000..0x84800000"
        )


def _emit_mov_imm32_a0(buf: bytearray, imm: int) -> int:
    off = len(buf)
    buf.extend(b"\xfc\xdc")
    buf.extend(struct.pack("<I", imm & 0xFFFFFFFF))
    return off + 2


def _emit_mov_imm32_a1(buf: bytearray, imm: int) -> int:
    off = len(buf)
    buf.extend(b"\xfc\xdd")
    buf.extend(struct.pack("<I", imm & 0xFFFFFFFF))
    return off + 2


def _emit_call_rel16(buf: bytearray) -> int:
    insn_off = len(buf)
    buf.extend(b"\xcd\x00\x00\x00\x00")
    return insn_off


def _emit_bne_back(buf: bytearray, target_off: int) -> None:
    insn_off = len(buf)
    disp = target_off - insn_off
    if disp < -128 or disp > 127:
        raise ValueError(f"MN103S BNE displacement out of s8 range: {disp}")
    buf.append(0xC9)
    buf.extend(struct.pack("<b", disp))


def _patch_call_rel16(buf: bytearray, call_insn_off: int, target_off: int) -> None:
    disp = target_off - call_insn_off
    if disp < -32768 or disp > 32767:
        raise ValueError(f"MN103S CALL displacement out of s16 range: {disp}")
    buf[call_insn_off + 1 : call_insn_off + 3] = struct.pack("<h", disp)


def build_readonly_nor_dumper_stub(
    start_addr: int,
    total_len: int,
    block_size: int = 4096,
    load_addr: int = DEFAULT_NOR_STUB_LOAD_ADDR,
) -> bytes:
    """Assemble position-coupled MN103S PIC/PIC-relocated read-only NOR UART dumper code."""
    validate_nor_dump_range(start_addr, total_len, block_size)
    if load_addr < 0 or load_addr > 0xFFF00000:
        raise ValueError(f"Invalid MN103S load_addr: 0x{load_addr:08X}")

    buf = bytearray()
    header_data = NOR_MAGIC_HEADER + struct.pack("<III", start_addr, total_len, block_size)

    # 1. Emit PANADMP1 + <start, total_len, block_size> header
    hdr_start_patch = _emit_mov_imm32_a0(buf, 0)
    hdr_end_patch = _emit_mov_imm32_a1(buf, 0)
    call_emit_hdr = _emit_call_rel16(buf)

    # 2. Setup NOR read cursor A0 and end pointer A1
    _emit_mov_imm32_a0(buf, start_addr)
    _emit_mov_imm32_a1(buf, start_addr + total_len)

    # 3. Outer block loop: reset D2 sum, load D1 = block_size
    block_loop_off = len(buf)
    buf.extend(b"\x8a\x00")  # MOV 0, D2
    buf.extend(b"\xfc\xcd")  # MOV imm32, D1
    buf.extend(struct.pack("<I", block_size))

    # 4. Inner read-only byte loop
    byte_loop_off = len(buf)
    buf.extend(b"\xf0\x40")  # MOVBU (A0), D0
    buf.append(0x83)  # MOV D0, D3
    buf.append(0xE2)  # ADD D0, D2
    call_putc_data = _emit_call_rel16(buf)
    buf.extend(b"\x20\x01")  # ADD 1, A0
    buf.extend(b"\x29\xff")  # ADD -1, D1
    buf.extend(b"\xa5\x00")  # CMP 0, D1
    _emit_bne_back(buf, byte_loop_off)

    # 5. Emit 32-bit little-endian per-block sum in D2
    buf.extend(b"\x85\x04")  # MOV 4, D1
    csum_loop_off = len(buf)
    buf.append(0x88)  # MOV D2, D0
    buf.append(0x83)  # MOV D0, D3
    call_putc_csum = _emit_call_rel16(buf)
    buf.extend(b"\xf8\xc6\x08")  # LSR 8, D2
    buf.extend(b"\x29\xff")  # ADD -1, D1
    buf.extend(b"\xa5\x00")  # CMP 0, D1
    _emit_bne_back(buf, csum_loop_off)

    # 6. Continue until A0 == A1
    buf.append(0xB4)  # CMP A1, A0
    _emit_bne_back(buf, block_loop_off)

    # 7. Emit PANADONE footer and park cleanly in BRA .
    ftr_start_patch = _emit_mov_imm32_a0(buf, 0)
    ftr_end_patch = _emit_mov_imm32_a1(buf, 0)
    call_emit_ftr = _emit_call_rel16(buf)
    buf.extend(b"\xca\x00")  # BRA .

    # 8. Shared span emitter subroutine [A0..A1)
    emit_span_off = len(buf)
    buf.extend(b"\xf0\x40")  # MOVBU (A0), D0
    buf.append(0x83)  # MOV D0, D3
    call_putc_span = _emit_call_rel16(buf)
    buf.extend(b"\x20\x01")  # ADD 1, A0
    buf.append(0xB4)  # CMP A1, A0
    _emit_bne_back(buf, emit_span_off)
    buf.extend(b"\xdf\x00\x00")  # RET [],0x0

    # 9. Shared TX subroutine + trailing constant tables
    putc_off = len(buf)
    buf.extend(_PUTC_D3_BYTES)

    hdr_off = len(buf)
    buf.extend(header_data)
    ftr_off = len(buf)
    buf.extend(NOR_MAGIC_FOOTER)
    end_off = len(buf)

    for call_off, tgt in (
        (call_emit_hdr, emit_span_off),
        (call_emit_ftr, emit_span_off),
        (call_putc_data, putc_off),
        (call_putc_csum, putc_off),
        (call_putc_span, putc_off),
    ):
        _patch_call_rel16(buf, call_off, tgt)

    for patch_off, rel_off in (
        (hdr_start_patch, hdr_off),
        (hdr_end_patch, ftr_off),
        (ftr_start_patch, ftr_off),
        (ftr_end_patch, end_off),
    ):
        buf[patch_off : patch_off + 4] = struct.pack("<I", (load_addr + rel_off) & 0xFFFFFFFF)

    return bytes(buf)


def decode_nor_dump_stream(stream: bytes) -> NorDumpResult:
    """Parse and checksum-verify a PANADMP1..PANADONE UART NOR capture."""
    hdr_idx = stream.find(NOR_MAGIC_HEADER)
    if hdr_idx < 0:
        raise ValueError("PANADMP1 header not found in UART stream")
    if len(stream) - hdr_idx < _HEADER_PREFIX_LEN:
        raise ValueError("Truncated PANADMP1 header")

    pos = hdr_idx + len(NOR_MAGIC_HEADER)
    start_addr, total_len, block_size = struct.unpack_from("<III", stream, pos)
    validate_nor_dump_range(start_addr, total_len, block_size)
    pos += 12

    num_blocks = total_len // block_size
    expected_payload_bytes = num_blocks * (block_size + 4) + len(NOR_MAGIC_FOOTER)
    if len(stream) - pos < expected_payload_bytes:
        raise ValueError(
            f"Truncated NOR dump stream: need {expected_payload_bytes} bytes after header, "
            f"have {len(stream) - pos}"
        )

    out = bytearray()
    checksums: list[int] = []
    for block_idx in range(num_blocks):
        block = stream[pos : pos + block_size]
        pos += block_size
        (expected_sum,) = struct.unpack_from("<I", stream, pos)
        pos += 4
        actual_sum = sum(block) & 0xFFFFFFFF
        if actual_sum != expected_sum:
            raise ValueError(
                f"Block {block_idx} checksum mismatch: expected 0x{expected_sum:08X}, "
                f"got 0x{actual_sum:08X}"
            )
        out.extend(block)
        checksums.append(expected_sum)

    footer = stream[pos : pos + len(NOR_MAGIC_FOOTER)]
    if footer != NOR_MAGIC_FOOTER:
        raise ValueError(f"Expected {NOR_MAGIC_FOOTER!r} footer, got {footer!r}")

    assembled = bytes(out)
    return NorDumpResult(
        start_addr=start_addr,
        total_len=total_len,
        block_size=block_size,
        data=assembled,
        sha256=hashlib.sha256(assembled).hexdigest(),
        block_checksums=tuple(checksums),
    )
