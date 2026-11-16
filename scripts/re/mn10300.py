"""Thin typed Python facade delegating Matsushita MN103S / AM34 xrefs to `panadiga re mn103 --json`."""

from __future__ import annotations

from dataclasses import dataclass

from scripts.panadiga_cli_bridge import (
    as_dict,
    as_list,
    opt_str,
    req_bool,
    req_int,
    req_str,
    run_panadiga_json,
)


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


def _decode_insn_refs(raw_list: object) -> list[InstructionRef]:
    rows = as_list(raw_list if isinstance(raw_list, list) else [])
    out: list[InstructionRef] = []
    for item in rows:
        d = as_dict(item)
        out.append(
            InstructionRef(
                va=req_int(d, "va"),
                opcode=req_int(d, "opcode"),
                mnemonic=req_str(d, "mnemonic"),
                displacement=req_int(d, "displacement"),
                target_va=req_int(d, "target_va"),
            )
        )
    return out


def _window_args(
    kind: str,
    image_base: int,
    va_start: int | None,
    va_end: int | None,
) -> list[str]:
    args = ["re", "mn103", "--kind", kind, "--input", "-", "--base", f"0x{image_base:08X}"]
    if va_start is not None:
        args.extend(["--va-start", f"0x{va_start:08X}"])
    if va_end is not None:
        args.extend(["--va-end", f"0x{va_end:08X}"])
    return args


def decode_regmask_names(regs_mask: int) -> tuple[str, ...]:
    """Decode an 8-bit MN103S `CALL`/`MOVM`/`RET` register mask via native Rust SSOT."""
    raw = run_panadiga_json(["re", "mn103", "--kind", "regmask", "--regmask", str(regs_mask & 0xFF)])
    return tuple(str(x) for x in as_list(raw) if isinstance(x, str))


def decode_branch_call_refs(
    buf: bytes,
    image_base: int,
    va_start: int | None = None,
    va_end: int | None = None,
    max_abs_disp: int = 0x200000,
) -> list[InstructionRef]:
    """Scan a binary window via `panadiga re mn103 --kind branches --json`."""
    args = _window_args("branches", image_base, va_start, va_end)
    args.extend(["--max-abs-disp", f"0x{max_abs_disp:X}"])
    return _decode_insn_refs(run_panadiga_json(args, stdin_bytes=buf))


def find_callers_to(
    buf: bytes,
    image_base: int,
    targets: set[int] | list[int] | tuple[int, ...],
) -> list[InstructionRef]:
    """Find every MN103S branch or call targeting `targets` via `panadiga re mn103 --kind callers`."""
    tgt_csv = ",".join(f"0x{t:08X}" for t in targets)
    args = [
        "re",
        "mn103",
        "--kind",
        "callers",
        "--input",
        "-",
        "--base",
        f"0x{image_base:08X}",
        "--targets",
        tgt_csv,
    ]
    return _decode_insn_refs(run_panadiga_json(args, stdin_bytes=buf))


def find_fc_immediate_loads(
    buf: bytes,
    image_base: int,
    va_start: int,
    va_end: int,
) -> list[ImmediateLoadRef]:
    """Extract 6-byte `0xFC <sub_op> <u32>` immediate loads via `panadiga re mn103 --kind fc-loads`."""
    args = _window_args("fc-loads", image_base, va_start, va_end)
    rows = as_list(run_panadiga_json(args, stdin_bytes=buf))
    return [
        ImmediateLoadRef(
            va=req_int(d, "va"),
            sub_opcode=req_int(d, "sub_opcode"),
            value=req_int(d, "value"),
            c_string=opt_str(d, "c_string"),
        )
        for d in (as_dict(item) for item in rows)
    ]


def find_pointer_xrefs(buf: bytes, image_base: int, target_va: int) -> list[int]:
    """Find every LE32 pointer reference to `target_va` via `panadiga re mn103 --kind pointer-xrefs`."""
    args = [
        "re",
        "mn103",
        "--kind",
        "pointer-xrefs",
        "--input",
        "-",
        "--base",
        f"0x{image_base:08X}",
        "--target",
        f"0x{target_va:08X}",
    ]
    return [int(x) for x in as_list(run_panadiga_json(args, stdin_bytes=buf)) if isinstance(x, int)]


def decode_bcc_refs(
    buf: bytes,
    image_base: int,
    va_start: int | None = None,
    va_end: int | None = None,
) -> list[InstructionRef]:
    """Decode 2-byte conditional branches `Bcc` via `panadiga re mn103 --kind bcc`."""
    args = _window_args("bcc", image_base, va_start, va_end)
    return _decode_insn_refs(run_panadiga_json(args, stdin_bytes=buf))


def decode_call_frames(
    buf: bytes,
    image_base: int,
    va_start: int | None = None,
    va_end: int | None = None,
    max_abs_disp: int = 0x200000,
) -> list[CallFrameRef]:
    """Decode full 5/7-byte `call16`/`call32` frame descriptors via `panadiga re mn103 --kind call-frames`."""
    args = _window_args("call-frames", image_base, va_start, va_end)
    args.extend(["--max-abs-disp", f"0x{max_abs_disp:X}"])
    rows = as_list(run_panadiga_json(args, stdin_bytes=buf))
    out: list[CallFrameRef] = []
    for item in rows:
        d = as_dict(item)
        groups = tuple(str(g) for g in as_list(d.get("saved_groups")) if isinstance(g, str))
        out.append(
            CallFrameRef(
                va=req_int(d, "va"),
                target_va=req_int(d, "target_va"),
                is_call32=req_bool(d, "is_call32"),
                regs_mask=req_int(d, "regs_mask"),
                frame_bytes=req_int(d, "frame_bytes"),
                saved_groups=groups,
            )
        )
    return out


def decode_setlb_loops(
    buf: bytes,
    image_base: int,
    va_start: int | None = None,
    va_end: int | None = None,
    max_body_bytes: int = 64,
) -> list[HardwareLoopRef]:
    """Pair `0xDB` (`SETLB`) markers with trailing `Lcc`/`LRA` via `panadiga re mn103 --kind setlb`."""
    args = _window_args("setlb", image_base, va_start, va_end)
    args.extend(["--max-body-bytes", str(max_body_bytes)])
    rows = as_list(run_panadiga_json(args, stdin_bytes=buf))
    return [
        HardwareLoopRef(
            setlb_va=req_int(d, "setlb_va"),
            loop_body_va=req_int(d, "loop_body_va"),
            lcc_va=req_int(d, "lcc_va"),
            opcode=req_int(d, "opcode"),
            mnemonic=req_str(d, "mnemonic"),
            body_len=req_int(d, "body_len"),
        )
        for d in (as_dict(item) for item in rows)
    ]
