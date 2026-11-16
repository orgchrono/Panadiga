"""Thin typed Python facade delegating `BOOT.BIN` / `DRV_decomp.bin` static tables to `panadiga re --json`."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Final

from scripts.panadiga_cli_bridge import (
    PanadigaCliError,
    as_dict,
    as_list,
    req_bool,
    req_int,
    req_str,
    run_panadiga_json,
)

DRV_UNLOCK_COMMAND: Final[str] = "PASS 0x4d415453 0x48495441"


@dataclass(frozen=True)
class ExtrheadMatch:
    """Location of an `EXTRHEAD` compressed section marker."""

    offset: int
    va: int
    tail_preview_hex: str


@dataclass(frozen=True)
class DrvCliEntry:
    """One `<8sIII>` entry from the `DRV_decomp.bin` factory serial CLI table."""

    index: int
    name: str
    handler_va: int
    arg1: int
    flags: int


@dataclass(frozen=True)
class AtapiDispatchEntry:
    """One ATAPI CDB[0] entry from the 256-entry dispatch table."""

    cdb_opcode: int
    handler_va: int
    is_active: bool


@dataclass(frozen=True)
class DrvMechaErrorEntry:
    """One 52-byte (`0x34`) Shift-JIS mechanical error table entry."""

    va: int
    code: int
    message_jp: str


@dataclass(frozen=True)
class SenseEntry:
    """One 3-byte `[SenseKey, ASC, ASCQ]` record in the DRV Sense LUT."""

    index: int
    va: int
    sense_key: int
    asc: int
    ascq: int


def decompress_drv_payload(drv_raw: bytes, header_size: int = 0x40) -> bytes:
    """Decompress Panasonic `DRV.BIN` via `panadiga re pana-frm --kind decompress-drv`."""
    try:
        rep = as_dict(
            run_panadiga_json(
                ["re", "pana-frm", "--kind", "decompress-drv", "--input", "-", "--header-size", str(header_size)],
                stdin_bytes=drv_raw,
            )
        )
    except PanadigaCliError as exc:
        raise ValueError(str(exc)) from exc
    return bytes.fromhex(req_str(rep, "data_hex"))


def find_extrhead_containers(buf: bytes, image_base: int = 0) -> list[ExtrheadMatch]:
    """Locate `'EXTRHEAD'` markers via `panadiga re pana-frm --kind extrhead`."""
    rows = as_list(
        run_panadiga_json(
            ["re", "pana-frm", "--kind", "extrhead", "--input", "-", "--base", f"0x{image_base:08X}"],
            stdin_bytes=buf,
        )
    )
    return [
        ExtrheadMatch(
            offset=req_int(d, "offset"),
            va=req_int(d, "va"),
            tail_preview_hex=req_str(d, "tail_preview_hex"),
        )
        for d in (as_dict(item) for item in rows)
    ]


def classify_drv_cli_command(name: str, flags: int, pass_unlocked: bool = False) -> str:
    """Classify one DRV CLI entry via `panadiga re pana-frm --kind classify-cli`."""
    args = [
        "re",
        "pana-frm",
        "--kind",
        "classify-cli",
        "--cmd-name",
        name,
        "--flags",
        str(flags),
    ]
    if pass_unlocked:
        args.append("--pass-unlocked")
    rep = as_dict(run_panadiga_json(args))
    return req_str(rep, "classification")


def parse_drv_cli_table(
    drv_buf: bytes,
    table_offset: int,
    image_base: int = 0x40000000,
    count: int = 113,
) -> list[DrvCliEntry]:
    """Decode `<8sIII>` factory serial monitor CLI table via `panadiga re drv-cli`."""
    rows = as_list(
        run_panadiga_json(
            [
                "re",
                "drv-cli",
                "--input",
                "-",
                "--offset",
                f"0x{table_offset:X}",
                "--base",
                f"0x{image_base:08X}",
                "--max-entries",
                str(count),
            ],
            stdin_bytes=drv_buf,
        )
    )
    return [
        DrvCliEntry(
            index=req_int(d, "index"),
            name=req_str(d, "name"),
            handler_va=req_int(d, "handler_va"),
            arg1=req_int(d, "arg_mask"),
            flags=req_int(d, "flags"),
        )
        for d in (as_dict(item) for item in rows)
    ]


def parse_atapi_cdb_dispatch(
    drv_buf: bytes,
    table_offset: int,
    count: int = 256,
    default_handler: int = 0x40014559,
) -> list[AtapiDispatchEntry]:
    """Decode ATAPI CDB[0] dispatch pointer table via `panadiga re pana-frm --kind atapi-dispatch`."""
    rows = as_list(
        run_panadiga_json(
            [
                "re",
                "pana-frm",
                "--kind",
                "atapi-dispatch",
                "--input",
                "-",
                "--offset",
                f"0x{table_offset:X}",
                "--count",
                str(count),
                "--default-handler",
                f"0x{default_handler:08X}",
            ],
            stdin_bytes=drv_buf,
        )
    )
    return [
        AtapiDispatchEntry(
            cdb_opcode=req_int(d, "cdb0"),
            handler_va=req_int(d, "handler_va"),
            is_active=req_bool(d, "is_active"),
        )
        for d in (as_dict(item) for item in rows)
    ]


def parse_drv_error_table(
    drv_buf: bytes,
    start_offset: int,
    end_offset: int,
    stride: int = 0x34,
    image_base: int = 0x40000000,
) -> list[DrvMechaErrorEntry]:
    """Decode 52-byte (`0x34`) CP932 error dictionary slice via `panadiga re drv-errors`."""
    del stride
    rows = as_list(
        run_panadiga_json(
            [
                "re",
                "drv-errors",
                "--input",
                "-",
                "--start",
                f"0x{start_offset:X}",
                "--end",
                f"0x{end_offset:X}",
            ],
            stdin_bytes=drv_buf,
        )
    )
    return [
        DrvMechaErrorEntry(
            va=image_base + req_int(d, "offset"),
            code=req_int(d, "code"),
            message_jp=bytes.fromhex(req_str(d, "raw_hex")).decode("cp932", errors="replace"),
        )
        for d in (as_dict(item) for item in rows)
    ]


def parse_sense_lut(
    drv_buf: bytes,
    table_offset: int,
    image_base: int = 0x40000000,
    count: int = 16,
) -> list[SenseEntry]:
    """Decode 3-byte `[SenseKey, ASC, ASCQ]` records via `panadiga re pana-frm --kind sense-lut`."""
    rows = as_list(
        run_panadiga_json(
            [
                "re",
                "pana-frm",
                "--kind",
                "sense-lut",
                "--input",
                "-",
                "--offset",
                f"0x{table_offset:X}",
                "--base",
                f"0x{image_base:08X}",
                "--count",
                str(count),
            ],
            stdin_bytes=drv_buf,
        )
    )
    return [
        SenseEntry(
            index=req_int(d, "index"),
            va=req_int(d, "va"),
            sense_key=req_int(d, "sense_key"),
            asc=req_int(d, "asc"),
            ascq=req_int(d, "ascq"),
        )
        for d in (as_dict(item) for item in rows)
    ]
