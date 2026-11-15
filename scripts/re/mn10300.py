"""Matsushita MN103S / AM34 instruction cross-reference helpers.

Encodes the empirically verified MN10300 / MN103S PC-relative encoding rule:
relative displacement is computed from the address of the current instruction
(`target_va = instruction_va + signed_disp`), not the following instruction.
"""

from __future__ import annotations

from dataclasses import dataclass
import struct
from typing import Final

# Opcode -> (mnemonic, struct_fmt, instruction_len)
_REL_BRANCH_TABLE: Final[dict[int, tuple[str, str, int]]] = {
    0xCA: ("bra8", "<b", 2),
    0xCC: ("jmp16", "<h", 3),
    0xCD: ("call16", "<h", 3),
    0xDC: ("jmp32", "<i", 5),
    0xDD: ("call32", "<i", 5),
}


@dataclass(frozen=True)
class InstructionRef:
    """Decoded MN103S relative call or jump instruction."""

    va: int
    opcode: int
    mnemonic: str
    displacement: int
    target_va: int


@dataclass(frozen=True)
class ImmediateLoadRef:
    """Decoded 6-byte `0xFC <sub> <imm32>` extended immediate load."""

    va: int
    sub_opcode: int
    value: int
    c_string: str | None


@dataclass(frozen=True)
class CallFrameRef:
    """Decoded MN103S caller-encoded callee stack frame (`0xCD` / `0xDD`)."""

    va: int
    target_va: int
    is_call32: bool
    regs_mask: int
    frame_bytes: int
    saved_groups: tuple[str, ...]


@dataclass(frozen=True)
class HardwareLoopRef:
    """Decoded MN103S `SETLB` .. `Lcc`/`LRA` hardware loop pair."""

    setlb_va: int
    loop_body_va: int
    lcc_va: int
    opcode: int
    mnemonic: str
    body_len: int


_REGMASK_ORDER: Final[tuple[str, ...]] = (
    "d2",
    "d3",
    "a2",
    "a3",
    "other",
    "exreg0",
    "exreg1",
    "exother",
)

_CC_SUFFIXES: Final[tuple[str, ...]] = ("lt", "gt", "ge", "le", "cs", "hi", "cc", "ls", "eq", "ne")


def decode_regmask_names(regs_mask: int) -> tuple[str, ...]:
    """Decode an 8-bit MN103S CALL/MOVM/RET register mask into group names."""
    mask = regs_mask & 0xFF
    return tuple(name for idx, name in enumerate(_REGMASK_ORDER) if (mask & (0x80 >> idx)) != 0)


def _va_bounds(buf_len: int, image_base: int, va_start: int | None, va_end: int | None) -> tuple[int, int]:
    lo = 0 if va_start is None else max(0, va_start - image_base)
    hi = buf_len if va_end is None else min(buf_len, va_end - image_base)
    return lo, hi


def _checked_rel_target(
    buf_len: int,
    image_base: int,
    off: int,
    insn_len: int,
    disp: int,
    max_abs_disp: int | None = None,
) -> tuple[int, int] | None:
    if off + insn_len > buf_len or disp == 0:
        return None
    if max_abs_disp is not None and abs(disp) > max_abs_disp:
        return None
    va = image_base + off
    target = va + disp
    if image_base <= target < image_base + buf_len:
        return va, target
    return None


def _extract_ascii_or_cp932(buf: bytes, offset: int, max_len: int = 80) -> str | None:
    """Decode a printable CP932 C-string at `offset`, or return None."""
    if offset < 0 or offset >= len(buf):
        return None
    end = buf.find(b"\x00", offset)
    if end == -1 or end - offset > max_len:
        end = min(len(buf), offset + max_len)
    raw = buf[offset:end]
    if not raw:
        return None
    try:
        decoded = raw.decode("cp932")
    except UnicodeDecodeError:
        return None
    if all((0x20 <= ord(ch) < 0xFF00) or ch in "\r\n\t" for ch in decoded):
        return decoded
    return None


def decode_branch_call_refs(
    buf: bytes,
    image_base: int,
    va_start: int | None = None,
    va_end: int | None = None,
    max_abs_disp: int = 0x200000,
) -> list[InstructionRef]:
    """Scan a binary window for MN103S relative branches/calls targeting `[image_base, image_end)`."""
    lo, hi = _va_bounds(len(buf), image_base, va_start, va_end)
    refs: list[InstructionRef] = []

    for off in range(lo, hi):
        op = buf[off]
        spec = _REL_BRANCH_TABLE.get(op)
        if spec is None or off + spec[2] > len(buf):
            continue
        mnemonic, fmt, insn_len = spec
        disp = int(struct.unpack_from(fmt, buf, off + 1)[0])
        resolved = _checked_rel_target(len(buf), image_base, off, insn_len, disp, max_abs_disp)
        if resolved is not None:
            refs.append(InstructionRef(resolved[0], op, mnemonic, disp, resolved[1]))
    return refs


