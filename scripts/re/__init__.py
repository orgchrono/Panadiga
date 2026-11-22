"""Reusable Matsushita MN103S / PANA_DVD.FRM static reverse-engineering toolkit."""

from scripts.re.mn10300 import (
    InstructionRef,
    ImmediateLoadRef,
    decode_branch_call_refs,
    find_callers_to,
    find_fc_immediate_loads,
    find_pointer_xrefs,
)
from scripts.re.mn103_traps import (
    DrvRevisionFingerprint,
    InstructionSpan,
    OverlapCollision,
    build_instruction_spans,
    detect_mid_instruction_overlaps,
    filter_mid_instruction_constant_hits,
    identify_drv_revision,
    is_cfi_command_address,
    scan_code_pointer_tables,
)
from scripts.re.pana_frm import (
    DRV_UNLOCK_COMMAND,
    AtapiDispatchEntry,
    DrvCliEntry,
    DrvMechaErrorEntry,
    ExtrheadMatch,
    SenseEntry,
    classify_drv_cli_command,
    decompress_drv_payload,
    find_extrhead_containers,
    parse_atapi_cdb_dispatch,
    parse_drv_cli_table,
    parse_drv_error_table,
    parse_sense_lut,
)
from importlib import import_module
from typing import Any

from scripts.re._discovery_common import (
    infer_model_id_from_path,
    is_panasonic_frm_path,
    scan_embedded_filesystems,
)

_LAZY_MODULE_EXPORTS: dict[str, tuple[str, str]] = {
    "decrypt_fleet": ("scripts.re.fleet_decryptor", "decrypt_fleet"),
    "harvest_frm_trains": ("scripts.re.frm_train_harvester", "harvest_frm_trains"),
    "resolve_language_for_record": ("scripts.re.ghidra_fleet_orchestrator", "resolve_language_for_record"),
}


def __getattr__(name: str) -> Any:
    """Lazy-load CLI script entrypoints so `python -m scripts.re.<script>` triggers no `runpy` warning."""
    target = _LAZY_MODULE_EXPORTS.get(name)
    if target is None:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    mod_name, attr_name = target
    return getattr(import_module(mod_name), attr_name)

__all__ = [
    "DRV_UNLOCK_COMMAND",
    "AtapiDispatchEntry",
    "DrvCliEntry",
    "DrvMechaErrorEntry",
    "DrvRevisionFingerprint",
    "ExtrheadMatch",
    "ImmediateLoadRef",
    "InstructionRef",
    "InstructionSpan",
    "OverlapCollision",
    "SenseEntry",
    "build_instruction_spans",
    "classify_drv_cli_command",
    "decode_branch_call_refs",
    "decompress_drv_payload",
    "decrypt_fleet",
    "detect_mid_instruction_overlaps",
    "filter_mid_instruction_constant_hits",
    "find_callers_to",
    "find_extrhead_containers",
    "find_fc_immediate_loads",
    "find_pointer_xrefs",
    "harvest_frm_trains",
    "identify_drv_revision",
    "infer_model_id_from_path",
    "is_cfi_command_address",
    "is_panasonic_frm_path",
    "parse_atapi_cdb_dispatch",
    "parse_drv_cli_table",
    "parse_drv_error_table",
    "parse_sense_lut",
    "resolve_language_for_record",
    "scan_embedded_filesystems",
    "scan_code_pointer_tables",
]
