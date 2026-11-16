//! Extended CLI handlers exposing MN103S static RE, Trap 1-7, and PANA_DVD/DRV helpers via `--json`.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::re::mn10300::{
    decode_bcc_refs, decode_branch_call_refs, decode_call_frames, decode_regmask_names,
    decode_setlb_loops, find_callers_to, find_fc_immediate_loads, find_pointer_xrefs,
};
use panadiga_core::re::mn10300_traps::{
    DRV_ATAPI_STUB_UNIMPL_VA, DRV_ATAPI_VENDOR_ED_VA, DRV_ATAPI_WRITE_BUFFER_3B_VA,
    InstructionSpan, build_instruction_spans, classify_drv_cli_command, decompress_drv_payload,
    detect_mid_instruction_overlaps, estimate_mn103_insn_len, filter_mid_instruction_constant_hits,
    find_extrhead_containers, identify_firmware_revision, is_flash_cfi_command_va,
    parse_atapi_cdb_dispatch, parse_sense_lut, scan_code_pointer_tables,
};
use serde::Serialize;

use super::json_io::{emit_json_output, parse_opt_u32, parse_u32_csv, read_cli_input_bytes};

#[derive(Serialize)]
struct PairEntry {
    va: u32,
    target_va: u32,
}

#[derive(Serialize)]
struct FilterHitsReport {
    genuine: Vec<u32>,
    false_hits: Vec<u32>,
}

#[derive(Serialize)]
struct HexPayloadReport {
    data_hex: String,
    byte_len: usize,
}

#[derive(Serialize)]
struct ClassifyCliReport {
    command: String,
    flags: u32,
    pass_unlocked: bool,
    classification: String,
}

#[derive(Serialize)]
struct RevisionReport {
    stage: String,
    revision: String,
    pass_handler_va: Option<u32>,
    cli_table_va: Option<u32>,
    atapi_write_buffer_va: Option<u32>,
    atapi_vendor_ed_va: Option<u32>,
}

/// Dispatches `panadiga re mn103` subcommands with clean JSON output.
#[allow(clippy::too_many_arguments)]
pub fn handle_mn103_inspect(
    kind: &str,
    input: Option<PathBuf>,
    base: &str,
    va_start: Option<String>,
    va_end: Option<String>,
    target: Option<String>,
    targets: Option<String>,
    max_abs_disp: &str,
    max_body_bytes: usize,
    regmask: u8,
    json: Option<PathBuf>,
) -> Result<()> {
    if kind.eq_ignore_ascii_case("regmask") {
        let groups: Vec<&'static str> = decode_regmask_names(regmask);
        return emit_json_output(&groups, json.as_deref());
    }
    let path = input.context("--input is required for this MN103 inspection kind")?;
    let data = read_cli_input_bytes(&path)?;
    let base_va = parse_u32_radix(base)?;
    let s_va = parse_opt_u32(va_start.as_deref())?;
    let e_va = parse_opt_u32(va_end.as_deref())?;
    let max_disp = parse_u32_radix(max_abs_disp)?;

    match kind {
        "branches" => {
            let res = decode_branch_call_refs(&data, base_va, s_va, e_va, max_disp);
            emit_json_output(&res, json.as_deref())
        }
        "bcc" => {
            let res = decode_bcc_refs(&data, base_va, s_va, e_va);
            emit_json_output(&res, json.as_deref())
        }
        "call-frames" => {
            let res = decode_call_frames(&data, base_va, s_va, e_va, max_disp);
            emit_json_output(&res, json.as_deref())
        }
        "setlb" => {
            let res = decode_setlb_loops(&data, base_va, s_va, e_va, max_body_bytes);
            emit_json_output(&res, json.as_deref())
        }
        "fc-loads" => {
            let lo = s_va.unwrap_or(base_va);
            let hi = e_va.unwrap_or_else(|| base_va.wrapping_add(data.len() as u32));
            let res = find_fc_immediate_loads(&data, base_va, lo, hi);
            emit_json_output(&res, json.as_deref())
        }
        "pointer-xrefs" => {
            let tgt = parse_u32_radix(target.as_deref().context("--target is required")?)?;
            let res = find_pointer_xrefs(&data, base_va, tgt);
            emit_json_output(&res, json.as_deref())
        }
        "callers" => {
            let mut tgt_list = parse_u32_csv(targets.as_deref())?;
            if let Some(t) = target.as_deref() {
                tgt_list.push(parse_u32_radix(t)?);
            }
            let res = find_callers_to(&data, base_va, &tgt_list);
            emit_json_output(&res, json.as_deref())
        }
        other => bail!(
            "Unknown mn103 inspect kind '{other}'. Expected branches|bcc|call-frames|setlb|fc-loads|pointer-xrefs|callers|regmask"
        ),
    }
}

