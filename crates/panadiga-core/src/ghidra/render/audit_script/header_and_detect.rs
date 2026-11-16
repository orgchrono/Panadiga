//! Python preamble, SSOT byte/MMIO/XREF constants, stage detection, and patch-site classifiers.

use super::super::{format_py_byte_tuple, render_keywords, render_labeled_tuples};
use crate::constants::{
    AM34_BOOT_BIN_BASE, AM34_DRV_S211_BASE, AM34_FLASH_BASE, AM34_FLASH_SIZE,
    AM34_FLASH_UNCACHED_BASE,
};
use crate::ghidra::specs::{
    DRV_S211_EXPECTED_SIZE, MAIN3060_EXPECTED_SIZE, MAIN3410_EXPECTED_SIZE, audit_mmio_constants,
    critical_xref_targets,
};
use crate::re::bgo_unbrick::{
    OFF_BGO_INIT_FAIL_HOOK, OFF_BGO_SUM_BEQ, OFF_DEVINFO_CHECK_BEQ, ORIG_A2CD_HOOK,
    ORIG_DEVINFO_BEQ, ORIG_SUM_BEQ, PATCH_A2CD_AUTOFORMAT, PATCH_DEVINFO_BRA, PATCH_SUM_BRA,
    PROG061_BASE_VA, PROG061_EXPECTED_SIZE,
};
use crate::re::main_passthrough::{
    MAIN_BASE_VA, OFF_FPC_STATUS_CTRL, OFF_ODD_INIT_STATUS, OFF_ODD_STATE_BRANCH,
    ORIG_FPC_STATUS_CTRL, ORIG_ODD_INIT_STATUS, ORIG_ODD_STATE_BRANCH, PATCH_FPC_STATUS_CTRL,
    PATCH_ODD_INIT_STATUS, PATCH_ODD_STATE_BRANCH,
};
use crate::re::prog061_uart_monitor::PATCH_A2CD_UART_MONITOR;

