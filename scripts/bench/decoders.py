"""Decoders for Pico P7501 telemetry (`STATUS`/`PINCHECK`/`BUSDIAG`) and AM34 UART logs."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
from typing import Final, Sequence

_MECHA_ERROR_GLOSSARY: Final[dict[int, str]] = {
    0x83A7: "0.5mW発光ができない(HFｵﾝ密調整) — 0.5mW laser emission failure during HF ON fine tuning",
}

_SDCD_ERROR_GLOSSARY: Final[dict[int, str]] = {
    -91006: "ACMD13 SD Status card-type rejection in GetSD_Status",
    -90002: "SD card identification / bus width initialization failed",
    -90001: "No SD card detected by sdcd_insrmv_check",
}

_BOOT_FAT_ERROR_GLOSSARY: Final[dict[int, str]] = {
    -10010: "FAT_ERR_FILE_NOT_FOUND (0x4400d16b: 8.3 entry missing in root dir)",
    -10011: "FAT_ERR_NO_FREE_FD (0x4400d11e: fd table full or volume unmounted)",
    -10020: "FAT_ERR_INVALID_ARG (0x4400cfde/0x4400c9bd: invalid 8.3 filename or seek offset)",
    -10025: "FAT_ERR_INVALID_FS_OR_IO (0x4400cb21/0x4400c696: bad MBR/BPB, FAT32 rejected, or SD sector read error)",
    -10028: "FAT_ERR_CORRUPT_CLUSTER (0x4400dfc8: FAT cluster entry < 2 or 0xFFF6/0xFFF7)",
    -10030: "FAT_ERR_EOF (0x4400dfc8: read offset >= file_size)",
}


def describe_boot_fat_error_code(code: int) -> str:
    """Map `/BOOT.BIN` FAT12/FAT16 negative status codes (`-10010..-10030`) to root causes."""
    return _BOOT_FAT_ERROR_GLOSSARY.get(
        code,
        "FAT_ERR_UNKNOWN (/BOOT.BIN FAT16/FAT12 negative status)",
    )


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
    """Read a captured COM10 log file handling UTF-16LE/BE BOM or UTF-8 transparently."""
    raw = path.read_bytes()
    if raw.startswith(b"\xff\xfe") or raw.startswith(b"\xfe\xff"):
        return raw.decode("utf-16", errors="replace")
    # Also handle BOM-less UTF-16LE if null bytes appear in even/odd lanes early
    if len(raw) >= 4 and raw[1] == 0 and raw[3] == 0:
        return raw.decode("utf-16-le", errors="replace")
    return raw.decode("utf-8", errors="replace")


def _parse_float_token(token: str) -> float | None:
    match = re.search(r"([0-9]+(?:\.[0-9]+)?)\s*V", token, re.IGNORECASE)
    return float(match.group(1)) if match else None


def parse_status_output(text: str) -> StatusReport:
    """Parse Pico `STATUS` / `VDD` telemetry and detect sticky-LOW GP26 bug."""
    match = re.search(r"VDD:\s*([0-9]+(?:\.[0-9]+)?)\s*V\s*(?:\(([^)]+)\))?", text, re.IGNORECASE)
    vdd = float(match.group(1)) if match else None
    status = match.group(2).strip().upper() if match and match.group(2) else "UNKNOWN"
    sticky_low = vdd is not None and vdd <= 0.10
    return StatusReport(vdd, status, sticky_low, text)


def parse_pincheck_output(text: str) -> list[PinCheckRow]:
    """Parse tabular rows produced by the Pico `PINCHECK` electrical matrix check."""
    rows: list[PinCheckRow] = []
    for line in text.splitlines():
        if "|" not in line or "Pull-Down" in line:
            continue
        cols = [c.strip() for c in line.split("|")]
        if len(cols) < 5 or cols[0].lower().startswith("pin"):
            continue
        rows.append(
            PinCheckRow(
                pin_label=cols[0],
                hiz_volts=_parse_float_token(cols[1]),
                pull_down_volts=_parse_float_token(cols[2]),
                pull_up_volts=_parse_float_token(cols[3]),
                electrical_state=cols[4],
            )
        )
    return rows


def parse_busdiag_output(text: str) -> BusDiagReport:
    """Extract per-pin transition counters and `XRESET` rise time from `BUSDIAG`."""
    def _extract_trans(gpio: str) -> int | None:
        m = re.search(rf"{gpio}\b[^\r\n]*?:\s*(\d+)\s+trans", text, re.IGNORECASE)
        return int(m.group(1)) if m else None

    gp2 = _extract_trans("GP2")
    gp3 = _extract_trans("GP3")
    gp4 = _extract_trans("GP4")
    rise_m = re.search(r"Rise Time\s*\(Hi-Z\)\s*:\s*(\d+)\s*us", text, re.IGNORECASE)
    rise_us = int(rise_m.group(1)) if rise_m else None
    sole_tx = bool(gp2 is not None and gp2 > 0 and (gp3 in (None, 0)) and (gp4 in (None, 0)))
    return BusDiagReport(gp2, gp3, gp4, rise_us, sole_tx)


def _classify_uart_boot_diagnosis(
    *,
    panamon_active: bool,
    diag_started: bool,
    sd_uncompress_failed: bool,
    sd_diag_load_failed: int | None,
    sd_hw_init_ok: bool,
    fat_mount_failed: int | None,
    fat_open_failed: tuple[str, int] | None,
    bgo_failed: int | None,
    boot_mode: str | None,
    sdcd_errs: Sequence[int],
    odd_tray_failed: bool,
    odd_83a7: bool,
    ddr_ok: bool,
) -> str:
    """Compute Stage-1 root-cause diagnosis without masking SD failures under post-reset BGO."""
    if panamon_active:
        return "PANAMON1 active on SC0 (ready for [B/R/W/D/F/J/E/Q])"
    if diag_started:
        return "Stage-1 DIGADIAG.FRM decompressed & jumped (waiting PANAMON1> / factory UI)"
    if sd_uncompress_failed:
        suffix = " (masked by post-reset BGO)" if bgo_failed is not None else " (check container header/footer)"
        return f"Stage-1 DIGADIAG.FRM opened, EXTRHEAD/zlib unpack failed{suffix}"
    if sd_diag_load_failed is not None:
        if fat_mount_failed is not None:
            desc = describe_boot_fat_error_code(fat_mount_failed)
            return f"Stage-1 SD HW OK (4-bit), FAT Mount failed[{fat_mount_failed}] ({desc}) before BGO reboot"
        if fat_open_failed is not None:
            desc = describe_boot_fat_error_code(fat_open_failed[1])
            return f"Stage-1 SD HW OK (4-bit), FAT Open({fat_open_failed[0]}) failed[{fat_open_failed[1]}] ({desc}) before BGO reboot"
        if sd_hw_init_ok:
            extra = " before BGO reboot" if bgo_failed is not None else " (Mount+Open OK, DIGADIAG.FRM < 128KiB or cluster read fail)"
            return f"Stage-1 SD HW OK (4-bit), FAILED at SD diag load failed[{sd_diag_load_failed}] (FUN_4400a97c: Mount+Open OK, fat_read 0x20000 failed){extra}"
        return f"Stage-1 FAILED at SD diag load failed[{sd_diag_load_failed}] (FUN_4400a97c: Mount+Open OK, fat_read 0x20000 failed)"
    if bgo_failed is not None:
        if odd_83a7:
            return f"BGO driver init failed[{bgo_failed}] -> U99 hang (ODD 80040904 / 0x83A7 also observed)"
        if odd_tray_failed:
            return f"BGO driver init failed[{bgo_failed}] -> U99 hang (ODD tray 80045300 also observed)"
        return f"BGO driver init failed[{bgo_failed}] -> U99 hang (load patched DIGADIAG.FRM)"
    if -91006 in sdcd_errs:
        return "SDDIAG -91006 (4-bit ACMD6/ACMD13 SD_STATUS mismatch)"
    if sdcd_errs:
        return f"SDDIAG SD init error ({sdcd_errs[0]})"
    if odd_83a7:
        return "CDDIAG 80040904 / 0x83A7 (Laser APC < 0.5mW)"
    if odd_tray_failed:
        return "ODD ** can not close tray ** (80045300 / no disc)"
    if boot_mode == "serial boot mode":
        return "Serial boot mode entered (waiting SD/CD media)"
    if ddr_ok:
        return "DDR CHECK: OK seen; boot in progress"
    return "No known Panasonic boot signatures observed"


def _reconstruct_diagwatch_target_stream(text: str) -> str:
    """Strip interleaved `[DIAGWATCH...]` probe lines and rejoin split target fragments."""
    lines = text.splitlines()
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.lstrip().startswith("[DIAGWATCH"):
            while i + 1 < len(lines) and lines[i + 1].lstrip().startswith("[DIAGWATCH"):
                i += 1
            if out and i + 1 < len(lines):
                nxt = lines[i + 1]
                if nxt and nxt[0] in "[ .,*)]":
                    out[-1] = out[-1] + nxt
                    i += 2
                    continue
            i += 1
            continue
        out.append(line)
        i += 1
    return "\n".join(out)


def parse_am34_uart_log(text: str) -> UartBootReport:
    """Decode bootloader milestones, SD errors, ATAPI errors, and Sense packets from a UART log."""
    text = _reconstruct_diagwatch_target_stream(text)
    ddr_ok = "DDR CHECK: OK" in text
    ver_m = re.search(r"Loader\s+(ver[0-9.]+\s*\[[^\]]+\])", text)
    loader_version = ver_m.group(1) if ver_m else None

    modes = re.findall(r"\*{5}\s*(normal mode|serial boot mode|FEonly|support card)\s*\*{5}", text, re.IGNORECASE)
    boot_mode = modes[-1] if modes else None

    bgo_m = re.search(r"BGO driver init failed\[(-?\d+)\]", text)
    bgo_failed = int(bgo_m.group(1)) if bgo_m else None

    reason_m = re.search(r"startup reason\s*=\s*\[([^\]]+)\]", text)
    startup_reason = reason_m.group(1) if reason_m else None

    cap_m = re.search(r"capacity:([0-9a-fA-F]+)\[LBA\]\((\d+)\s*Byte\)", text)
    sd_lba_m = re.search(r"SD\s*=\s*0x([0-9a-fA-F]+)\[LBA\]", text)
    sd_capacity_lba = int(cap_m.group(1), 16) if cap_m else (int(sd_lba_m.group(1), 16) if sd_lba_m else None)
    sd_capacity_bytes = int(cap_m.group(2)) if cap_m else (sd_capacity_lba * 512 if sd_capacity_lba is not None else None)

    bw_m = re.search(r"Bus Width:(\d+)\(current\)", text)
    sd_bus_width = int(bw_m.group(1)) if bw_m else None

    spd_m = re.search(r"transfer speed:(\d+)\[Kbps\]", text)
    sd_transfer_kbps = int(spd_m.group(1)) if spd_m else None

    sd_hw_init_ok = "... SD init end" in text or (sd_capacity_lba is not None and sd_bus_width is not None)
    load_fail_m = re.search(r"SD diag load failed\[(-?\d+)\]", text)
    sd_diag_load_failed = int(load_fail_m.group(1)) if load_fail_m else None

    fat_mount_m = re.search(r"FAT Mount failed\[(-?\d+)\]", text)
    fat_mount_failed = int(fat_mount_m.group(1)) if fat_mount_m else None
    fat_open_m = re.search(r"FAT Open\(([^)]+)\)\s*failed\[(-?\d+)\]", text)
    fat_open_failed = (fat_open_m.group(1), int(fat_open_m.group(2))) if fat_open_m else None

    sd_uncompress_failed = any(
        tok in text
        for tok in (
            "diag loader uncompress failed",
            "fvu uncompress failed",
            "no header!",
            "no footer!",
            "check sum mismatch",
        )
    )
    diag_started = "*** start diag ***" in text or "diag loader is preserved" in text
    odd_tray_failed = "** can not close tray" in text or "80045300" in text
    panamon_active = "PANAMON1>" in text

    sdcd_errs: list[int] = []
    for pat in (
        r"SD Status ERROR\s+(-?\d+)",
        r"SD init failed\[(-?\d+)\]",
        r"SDCD:[^:\r\n]+:[^\[\r\n]*\[(-?\d+)\]",
    ):
        for m in re.finditer(pat, text):
            code = int(m.group(1))
            if code not in sdcd_errs:
                sdcd_errs.append(code)

    drive_errs: list[str] = []
    for m in re.finditer(r"X\(([0-9a-fA-F]{6,8})\)", text):
        code_hex = m.group(1).upper()
        if code_hex not in drive_errs:
            drive_errs.append(code_hex)

    senses: list[SensePacket] = []
    seen_sense: set[tuple[int, ...]] = set()
    for m in re.finditer(r"SENSE\[([0-9a-fA-F\s]+)\]", text):
        byte_vals = tuple(int(tok, 16) for tok in m.group(1).split() if len(tok) == 2)
        if len(byte_vals) < 14 or byte_vals in seen_sense:
            continue
        seen_sense.add(byte_vals)
        sk = byte_vals[2] & 0x0F
        asc = byte_vals[12]
        ascq = byte_vals[13]
        mecha = (byte_vals[10] << 8) | byte_vals[11]
        senses.append(SensePacket(byte_vals, sk, asc, ascq, mecha, _MECHA_ERROR_GLOSSARY.get(mecha)))

    odd_83a7 = any(s.vendor_mecha_code == 0x83A7 for s in senses) or bool(
        re.search(r"0x83a7|\b83\s+a7\b|0\.5\s*mw", text, re.IGNORECASE)
    )

    stages: list[str] = []
    if ddr_ok:
        stages.append("DDR_OK")
    if boot_mode == "serial boot mode" or "serial boot mode" in text:
        stages.append("SERIAL_BOOT")
    if sd_hw_init_ok:
        stages.append("SD_HW_OK")
    if sdcd_errs:
        stages.extend(f"SD_ERR_{c}" for c in sdcd_errs)
    if sd_diag_load_failed is not None:
        stages.append("SD_DIAG_LOAD_FAIL")
    if sd_uncompress_failed:
        stages.append("SD_UNCOMPRESS_FAIL")
    if diag_started:
        stages.append("DIAG_EXEC")
    if panamon_active:
        stages.append("PANAMON_ACTIVE")
    if "normal mode" in text:
        stages.append("NORMAL_MODE")
    if bgo_failed is not None:
        stages.append("BGO_FAIL")
    if odd_tray_failed:
        stages.append("ODD_NO_TRAY_80045300")
    if odd_83a7:
        stages.append("ODD_APC_80040904")

    diagnosis = _classify_uart_boot_diagnosis(
        panamon_active=panamon_active,
        diag_started=diag_started,
        sd_uncompress_failed=sd_uncompress_failed,
        sd_diag_load_failed=sd_diag_load_failed,
        sd_hw_init_ok=sd_hw_init_ok,
        fat_mount_failed=fat_mount_failed,
        fat_open_failed=fat_open_failed,
        bgo_failed=bgo_failed,
        boot_mode=boot_mode,
        sdcd_errs=sdcd_errs,
        odd_tray_failed=odd_tray_failed,
        odd_83a7=odd_83a7,
        ddr_ok=ddr_ok,
    )

    notes: list[str] = []
    for code in sdcd_errs:
        if code in _SDCD_ERROR_GLOSSARY:
            notes.append(f"SDCD {code}: {_SDCD_ERROR_GLOSSARY[code]}")
    if sd_diag_load_failed is not None:
        if fat_mount_failed is not None:
            notes.append(f"FAT Mount failed[{fat_mount_failed}]: {describe_boot_fat_error_code(fat_mount_failed)}")
        elif fat_open_failed is not None:
            notes.append(
                f"FAT Open({fat_open_failed[0]}) failed[{fat_open_failed[1]}]: "
                f"{describe_boot_fat_error_code(fat_open_failed[1])}"
            )
        elif sd_hw_init_ok:
            notes.append(
                f"SD diag load failed[{sd_diag_load_failed}] (FUN_4400a97c) without prior FAT Mount/Open error: "
                "FUN_4400c696 (fat_mount) and FUN_4400c879 (fat_open DIGADIAG.FRM) succeeded "
                "(FUN_4400b3d9 prints unconditionally on mount/open error), so FUN_4400c922 fat_read(0x93fc0000, 0x20000) "
                "returned != 0x20000 (DIGADIAG.FRM < 131072 B or SD cluster/multi-block read failure in FUN_4400dfc8 -> FUN_4400ca59). "
                "Check MBR/FAT16 cluster count <= 0xFFF5 and root 8.3 entry."
            )
    if sd_uncompress_failed:
        notes.append(
            "DIGADIAG.FRM was opened from SD, but EXTRHEAD/zlib decompression in FUN_44007ba5 failed."
        )
    if odd_tray_failed:
        notes.append(
            "ODD tray close failure (80045300 / ** can not close tray **): expected when 4-pin tray connector is unplugged or drive cannot close."
        )
    for s in senses:
        if s.mecha_description:
            notes.append(f"Mecha 0x{s.vendor_mecha_code:04X}: {s.mecha_description}")
    if bgo_failed is not None:
        notes.append(
            "FUN_4400836f (bgo_driver_init) halted before fmu_sd_dispatcher(1) "
            "(boot_panic_or_halt_7812), so MAIN/_dev_SERIAL (PANARSP) does not start in NORMAL mode."
        )
    if odd_83a7:
        notes.append(
            "FUN_400db60c reads Ram88000194 (0x000103A7) laser APC fault prior to spindle actuation."
        )

    return UartBootReport(
        ddr_check_ok=ddr_ok,
        loader_version=loader_version,
        boot_mode=boot_mode,
        bgo_failed_code=bgo_failed,
        startup_reason=startup_reason,
        sdcd_errors=tuple(sdcd_errs),
        drive_error_codes=tuple(drive_errs),
        sense_packets=tuple(senses),
        notes=tuple(notes),
        sd_capacity_lba=sd_capacity_lba,
        sd_capacity_bytes=sd_capacity_bytes,
        sd_bus_width=sd_bus_width,
        sd_transfer_kbps=sd_transfer_kbps,
        sd_hw_init_ok=sd_hw_init_ok,
        sd_diag_load_failed=sd_diag_load_failed,
        sd_uncompress_failed=sd_uncompress_failed,
        diag_started=diag_started,
        fat_mount_failed=fat_mount_failed,
        fat_open_failed=fat_open_failed,
        odd_tray_failed=odd_tray_failed,
        boot_stages=tuple(stages),
        diagnosis=diagnosis,
    )


def parse_piologic_output(text: str) -> PiologicReport:
    """Parse `PIOLOGIC` edge counts, FIFO stall/truncation flags, and FMU/MN103S latencies."""
    stall_m = re.search(r"fifo_stall=(YES|NO)", text, re.IGNORECASE)
    edges_m = re.search(r"edges=(\d+)", text, re.IGNORECASE)
    span_m = re.search(r"span=(\d+)us", text, re.IGNORECASE)
    nsw_m = re.search(r"NSW5V[^\r\n]*Transitions=(\d+)", text, re.IGNORECASE)
    rst_m = re.search(r"XRESET_rise\s*->\s*first_RXD_fall:\s*(\d+)\s*us", text, re.IGNORECASE)
    tr_m = re.search(r"TXD_fall\s*->\s*RXD_fall\s*turnaround:\s*min=(\d+)\s*us\s*max=(\d+)\s*us", text, re.IGNORECASE)

    return PiologicReport(
        fifo_stall=bool(stall_m and stall_m.group(1).upper() == "YES"),
        timing_skew="TIMING_SKEW" in text,
        edges_count=int(edges_m.group(1)) if edges_m else 0,
        edges_truncated="(TRUNCATED)" in text,
        span_us=int(span_m.group(1)) if span_m else None,
        nsw5v_transitions=int(nsw_m.group(1)) if nsw_m else None,
        reset_to_first_rxd_us=int(rst_m.group(1)) if rst_m else None,
        min_tx_to_rx_us=int(tr_m.group(1)) if tr_m else None,
        max_tx_to_rx_us=int(tr_m.group(2)) if tr_m else None,
    )


def parse_panarsp_output(text: str) -> PanarspReport:
    """Parse `PANARSP` RX byte count, frame verification, and untruncated `PayloadHex`."""
    rx_m = re.search(r"PANARSP:\s*OK\s*\([^)]*rx=(\d+)\s+bytes", text, re.IGNORECASE)
    status_m = re.search(r"rsp=([A-Z0-9_]+)", text, re.IGNORECASE)
    ovf_m = re.search(r"fifo_ovf=(YES|NO)", text, re.IGNORECASE)
    frame_m = re.search(
        r"\[PANARSP\]\s*Frame:\s*ack=([^\s]+)\s+nak=([^\s]+)\s+nak0x15=([^\s|]+)\s*\|\s*payload=\"([^\"]*)\"",
        text,
    )
    none_m = re.search(
        r"\[PANARSP\]\s*Frame:\s*none\s*\(ack=([^\s]+)\s+nak=([^\s]+)\s+nak0x15=([^)\s]+)\)",
        text,
    )
    full_m = re.search(r"\[PANARSP\]\s*PayloadHex:\s*([^\r\n]+)", text)

    ack = nak = n15 = False
    payload: str | None = None
    csum_ok: bool | None = None
    if frame_m:
        ack = frame_m.group(1) == "+"
        nak = frame_m.group(2) == "-"
        n15 = frame_m.group(3).lower() == "yes"
        payload = frame_m.group(4)
        csum_ok = "(VALID)" in text
    elif none_m:
        ack = none_m.group(1) == "+"
        nak = none_m.group(2) == "-"
        n15 = none_m.group(3).lower() == "yes"

    return PanarspReport(
        rx_bytes=int(rx_m.group(1)) if rx_m else None,
        ack_plus=ack,
        nak_minus=nak,
        nak_0x15=n15,
        payload=payload,
        payload_hex=full_m.group(1).strip() if full_m else None,
        checksum_ok=csum_ok,
        rsp_status=status_m.group(1).upper() if status_m else "UNKNOWN",
        fifo_overflow=bool(ovf_m and ovf_m.group(1).upper() == "YES"),
    )