/// Dispatches `panadiga re traps` subcommands with clean JSON output.
#[allow(clippy::too_many_arguments)]
pub fn handle_traps_inspect(
    kind: &str,
    input: Option<PathBuf>,
    base: &str,
    code_lo: Option<String>,
    code_hi: Option<String>,
    min_run: usize,
    stub_vas: Option<String>,
    va: Option<String>,
    extra_spans: Option<String>,
    hits: Option<String>,
    opcode: u8,
    json: Option<PathBuf>,
) -> Result<()> {
    match kind {
        "insn-len" => emit_json_output(&estimate_mn103_insn_len(opcode), json.as_deref()),
        "cfi" => {
            let va_u32 = parse_u32_radix(va.as_deref().context("--va is required")?)?;
            emit_json_output(&is_flash_cfi_command_va(va_u32), json.as_deref())
        }
        "filter-hits" => {
            let hit_vas = parse_u32_csv(hits.as_deref())?;
            let spans: Vec<InstructionSpan> = match extra_spans.as_deref() {
                Some(raw) => serde_json::from_str(raw)?,
                None => Vec::new(),
            };
            let (genuine, false_hits) = filter_mid_instruction_constant_hits(&hit_vas, &spans);
            emit_json_output(
                &FilterHitsReport {
                    genuine,
                    false_hits,
                },
                json.as_deref(),
            )
        }
        "spans" | "overlaps" | "ptr-tables" | "revision" => {
            let data = read_cli_input_bytes(&input.context("--input is required")?)?;
            let base_va = parse_u32_radix(base)?;
            match kind {
                "spans" => {
                    emit_json_output(&build_instruction_spans(&data, base_va), json.as_deref())
                }
                "overlaps" => {
                    let extra: Vec<InstructionSpan> = match extra_spans.as_deref() {
                        Some(raw) => serde_json::from_str(raw)?,
                        None => Vec::new(),
                    };
                    let res = detect_mid_instruction_overlaps(&data, base_va, &extra);
                    emit_json_output(&res, json.as_deref())
                }
                "ptr-tables" => {
                    let lo = parse_opt_u32(code_lo.as_deref())?.unwrap_or(base_va);
                    let hi = parse_opt_u32(code_hi.as_deref())?
                        .unwrap_or_else(|| base_va.wrapping_add(data.len() as u32));
                    let stubs = if stub_vas.is_some() {
                        parse_u32_csv(stub_vas.as_deref())?
                    } else {
                        vec![DRV_ATAPI_STUB_UNIMPL_VA]
                    };
                    let pairs: Vec<PairEntry> =
                        scan_code_pointer_tables(&data, base_va, lo, hi, min_run, &stubs)
                            .into_iter()
                            .map(|(entry_va, target_va)| PairEntry {
                                va: entry_va,
                                target_va,
                            })
                            .collect();
                    emit_json_output(&pairs, json.as_deref())
                }
                "revision" => {
                    let rev = identify_firmware_revision(&data);
                    let is_drv_decomp = rev.revision == "DRV_decomp";
                    let fp = RevisionReport {
                        stage: rev.stage,
                        revision: rev.revision,
                        pass_handler_va: rev.pass_handler_va,
                        cli_table_va: rev.cli_table_va,
                        atapi_write_buffer_va: is_drv_decomp
                            .then_some(DRV_ATAPI_WRITE_BUFFER_3B_VA),
                        atapi_vendor_ed_va: is_drv_decomp.then_some(DRV_ATAPI_VENDOR_ED_VA),
                    };
                    emit_json_output(&fp, json.as_deref())
                }
                _ => unreachable!(),
            }
        }
        other => bail!(
            "Unknown traps inspect kind '{other}'. Expected spans|overlaps|filter-hits|cfi|ptr-tables|revision|insn-len"
        ),
    }
}

/// Dispatches `panadiga re pana-frm` subcommands with clean JSON output.
#[allow(clippy::too_many_arguments)]
pub fn handle_pana_frm_inspect(
    kind: &str,
    input: Option<PathBuf>,
    base: &str,
    offset: &str,
    count: usize,
    header_size: usize,
    default_handler: &str,
    cmd_name: Option<String>,
    flags: u32,
    pass_unlocked: bool,
    json: Option<PathBuf>,
) -> Result<()> {
    if kind == "classify-cli" {
        let name = cmd_name.context("--cmd-name is required")?;
        let classification = classify_drv_cli_command(&name, flags, pass_unlocked).to_string();
        return emit_json_output(
            &ClassifyCliReport {
                command: name,
                flags,
                pass_unlocked,
                classification,
            },
            json.as_deref(),
        );
    }

    let data = read_cli_input_bytes(&input.context("--input is required")?)?;
    let base_va = parse_u32_radix(base)?;
    let off = parse_u32_radix(offset)? as usize;

    match kind {
        "decompress-drv" => {
            let decomp = decompress_drv_payload(&data, header_size)?;
            emit_json_output(
                &HexPayloadReport {
                    byte_len: decomp.len(),
                    data_hex: hex::encode(decomp),
                },
                json.as_deref(),
            )
        }
        "extrhead" => emit_json_output(&find_extrhead_containers(&data, base_va), json.as_deref()),
        "atapi-dispatch" => {
            let def_va = parse_u32_radix(default_handler)?;
            let slots = parse_atapi_cdb_dispatch(&data, off, base_va, count, def_va);
            emit_json_output(&slots, json.as_deref())
        }
        "sense-lut" => {
            let entries = parse_sense_lut(&data, off, base_va, count);
            emit_json_output(&entries, json.as_deref())
        }
        other => bail!(
            "Unknown pana-frm kind '{other}'. Expected decompress-drv|extrhead|atapi-dispatch|sense-lut|classify-cli"
        ),
    }
}