pub(super) fn render_header_and_classifiers() -> String {
    let kw_block = render_keywords();
    let mmio_block = render_labeled_tuples(&audit_mmio_constants());
    let xref_block = render_labeled_tuples(&critical_xref_targets());

    format!(
        r#"# Ghidra Script: Panasonic DMR-EH55 Automated Stage & Unbrick Forensics Audit
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT)
#
# Auto-detects open firmware (BOOT.BIN, PROG061, MAIN3410PQ, MAIN3060NP, DRV_S211, NOR),
# maps missing AM34 MMIO/SDRAM blocks, imports SSOT symbols, verifies live BGO & AV TBC
# opcodes, hunts unindexed MMIO literals & diagnostic strings, adds bookmarks, and writes JSON.

from __future__ import print_function

import json
import os
import struct
import sys
import tempfile

_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if _SCRIPT_DIR not in sys.path:
    sys.path.insert(0, _SCRIPT_DIR)

from ghidra_am34_memory_map import setup_memory_map
from ghidra_import_symbols_panadiga import import_symbols

ORIG_SUM_BEQ = {orig_sum_beq}
PATCH_SUM_BRA = {patch_sum_bra}
ORIG_DEVINFO_BEQ = {orig_devinfo_beq}
PATCH_DEVINFO_BRA = {patch_devinfo_bra}
ORIG_A2CD_HOOK = {orig_a2cd}
PATCH_A2CD_AUTOFORMAT = {patch_a2cd_auto}
PATCH_A2CD_UART_MONITOR = {patch_a2cd_mon}

ORIG_FPC_STATUS_CTRL = {orig_fpc}
PATCH_FPC_STATUS_CTRL = {patch_fpc}
ORIG_ODD_STATE_BRANCH = {orig_odd_br}
PATCH_ODD_STATE_BRANCH = {patch_odd_br}
ORIG_ODD_INIT_STATUS = {orig_odd_st}
PATCH_ODD_INIT_STATUS = {patch_odd_st}

AUDIT_KEYWORDS = (
{kw_block})

AUDIT_MMIO_CONSTANTS = (
{mmio_block})

CRITICAL_XREF_TARGETS = (
{xref_block})

def detect_stage_from_metadata(image_base, prog_name, strings_found, exec_size):
    name_up = (prog_name or "").upper()
    hits = set(strings_found or ())
    if image_base == 0x{prog_base:08X}:
        return "PROG061"
    if image_base == 0x{main_base:08X}:
        return "MAIN3060NP" if (exec_size == {main3060_size} or "3060NP" in name_up) else "MAIN3410PQ"
    if image_base == 0x{boot_base:08X}:
        return "BOOT.BIN"
    if image_base == 0x{drv_base:08X}:
        return "DRV_S211"
    if image_base in (0x{flash_base:08X}, 0x{nor_uncached:08X}) or exec_size == 0x{flash_size:X}:
        return "NOR_8MB"
    if exec_size == {main3060_size} or "3060NP" in name_up:
        return "MAIN3060NP"
    if exec_size == {main3410_size} or "3410PQ" in name_up or "MAIN" in name_up or "_dev_SERIAL" in hits:
        return "MAIN3410PQ"
    if "PROG061" in name_up:
        return "PROG061"
    if exec_size == {drv_size} or "DRV" in name_up:
        return "DRV_S211"
    if "BOOT" in name_up or "-91006" in hits:
        return "BOOT.BIN"
    if exec_size == 0x{prog_size:X} and "BGO driver init" in hits:
        return "PROG061"
    if "SetBusWidth" in hits:
        return "BOOT.BIN"
    return "UNKNOWN"

def _label_pair(actual, orig, patch, orig_name, patch_name):
    if actual is None:
        return "UNMAPPED"
    t = tuple(b & 0xFF for b in actual)
    if t == orig:
        return orig_name
    if t == patch:
        return patch_name
    return "CUSTOM(" + " ".join(["%02X" % b for b in t[:8]]) + ")"

def classify_prog061_sites(sum_bytes, devinfo_bytes, a2cd_bytes):
    sum_state = _label_pair(sum_bytes, ORIG_SUM_BEQ, PATCH_SUM_BRA, "FACTORY", "BYPASSED")
    dev_state = _label_pair(devinfo_bytes, ORIG_DEVINFO_BEQ, PATCH_DEVINFO_BRA, "FACTORY", "BYPASSED")
    a2cd_t = tuple(b & 0xFF for b in a2cd_bytes) if a2cd_bytes is not None else None
    if a2cd_t == ORIG_A2CD_HOOK:
        a2cd_state = "FACTORY"
    elif a2cd_t == PATCH_A2CD_AUTOFORMAT:
        a2cd_state = "AUTO_FORMAT_BGO"
    elif a2cd_t == PATCH_A2CD_UART_MONITOR:
        a2cd_state = "UART_MONITOR_PANAMON1"
    elif a2cd_t is None:
        a2cd_state = "UNMAPPED"
    else:
        a2cd_state = "CUSTOM"

    if sum_state == "FACTORY" and dev_state == "FACTORY" and a2cd_state == "FACTORY":
        mode = "FACTORY_ORIGINAL"
    elif sum_state == "BYPASSED" and dev_state == "BYPASSED" and a2cd_state == "FACTORY":
        mode = "BYPASS_CHECKS_ONLY"
    elif sum_state == "BYPASSED" and dev_state == "BYPASSED" and a2cd_state in ("AUTO_FORMAT_BGO", "UART_MONITOR_PANAMON1"):
        mode = a2cd_state
    else:
        mode = "CUSTOM_OR_PARTIAL"

    return {{
        "mode": mode,
        "sites": {{
            "0x{prog_sum:08X}_bgo_sum_beq": sum_state,
            "0x{prog_devinfo:08X}_devinfo_check_beq": dev_state,
            "0x{prog_a2cd:08X}_bgo_init_fail_hook": a2cd_state,
        }},
    }}

def classify_main3410_sites(fpc_bytes, branch_bytes, status_bytes):
    fpc_state = _label_pair(fpc_bytes, ORIG_FPC_STATUS_CTRL, PATCH_FPC_STATUS_CTRL, "FACTORY", "PATCHED")
    br_state = _label_pair(branch_bytes, ORIG_ODD_STATE_BRANCH, PATCH_ODD_STATE_BRANCH, "FACTORY", "PATCHED")
    st_state = _label_pair(status_bytes, ORIG_ODD_INIT_STATUS, PATCH_ODD_INIT_STATUS, "FACTORY", "PATCHED")

    states = (fpc_state, br_state, st_state)
    if all(s == "FACTORY" for s in states):
        mode = "FACTORY_ORIGINAL"
    elif all(s == "PATCHED" for s in states):
        mode = "AV_PASSTHROUGH"
    else:
        mode = "CUSTOM_OR_PARTIAL"

    return {{
        "mode": mode,
        "sites": {{
            "0x{main_fpc_ctrl:08X}_fpc_status_ctrl": fpc_state,
            "0x{main_odd_branch:08X}_odd_state_branch": br_state,
            "0x{main_odd_status:08X}_odd_init_status": st_state,
        }},
    }}
"#,
        orig_sum_beq = format_py_byte_tuple(&ORIG_SUM_BEQ),
        patch_sum_bra = format_py_byte_tuple(&PATCH_SUM_BRA),
        orig_devinfo_beq = format_py_byte_tuple(&ORIG_DEVINFO_BEQ),
        patch_devinfo_bra = format_py_byte_tuple(&PATCH_DEVINFO_BRA),
        orig_a2cd = format_py_byte_tuple(&ORIG_A2CD_HOOK),
        patch_a2cd_auto = format_py_byte_tuple(&PATCH_A2CD_AUTOFORMAT),
        patch_a2cd_mon = format_py_byte_tuple(&PATCH_A2CD_UART_MONITOR),
        orig_fpc = format_py_byte_tuple(&ORIG_FPC_STATUS_CTRL),
        patch_fpc = format_py_byte_tuple(&PATCH_FPC_STATUS_CTRL),
        orig_odd_br = format_py_byte_tuple(&ORIG_ODD_STATE_BRANCH),
        patch_odd_br = format_py_byte_tuple(&PATCH_ODD_STATE_BRANCH),
        orig_odd_st = format_py_byte_tuple(&ORIG_ODD_INIT_STATUS),
        patch_odd_st = format_py_byte_tuple(&PATCH_ODD_INIT_STATUS),
        boot_base = AM34_BOOT_BIN_BASE,
        prog_base = PROG061_BASE_VA,
        prog_size = PROG061_EXPECTED_SIZE,
        main3060_size = MAIN3060_EXPECTED_SIZE,
        main3410_size = MAIN3410_EXPECTED_SIZE,
        main_base = MAIN_BASE_VA,
        drv_base = AM34_DRV_S211_BASE,
        drv_size = DRV_S211_EXPECTED_SIZE,
        flash_base = AM34_FLASH_BASE,
        nor_uncached = AM34_FLASH_UNCACHED_BASE,
        flash_size = AM34_FLASH_SIZE,
        prog_sum = PROG061_BASE_VA + OFF_BGO_SUM_BEQ as u32,
        prog_devinfo = PROG061_BASE_VA + OFF_DEVINFO_CHECK_BEQ as u32,
        prog_a2cd = PROG061_BASE_VA + OFF_BGO_INIT_FAIL_HOOK as u32,
        main_fpc_ctrl = MAIN_BASE_VA + OFF_FPC_STATUS_CTRL as u32,
        main_odd_branch = MAIN_BASE_VA + OFF_ODD_STATE_BRANCH as u32,
        main_odd_status = MAIN_BASE_VA + OFF_ODD_INIT_STATUS as u32,
    )
}
