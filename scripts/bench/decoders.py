"""Thin typed Python facade delegating P7501 telemetry & AM34 UART decoders to `panadiga pico --json`."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from scripts.panadiga_cli_bridge import (
    as_dict,
    as_list,
    opt_float,
    opt_int,
    opt_str,
    req_bool,
    req_int,
    req_str,
    run_panadiga_json,
)


def describe_boot_fat_error_code(code: int) -> str:
    """Map `/BOOT.BIN` FAT12/FAT16 negative status codes via `panadiga pico parse-diag --kind fat-error`."""
    rep = as_dict(run_panadiga_json(["pico", "parse-diag", "--kind", "fat-error", f"--code={code}"]))
    return req_str(rep, "text")


@dataclass(frozen=True)
class StatusReport:
    """Decoded output of Pico `STATUS` or `VDD` command."""

    vdd_volts: float | None
    vdd_status: str
    gp26_sticky_low_suspected: bool
    raw_text: str


@dataclass(frozen=True)
class PinCheckRow:
    """One pin row parsed from `PINCHECK` output."""

    pin_label: str
    hiz_volts: float | None
    pull_down_volts: float | None
    pull_up_volts: float | None
    electrical_state: str


@dataclass(frozen=True)
class BusDiagReport:
    """Decoded transition counts and reset rise time from `BUSDIAG`."""

    gp2_transitions: int | None
    gp3_transitions: int | None
    gp4_transitions: int | None
    xreset_rise_us: int | None
    gp2_sole_active_tx: bool


@dataclass(frozen=True)
class SensePacket:
    """Decoded 18-byte ATAPI Sense packet captured from AM34 serial output."""

    raw_bytes: tuple[int, ...]
    sense_key: int
    asc: int
    ascq: int
    vendor_mecha_code: int
    mecha_description: str | None


@dataclass(frozen=True)
class PiologicReport:
    """Decoded 4-channel PIO0 logic timing and NSW5V safety audit from `PIOLOGIC`."""

    fifo_stall: bool
    timing_skew: bool
    edges_count: int
    edges_truncated: bool
    span_us: int | None
    nsw5v_transitions: int | None
    reset_to_first_rxd_us: int | None
    min_tx_to_rx_us: int | None
    max_tx_to_rx_us: int | None


@dataclass(frozen=True)
class PanarspReport:
    """Decoded AM34 `_dev_SERIAL` RSP probe response from `PANARSP`."""

    rx_bytes: int | None
    ack_plus: bool
    nak_minus: bool
    nak_0x15: bool
    payload: str | None
    payload_hex: str | None
    checksum_ok: bool | None
    rsp_status: str
    fifo_overflow: bool


@dataclass(frozen=True)
class UartBootReport:
    """Structured facts decoded from an AM34 115200 8N1 P7501 UART capture."""

    ddr_check_ok: bool
    loader_version: str | None
    boot_mode: str | None
    bgo_failed_code: int | None
    startup_reason: str | None
    sdcd_errors: tuple[int, ...]
    drive_error_codes: tuple[str, ...]
    sense_packets: tuple[SensePacket, ...]
    notes: tuple[str, ...]
    sd_capacity_lba: int | None = None
    sd_capacity_bytes: int | None = None
    sd_bus_width: int | None = None
    sd_transfer_kbps: int | None = None
    sd_hw_init_ok: bool = False
    sd_diag_load_failed: int | None = None
    sd_uncompress_failed: bool = False
    diag_started: bool = False
    fat_mount_failed: int | None = None
    fat_open_failed: tuple[str, int] | None = None
    odd_tray_failed: bool = False
    boot_stages: tuple[str, ...] = ()
    diagnosis: str = ""


def read_captured_log(path: Path) -> str:
    """Read a captured COM10 log file via Rust `decode_captured_log_bytes` (UTF-16LE/BE & UTF-8)."""
    rep = as_dict(
        run_panadiga_json(["pico", "parse-diag", "--kind", "raw-log", "--file", str(path)])
    )
    return req_str(rep, "text")


def parse_status_output(text: str) -> StatusReport:
    """Parse Pico `STATUS` / `VDD` telemetry via `panadiga pico parse-diag --kind status`."""
    rep = as_dict(
        run_panadiga_json(["pico", "parse-diag", "--kind", "status", "--file", "-"], stdin_bytes=text.encode("utf-8"))
    )
    return StatusReport(
        vdd_volts=opt_float(rep, "vdd_volts"),
        vdd_status=req_str(rep, "vdd_status"),
        gp26_sticky_low_suspected=req_bool(rep, "gp26_sticky_low_suspected"),
        raw_text=req_str(rep, "raw_text"),
    )


def parse_pincheck_output(text: str) -> list[PinCheckRow]:
    """Parse `PINCHECK` rows via `panadiga pico parse-diag --kind pincheck`."""
    rows = as_list(
        run_panadiga_json(["pico", "parse-diag", "--kind", "pincheck", "--file", "-"], stdin_bytes=text.encode("utf-8"))
    )
    return [
        PinCheckRow(
            pin_label=req_str(d, "pin_label"),
            hiz_volts=opt_float(d, "hiz_volts"),
            pull_down_volts=opt_float(d, "pull_down_volts"),
            pull_up_volts=opt_float(d, "pull_up_volts"),
            electrical_state=req_str(d, "electrical_state"),
        )
        for d in (as_dict(item) for item in rows)
    ]


def parse_busdiag_output(text: str) -> BusDiagReport:
    """Parse `BUSDIAG` telemetry via `panadiga pico parse-diag --kind busdiag`."""
    rep = as_dict(
        run_panadiga_json(["pico", "parse-diag", "--kind", "busdiag", "--file", "-"], stdin_bytes=text.encode("utf-8"))
    )
    return BusDiagReport(
        gp2_transitions=opt_int(rep, "gp2_transitions"),
        gp3_transitions=opt_int(rep, "gp3_transitions"),
        gp4_transitions=opt_int(rep, "gp4_transitions"),
        xreset_rise_us=opt_int(rep, "xreset_rise_us"),
        gp2_sole_active_tx=req_bool(rep, "gp2_sole_active_tx"),
    )


def _normalize_boot_mode(raw_mode: str | None, boot_stages: tuple[str, ...]) -> str | None:
    if raw_mode == "normal":
        return "normal mode"
    if raw_mode == "serial_boot":
        return "serial boot mode"
    if raw_mode == "cd_diag":
        return "serial boot mode" if "SERIAL_BOOT" in boot_stages else "CD diag boot mode"
    if raw_mode == "emergency":
        return "Emergency"
    return raw_mode


def parse_am34_uart_log(text: str) -> UartBootReport:
    """Decode AM34 UART capture via native Rust `panadiga pico decode-log --file - --json -`."""
    rep = as_dict(
        run_panadiga_json(["pico", "decode-log", "--file", "-"], stdin_bytes=text.encode("utf-8"))
    )
    boot_stages = tuple(str(s) for s in as_list(rep.get("boot_stages")) if isinstance(s, str))
    raw_loader = opt_str(rep, "loader_version")
    loader_version = raw_loader.removeprefix("Loader ") if raw_loader else None

    raw_senses = as_list(rep.get("sense_packets"))
    senses: list[SensePacket] = []
    for item in raw_senses:
        sp = as_dict(item)
        mecha_hex = opt_str(sp, "mecha_error_code")
        vendor_code = int(mecha_hex, 16) if mecha_hex else 0
        raw_bytes = tuple(bytes.fromhex(req_str(sp, "raw_hex").replace(" ", "")))
        senses.append(
            SensePacket(
                raw_bytes=raw_bytes,
                sense_key=req_int(sp, "sense_key"),
                asc=req_int(sp, "asc"),
                ascq=req_int(sp, "ascq"),
                vendor_mecha_code=vendor_code,
                mecha_description=opt_str(sp, "interpretation"),
            )
        )

    raw_fat_open = rep.get("fat_open_failed")
    fat_open_pair: tuple[str, int] | None = None
    if isinstance(raw_fat_open, list) and len(raw_fat_open) == 2:
        fname, fcode = raw_fat_open
        if isinstance(fname, str) and isinstance(fcode, int) and not isinstance(fcode, bool):
            fat_open_pair = (fname, fcode)

    return UartBootReport(
        ddr_check_ok=("DDR_OK" in boot_stages),
        loader_version=loader_version,
        boot_mode=_normalize_boot_mode(opt_str(rep, "boot_mode"), boot_stages),
        bgo_failed_code=opt_int(rep, "bgo_failed_code"),
        startup_reason=opt_str(rep, "startup_reason"),
        sdcd_errors=tuple(int(x) for x in as_list(rep.get("sdcd_errors")) if isinstance(x, int)),
        drive_error_codes=tuple(
            str(x) for x in as_list(rep.get("drive_error_codes")) if isinstance(x, str)
        ),
        sense_packets=tuple(senses),
        notes=tuple(str(x) for x in as_list(rep.get("notes")) if isinstance(x, str)),
        sd_capacity_lba=opt_int(rep, "sd_capacity_lba"),
        sd_capacity_bytes=opt_int(rep, "sd_capacity_bytes"),
        sd_bus_width=opt_int(rep, "sd_bus_width"),
        sd_transfer_kbps=opt_int(rep, "sd_transfer_kbps"),
        sd_hw_init_ok=req_bool(rep, "sd_hw_init_ok"),
        sd_diag_load_failed=opt_int(rep, "sd_diag_load_failed"),
        sd_uncompress_failed=req_bool(rep, "sd_uncompress_failed"),
        diag_started=req_bool(rep, "diag_started"),
        fat_mount_failed=opt_int(rep, "fat_mount_failed"),
        fat_open_failed=fat_open_pair,
        odd_tray_failed=req_bool(rep, "odd_tray_failed"),
        boot_stages=boot_stages,
        diagnosis=req_str(rep, "diagnosis"),
    )


def parse_piologic_output(text: str) -> PiologicReport:
    """Parse `PIOLOGIC` timing telemetry via `panadiga pico parse-diag --kind piologic`."""
    rep = as_dict(
        run_panadiga_json(["pico", "parse-diag", "--kind", "piologic", "--file", "-"], stdin_bytes=text.encode("utf-8"))
    )
    return PiologicReport(
        fifo_stall=req_bool(rep, "fifo_stall"),
        timing_skew=req_bool(rep, "timing_skew"),
        edges_count=req_int(rep, "edges_count"),
        edges_truncated=req_bool(rep, "edges_truncated"),
        span_us=opt_int(rep, "span_us"),
        nsw5v_transitions=opt_int(rep, "nsw5v_transitions"),
        reset_to_first_rxd_us=opt_int(rep, "reset_to_first_rxd_us"),
        min_tx_to_rx_us=opt_int(rep, "min_tx_to_rx_us"),
        max_tx_to_rx_us=opt_int(rep, "max_tx_to_rx_us"),
    )


def parse_panarsp_output(text: str) -> PanarspReport:
    """Parse `PANARSP` probe telemetry via `panadiga pico parse-diag --kind panarsp`."""
    rep = as_dict(
        run_panadiga_json(["pico", "parse-diag", "--kind", "panarsp", "--file", "-"], stdin_bytes=text.encode("utf-8"))
    )
    raw_csum = rep.get("checksum_ok")
    checksum_ok = raw_csum if isinstance(raw_csum, bool) else None
    return PanarspReport(
        rx_bytes=opt_int(rep, "rx_bytes"),
        ack_plus=req_bool(rep, "ack_plus"),
        nak_minus=req_bool(rep, "nak_minus"),
        nak_0x15=req_bool(rep, "nak_0x15"),
        payload=opt_str(rep, "payload"),
        payload_hex=opt_str(rep, "payload_hex"),
        checksum_ok=checksum_ok,
        rsp_status=req_str(rep, "rsp_status"),
        fifo_overflow=req_bool(rep, "fifo_overflow"),
    )
