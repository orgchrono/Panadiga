"""Thin typed Python facade delegating MN103S Trap 1-7 analysis to `panadiga re traps --json`."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import json
from typing import Final, Sequence

from scripts.panadiga_cli_bridge import (
    as_dict,
    as_list,
    opt_int,
    req_bool,
    req_int,
    req_str,
    run_panadiga_json,
)

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
    """Estimate conservative MN103S instruction length via `panadiga re traps --kind insn-len`."""
    res = run_panadiga_json(["re", "traps", "--kind", "insn-len", "--opcode", str(opcode & 0xFF)])
    if isinstance(res, bool) or not isinstance(res, int):
        raise RuntimeError(f"Unexpected insn-len reply: {res!r}")
    return res


def build_instruction_spans(data: bytes, base_va: int) -> tuple[InstructionSpan, ...]:
    """Build instruction spans via `panadiga re traps --kind spans`."""
    rows = as_list(
        run_panadiga_json(
            ["re", "traps", "--kind", "spans", "--input", "-", "--base", f"0x{base_va:08X}"],
            stdin_bytes=data,
        )
    )
    return tuple(
        InstructionSpan(
            start_va=req_int(d, "start_va"),
            end_va=req_int(d, "end_va"),
            mnemonic=req_str(d, "mnemonic"),
        )
        for d in (as_dict(item) for item in rows)
    )


def detect_mid_instruction_overlaps(
    data: bytes,
    base_va: int,
    extra_spans: Sequence[InstructionSpan] = (),
) -> tuple[OverlapCollision, ...]:
    """Trap 4: flag mid-instruction branch/call overlaps via `panadiga re traps --kind overlaps`."""
    args = ["re", "traps", "--kind", "overlaps", "--input", "-", "--base", f"0x{base_va:08X}"]
    if extra_spans:
        args.extend(["--extra-spans", json.dumps([asdict(sp) for sp in extra_spans])])
    rows = as_list(run_panadiga_json(args, stdin_bytes=data))
    return tuple(
        OverlapCollision(
            source_va=req_int(d, "source_va"),
            target_va=req_int(d, "target_va"),
            containing_start_va=req_int(d, "containing_start_va"),
            containing_end_va=req_int(d, "containing_end_va"),
            mnemonic=req_str(d, "mnemonic"),
        )
        for d in (as_dict(item) for item in rows)
    )


def filter_mid_instruction_constant_hits(
    hit_vas: Sequence[int],
    insn_spans: Sequence[InstructionSpan],
) -> tuple[tuple[int, ...], tuple[int, ...]]:
    """Trap 7: filter mid-instruction false constant hits via `panadiga re traps --kind filter-hits`."""
    args = [
        "re",
        "traps",
        "--kind",
        "filter-hits",
        "--hits",
        ",".join(f"0x{va:08X}" for va in hit_vas),
        "--extra-spans",
        json.dumps([asdict(sp) for sp in insn_spans]),
    ]
    rep = as_dict(run_panadiga_json(args))
    genuine = tuple(int(v) for v in as_list(rep.get("genuine")) if isinstance(v, int))
    false_hits = tuple(int(v) for v in as_list(rep.get("false_hits")) if isinstance(v, int))
    return genuine, false_hits


def is_cfi_command_address(va: int) -> bool:
    """Trap 5: validate Spansion CS1 CFI/JEDEC aperture via `panadiga re traps --kind cfi`."""
    res = run_panadiga_json(["re", "traps", "--kind", "cfi", "--va", f"0x{va & 0xFFFFFFFF:08X}"])
    return req_bool({"ok": res}, "ok")


def scan_code_pointer_tables(
    data: bytes,
    base_va: int,
    code_lo: int,
    code_hi: int,
    *,
    min_run: int = 4,
    stub_vas: frozenset[int] = KNOWN_DRV_STUB_VAS,
) -> tuple[tuple[int, int], ...]:
    """Trap 1: extract aligned LE32 code pointer runs via `panadiga re traps --kind ptr-tables`."""
    args = [
        "re",
        "traps",
        "--kind",
        "ptr-tables",
        "--input",
        "-",
        "--base",
        f"0x{base_va:08X}",
        "--code-lo",
        f"0x{code_lo:08X}",
        "--code-hi",
        f"0x{code_hi:08X}",
        "--min-run",
        str(min_run),
        "--stub-vas",
        ",".join(f"0x{s:08X}" for s in sorted(stub_vas)),
    ]
    rows = as_list(run_panadiga_json(args, stdin_bytes=data))
    return tuple((req_int(d, "va"), req_int(d, "target_va")) for d in (as_dict(item) for item in rows))


def identify_drv_revision(data: bytes) -> DrvRevisionFingerprint:
    """Trap 7: identify DRV_decomp vs EH55_DRV_S211 vs BOOT/PROG061 via `panadiga re traps --kind revision`."""
    rep = as_dict(run_panadiga_json(["re", "traps", "--kind", "revision", "--input", "-"], stdin_bytes=data))
    return DrvRevisionFingerprint(
        stage=req_str(rep, "stage"),
        revision=req_str(rep, "revision"),
        pass_handler_va=opt_int(rep, "pass_handler_va"),
        cli_table_va=opt_int(rep, "cli_table_va"),
        atapi_write_buffer_va=opt_int(rep, "atapi_write_buffer_va"),
        atapi_vendor_ed_va=opt_int(rep, "atapi_vendor_ed_va"),
    )
