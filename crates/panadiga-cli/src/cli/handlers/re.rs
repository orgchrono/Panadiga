//! Thin CLI handlers for Matsushita MN103S / AM34 static reverse-engineering audits.

use anyhow::Result;
use panadiga_core::crypto::parse_u32_radix;
use panadiga_core::log_println;
use panadiga_core::re::mn10300::{find_callers_to, find_fc_immediate_loads, find_pointer_xrefs};
use panadiga_core::re::mn10300_traps::{
    analyze_seven_traps_in_slice, identify_firmware_revision, parse_drv_cli_table,
    parse_drv_mecha_error_table,
};
use serde::Serialize;

use super::json_io::read_cli_input_bytes;
use super::re_extra::{handle_mn103_inspect, handle_pana_frm_inspect, handle_traps_inspect};
use super::unbrick::render::emit_json_if_requested;
use crate::cli::args::re::{ReActions, ReArgs};

#[derive(Serialize)]
struct XrefsReport {
    base_va: String,
    target_va: String,
    callers: Vec<String>,
    pointer_xrefs: Vec<String>,
    fc_load_sites: Vec<String>,
}

fn infer_base_va(data: &[u8]) -> u32 {
    let rev = identify_firmware_revision(data);
    match rev.stage.as_str() {
        "DRV" => 0x4000_0000,
        "BOOT_OR_PROG061" => {
            if data
                .windows(8)
                .any(|w| w == b"FIRMUP\0\0" || w == b"FVU\0\0\0\0\0")
                || data.len() == 0x0004_0000 && data.starts_with(b"\x01\x00\x00\x00")
            {
                0x5040_0000
            } else {
                0x4400_0000
            }
        }
        _ => {
            if data.len() > 0x0040_0000 {
                0x5700_0000
            } else {
                0x4400_0000
            }
        }
    }
}

fn load_binary_and_base(input: &std::path::Path, base: Option<&str>) -> Result<(Vec<u8>, u32)> {
    let data = read_cli_input_bytes(input)?;
    let base_va = match base {
        Some(raw) => parse_u32_radix(raw)?,
        None => infer_base_va(&data),
    };
    Ok((data, base_va))
}

