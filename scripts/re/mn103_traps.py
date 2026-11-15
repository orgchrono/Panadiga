"""Static Matsushita MN103S / AM34 Trap 1-7 analysis and resolution helpers.

Complements `scripts.re.mn10300` without duplicating decoding primitives:
- Trap 1: Pointer table & orphan dispatch table scanning (`scan_code_pointer_tables`)
- Trap 4: Mid-instruction branch/call overlap detection (`detect_mid_instruction_overlaps`)
- Trap 5: Spansion CS1 CFI/JEDEC volatile probe detection (`is_cfi_command_address`)
- Trap 6: SETLB..Lcc hardware loop & negative frame verification
- Trap 7: DRV revision fingerprint & mid-instruction constant false-hit filtering
"""

from __future__ import annotations

from dataclasses import dataclass
import struct
from typing import Final, Sequence

from scripts.re.mn10300 import decode_branch_call_refs, decode_setlb_loops

KNOWN_CFI_LOW_OFFSETS: Final[frozenset[int]] = frozenset({0x000, 0x554, 0x555, 0xAAA, 0xAAB})
KNOWN_DRV_STUB_VAS: Final[frozenset[int]] = frozenset({0x40014559})


@dataclass(frozen=True)
class InstructionSpan:
    """Address span [start_va, end_va) for a single MN103S machine instruction."""

    start_va: int
    end_va: int
    mnemonic: str


@dataclass(frozen=True)
class OverlapCollision:
    """Trap 4 collision where a branch/call lands strictly inside an instruction."""

    source_va: int
    target_va: int
    containing_start_va: int
    containing_end_va: int
    mnemonic: str


@dataclass(frozen=True)
class DrvRevisionFingerprint:
    """Trap 7 fingerprint separating DRV_decomp vs EH55_DRV_S211 vs BOOT/PROG061."""

    stage: str
    revision: str
    pass_handler_va: int | None
    cli_table_va: int | None
    atapi_write_buffer_va: int | None
    atapi_vendor_ed_va: int | None


def estimate_mn103_insn_len(opcode: int) -> int:
    """Estimate conservative MN103S instruction length from leading opcode byte."""
    b = opcode & 0xFF
    if b == 0xFC:
        return 6
    if b == 0xDD:
        return 7
    if b in (0xDC, 0xCD):
        return 5
    if b in (0xF8, 0xFA, 0xFE, 0xDE, 0xDF, 0xCC):
        return 3
    if b in (0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB, 0xCE):
        return 2
    return 1


def build_instruction_spans(data: bytes, base_va: int) -> tuple[InstructionSpan, ...]:
    """Build instruction spans across decoded branch/call/SETLB opcodes."""
    spans: dict[int, InstructionSpan] = {}
    for br in decode_branch_call_refs(data, base_va):
        insn_len = estimate_mn103_insn_len(br.opcode)
        spans[br.va] = InstructionSpan(br.va, br.va + insn_len, br.mnemonic)
    for lp in decode_setlb_loops(data, base_va):
        spans[lp.setlb_va] = InstructionSpan(lp.setlb_va, lp.setlb_va + 1, "setlb")
        spans[lp.lcc_va] = InstructionSpan(lp.lcc_va, lp.lcc_va + 1, lp.mnemonic)
    return tuple(spans[k] for k in sorted(spans))


def detect_mid_instruction_overlaps(
    data: bytes,
    base_va: int,
    extra_spans: Sequence[InstructionSpan] = (),
) -> tuple[OverlapCollision, ...]:
    """Trap 4: flag branch/call targets falling inside [start_va + 1, end_va)."""
    merged = list(build_instruction_spans(data, base_va))
    merged.extend(extra_spans)
    refs = decode_branch_call_refs(data, base_va)
    hits: list[OverlapCollision] = []
    for ref in refs:
        tgt = ref.target_va
        for sp in merged:
            if sp.start_va < tgt < sp.end_va:
                hits.append(
                    OverlapCollision(
                        source_va=ref.va,
                        target_va=tgt,
                        containing_start_va=sp.start_va,
                        containing_end_va=sp.end_va,
                        mnemonic=ref.mnemonic,
                    )
                )
                break
    return tuple(hits)