def find_callers_to(
    buf: bytes,
    image_base: int,
    targets: set[int] | list[int] | tuple[int, ...],
) -> list[InstructionRef]:
    """Return every MN103S branch or call referencing any virtual address in `targets`."""
    target_set = set(targets)
    return [ref for ref in decode_branch_call_refs(buf, image_base) if ref.target_va in target_set]


def find_fc_immediate_loads(
    buf: bytes,
    image_base: int,
    va_start: int,
    va_end: int,
) -> list[ImmediateLoadRef]:
    """Extract 6-byte `0xFC <sub_op> <u32>` immediate/pointer loads in a VA range."""
    lo = max(0, va_start - image_base)
    hi = min(len(buf) - 6, va_end - image_base)
    upper_va = image_base + len(buf)
    results: list[ImmediateLoadRef] = []
    for off in range(lo, hi + 1):
        if buf[off] != 0xFC:
            continue
        sub = buf[off + 1]
        val = int(struct.unpack_from("<I", buf, off + 2)[0])
        cstr = _extract_ascii_or_cp932(buf, val - image_base) if image_base <= val < upper_va else None
        results.append(ImmediateLoadRef(image_base + off, sub, val, cstr))
    return results


def find_pointer_xrefs(buf: bytes, image_base: int, target_va: int) -> list[int]:
    """Find every 32-bit little-endian literal reference to `target_va` inside `buf`."""
    needle = struct.pack("<I", target_va)
    hits: list[int] = []
    pos = 0
    while True:
        idx = buf.find(needle, pos)
        if idx == -1:
            break
        hits.append(image_base + idx)
        pos = idx + 1
    return hits


def decode_bcc_refs(
    buf: bytes,
    image_base: int,
    va_start: int | None = None,
    va_end: int | None = None,
) -> list[InstructionRef]:
    """Decode 2-byte `Bcc` (`0xC0..=0xC9`) relative conditional branches."""
    lo, hi = _va_bounds(len(buf), image_base, va_start, va_end)
    refs: list[InstructionRef] = []
    for off in range(lo, hi):
        op = buf[off]
        if not (0xC0 <= op <= 0xC9) or off + 2 > len(buf):
            continue
        disp = int(struct.unpack_from("<b", buf, off + 1)[0])
        resolved = _checked_rel_target(len(buf), image_base, off, 2, disp)
        if resolved is not None:
            refs.append(InstructionRef(resolved[0], op, f"b{_CC_SUFFIXES[op & 0x0F]}", disp, resolved[1]))
    return refs


def decode_call_frames(
    buf: bytes,
    image_base: int,
    va_start: int | None = None,
    va_end: int | None = None,
    max_abs_disp: int = 0x200000,
) -> list[CallFrameRef]:
    """Decode full 5-byte `0xCD` (`call16`) and 7-byte `0xDD` (`call32`) frame descriptors."""
    lo, hi = _va_bounds(len(buf), image_base, va_start, va_end)
    frames: list[CallFrameRef] = []
    for off in range(lo, hi):
        op = buf[off]
        if op not in (0xCD, 0xDD):
            continue
        is_call32 = op == 0xDD
        insn_len = 7 if is_call32 else 5
        if off + insn_len > len(buf):
            continue
        disp = int(struct.unpack_from("<i" if is_call32 else "<h", buf, off + 1)[0])
        resolved = _checked_rel_target(len(buf), image_base, off, insn_len, disp, max_abs_disp)
        if resolved is None:
            continue
        regs = buf[off + insn_len - 2]
        frame = buf[off + insn_len - 1]
        frames.append(CallFrameRef(resolved[0], resolved[1], is_call32, regs, frame, decode_regmask_names(regs)))
    return frames


def decode_setlb_loops(
    buf: bytes,
    image_base: int,
    va_start: int | None = None,
    va_end: int | None = None,
    max_body_bytes: int = 64,
) -> list[HardwareLoopRef]:
    """Pair `0xDB` (`SETLB`) markers with trailing `Lcc` (`0xD0..=0xD9`) or `LRA` (`0xDA`)."""
    lo = 0 if va_start is None else max(0, va_start - image_base)
    hi = len(buf) if va_end is None else min(len(buf), va_end - image_base)
    loops: list[HardwareLoopRef] = []
    for off in range(lo, hi):
        if buf[off] != 0xDB or off + 5 > len(buf):
            continue
        body_start = off + 1
        scan_end = min(len(buf), off + 1 + max_body_bytes)
        for cur in range(min(scan_end, body_start + 4), scan_end):
            term = buf[cur]
            if term == 0xDA:
                mnemonic = "lra"
            elif 0xD0 <= term <= 0xD9:
                mnemonic = f"l{_CC_SUFFIXES[term & 0x0F]}"
            else:
                continue
            loops.append(
                HardwareLoopRef(
                    setlb_va=image_base + off,
                    loop_body_va=image_base + body_start,
                    lcc_va=image_base + cur,
                    opcode=term,
                    mnemonic=mnemonic,
                    body_len=cur - body_start,
                )
            )
            break
    return loops