pub fn handle_re(args: ReArgs) -> Result<()> {
    match args.action {
        ReActions::Audit {
            input,
            base,
            code_lo,
            code_hi,
            json,
        } => {
            let (data, base_va) = load_binary_and_base(&input, base.as_deref())?;
            let lo = match code_lo {
                Some(ref raw) => parse_u32_radix(raw)?,
                None => base_va,
            };
            let hi = match code_hi {
                Some(ref raw) => parse_u32_radix(raw)?,
                None => base_va.wrapping_add(data.len() as u32),
            };
            let summary = analyze_seven_traps_in_slice(&data, base_va, lo, hi);
            log_println!("=== PANADIGA MN103S SEVEN TRAPS & AM33 SLEIGH AUDIT ===");
            log_println!(
                "Image        : {} ({} bytes) | Base=0x{:08X}",
                input.display(),
                data.len(),
                base_va
            );
            log_println!(
                "Revision     : {} ({})",
                summary.revision_profile.stage,
                summary.revision_profile.revision
            );
            log_println!("Orphan ptrs  : {}", summary.orphan_code_pointers.len());
            log_println!("CFI sites    : {}", summary.cfi_constant_sites.len());
            log_println!("Mid overlaps : {}", summary.mid_instruction_overlaps.len());
            log_println!("SETLB loops  : {}", summary.setlb_loop_count);
            log_println!("AM33 F9 EA   : {}", summary.am33_f9_ea_sites.len());
            log_println!("AM33 FF RETS : {}", summary.am33_ff_pre_rets_sites.len());
            emit_json_if_requested(&summary, json.as_deref())
        }
        ReActions::Xrefs {
            input,
            target,
            base,
            json,
        } => {
            let (data, base_va) = load_binary_and_base(&input, base.as_deref())?;
            let target_va = parse_u32_radix(&target)?;
            let callers = find_callers_to(&data, base_va, &[target_va]);
            let ptrs = find_pointer_xrefs(&data, base_va, target_va);
            let upper = base_va.wrapping_add(data.len().saturating_sub(6) as u32);
            let fc_loads: Vec<String> = find_fc_immediate_loads(&data, base_va, base_va, upper)
                .into_iter()
                .filter(|r| r.value == target_va)
                .map(|r| format!("0x{:08X} (0xFC {:02X})", r.va, r.sub_opcode))
                .collect();

            let report = XrefsReport {
                base_va: format!("0x{base_va:08X}"),
                target_va: format!("0x{target_va:08X}"),
                callers: callers
                    .iter()
                    .map(|c| format!("0x{:08X} ({})", c.va, c.mnemonic))
                    .collect(),
                pointer_xrefs: ptrs.iter().map(|va| format!("0x{va:08X}")).collect(),
                fc_load_sites: fc_loads,
            };
            log_println!("=== MN103S XREFS FOR 0x{target_va:08X} (base 0x{base_va:08X}) ===");
            log_println!("Callers/Branches : {}", report.callers.len());
            for c in report.callers.iter().take(20) {
                log_println!("  - {c}");
            }
            log_println!("Pointer LE32 refs: {}", report.pointer_xrefs.len());
            for p in report.pointer_xrefs.iter().take(20) {
                log_println!("  - {p}");
            }
            log_println!("0xFC imm32 refs  : {}", report.fc_load_sites.len());
            for f in report.fc_load_sites.iter().take(20) {
                log_println!("  - {f}");
            }
            emit_json_if_requested(&report, json.as_deref())
        }
        ReActions::DrvCli {
            input,
            offset,
            base,
            max_entries,
            pass_unlocked,
            json,
        } => {
            let data = read_cli_input_bytes(&input)?;
            let off = parse_u32_radix(&offset)? as usize;
            let base_va = parse_u32_radix(&base)?;
            let entries = parse_drv_cli_table(&data, off, base_va, max_entries, pass_unlocked);
            log_println!(
                "=== DRV CLI TABLE ({} entries, PASS={}) ===",
                entries.len(),
                if pass_unlocked { "UNLOCKED" } else { "LOCKED" }
            );
            for e in &entries {
                log_println!(
                    "  [{:02}] {:<10} @ 0x{:08X} | flags=0x{:02X} | {}",
                    e.index,
                    e.name,
                    e.handler_va,
                    e.flags,
                    e.classification
                );
            }
            emit_json_if_requested(&entries, json.as_deref())
        }
        ReActions::DrvErrors {
            input,
            start,
            end,
            json,
        } => {
            let data = read_cli_input_bytes(&input)?;
            let s = parse_u32_radix(&start)? as usize;
            let e = parse_u32_radix(&end)? as usize;
            let records = parse_drv_mecha_error_table(&data, s, e);
            log_println!(
                "=== DRV MECHA ERROR TABLE ({} records in 0x{:X}..0x{:X}) ===",
                records.len(),
                s,
                e
            );
            for r in records.iter().take(30) {
                log_println!(
                    "  [0x{:05X}] {} | {}",
                    r.offset,
                    r.code_hex,
                    r.ascii_preview
                );
            }
            emit_json_if_requested(&records, json.as_deref())
        }
        ReActions::Mn103 {
            common,
            va_start,
            va_end,
            target,
            targets,
            max_abs_disp,
            max_body_bytes,
            regmask,
        } => handle_mn103_inspect(
            &common.kind,
            common.input,
            &common.base,
            va_start,
            va_end,
            target,
            targets,
            &max_abs_disp,
            max_body_bytes,
            regmask,
            common.json,
        ),
        ReActions::Traps {
            common,
            code_lo,
            code_hi,
            min_run,
            stub_vas,
            va,
            extra_spans,
            hits,
            opcode,
        } => handle_traps_inspect(
            &common.kind,
            common.input,
            &common.base,
            code_lo,
            code_hi,
            min_run,
            stub_vas,
            va,
            extra_spans,
            hits,
            opcode,
            common.json,
        ),
        ReActions::PanaFrm {
            kind,
            input,
            base,
            offset,
            count,
            header_size,
            default_handler,
            cmd_name,
            flags,
            pass_unlocked,
            json,
        } => handle_pana_frm_inspect(
            &kind,
            input,
            &base,
            &offset,
            count,
            header_size,
            &default_handler,
            cmd_name,
            flags,
            pass_unlocked,
            json,
        ),
    }
}