def filter_mid_instruction_constant_hits(
    hit_vas: Sequence[int],
    insn_spans: Sequence[InstructionSpan],
) -> tuple[tuple[int, ...], tuple[int, ...]]:
    """Trap 7: separate genuine pointer/constant addresses from mid-instruction false hits."""
    genuine: list[int] = []
    false_hits: list[int] = []
    for va in hit_vas:
        if any(sp.start_va < va < sp.end_va for sp in insn_spans):
            false_hits.append(va)
        else:
            genuine.append(va)
    return tuple(genuine), tuple(false_hits)


def is_cfi_command_address(va: int) -> bool:
    """Trap 5: return True when virtual address hits Spansion CS1 CFI/JEDEC aperture."""
    u = va & 0xFFFFFFFF
    low = u & 0x0FFF
    return (0x84000000 <= u < 0x847F0000 or 0x80000000 <= u < 0x807F0000) and (
        low in KNOWN_CFI_LOW_OFFSETS
    )


def scan_code_pointer_tables(
    data: bytes,
    base_va: int,
    code_lo: int,
    code_hi: int,
    *,
    min_run: int = 4,
    stub_vas: frozenset[int] = KNOWN_DRV_STUB_VAS,
) -> tuple[tuple[int, int], ...]:
    """Trap 1: extract contiguous aligned LE32 code pointer runs pointing into [code_lo, code_hi)."""
    out: list[tuple[int, int]] = []
    run: list[tuple[int, int]] = []
    limit = (len(data) // 4) * 4
    for off in range(0, limit, 4):
        (val,) = struct.unpack_from("<I", data, off)
        if code_lo <= val < code_hi:
            run.append((base_va + off, val))
        else:
            if len(run) >= min_run:
                out.extend(pair for pair in run if pair[1] not in stub_vas)
            run = []
    if len(run) >= min_run:
        out.extend(pair for pair in run if pair[1] not in stub_vas)
    return tuple(out)


def identify_drv_revision(data: bytes) -> DrvRevisionFingerprint:
    """Trap 7: distinguish DRV_decomp vs EH55_DRV_S211 vs BOOT/PROG061 from exact signatures."""
    has_slogon = b"SLOGON\x00\x00" in data or b"MMODE\x00\x00\x00" in data
    has_s211 = b"R6-MV4" in data or b"S211" in data
    if has_slogon and len(data) < 0x180000:
        return DrvRevisionFingerprint(
            stage="DRV",
            revision="DRV_decomp",
            pass_handler_va=0x4013420A,
            cli_table_va=0x400DF5C0,
            atapi_write_buffer_va=0x4002168D,
            atapi_vendor_ed_va=0x400209F3,
        )
    if has_s211 or len(data) == 0x200000:
        return DrvRevisionFingerprint(
            stage="DRV",
            revision="EH55_DRV_S211",
            pass_handler_va=0x40130EFA,
            cli_table_va=None,
            atapi_write_buffer_va=None,
            atapi_vendor_ed_va=None,
        )
    if b"Loader ver0.61" in data:
        return DrvRevisionFingerprint(
            stage="BOOT_OR_PROG061",
            revision="ver0.61",
            pass_handler_va=None,
            cli_table_va=None,
            atapi_write_buffer_va=None,
            atapi_vendor_ed_va=None,
        )
    return DrvRevisionFingerprint(
        stage="UNKNOWN",
        revision="generic",
        pass_handler_va=None,
        cli_table_va=None,
        atapi_write_buffer_va=None,
        atapi_vendor_ed_va=None,
    )
