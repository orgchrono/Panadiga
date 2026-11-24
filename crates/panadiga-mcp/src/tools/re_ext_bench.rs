//! Handlers for `panadiga_eh55_bgo` and `panadiga_davis_probe` MCP tools.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::{fs, path::Path, time::Duration};

use panadiga_core::constants::{
    AM34_DAVIS_SEC_MMIO_BASE, AM34_FORBIDDEN_LETHAL_REG, DEFAULT_BAUD_RATE,
};
use panadiga_core::nvram::calculate_hashes;
use panadiga_core::pico::panamon1::{Panamon1Client, Panamon1Transport, SerialPanamon1Transport};
use panadiga_core::re::bgo_unbrick::ensure_parent_dir;
use panadiga_core::re::davis_probe::{
    CompiledProbeStub, DAVIS_IDENTIFY_BUF_DEFAULT_VA, DAVIS_STUB_DEFAULT_VA, DavisProbeConfig,
    DavisProbeEngine, build_ata_identify_ec_stub, build_ch0_probe_stub, build_ch1_probe_stub,
    build_fe_release_stub, parse_ata_identify_pio_buffer, parse_ata_identify_words,
    validate_safe_mmio_read_range,
};
use panadiga_core::re::eh55_bgo::{
    BgoRepairPlan, apply_bgo_repair_plan_live, audit_eh55_bgo_nor_8mb, build_bgo_repair_plan,
};

use super::forensics_re::parse_u32_arg;
use crate::policy::McpSecurityPolicy;
use crate::workspace::{
    optional_non_empty_str, paths_refer_to_same_file, require_non_empty_str, resolve_workspace_path,
};

fn run_with_panamon1_client<T: Panamon1Transport, R>(
    args: &Value,
    transport_override: Option<T>,
    f: impl FnOnce(&mut Panamon1Client<T>) -> Result<R>,
    f_serial: impl FnOnce(&mut Panamon1Client<SerialPanamon1Transport>) -> Result<R>,
) -> Result<R> {
    if let Some(t) = transport_override {
        let mut client = Panamon1Client::new(t);
        let out = f(&mut client)?;
        let _ = client.close_bridge_and_isolate()?;
        Ok(out)
    } else {
        let port = require_non_empty_str(args, "port")?;
        let t =
            SerialPanamon1Transport::open(port, DEFAULT_BAUD_RATE, Duration::from_millis(1_500))?;
        let mut client = Panamon1Client::new(t);
        let out = f_serial(&mut client)?;
        let _ = client.close_bridge_and_isolate()?;
        Ok(out)
    }
}

fn stub_summary(stub: CompiledProbeStub) -> Value {
    let (_, sha256) = calculate_hashes(&stub.code);
    json!({
        "name": stub.name,
        "load_va": format!("0x{:08X}", stub.load_va),
        "code_len": stub.code.len(),
        "code_sha256": sha256,
        "output_va": stub.output_va.map(|v| format!("0x{v:08X}")),
        "output_len": stub.output_len,
        "meta_va": stub.meta_va.map(|v| format!("0x{v:08X}")),
        "code_hex": hex::encode_upper(&stub.code),
    })
}

fn parse_drive_sel_opt(args: &Value) -> Result<u8> {
    match optional_non_empty_str(args, "drive")
        .unwrap_or("master")
        .to_ascii_lowercase()
        .as_str()
    {
        "master" => Ok(0xA0),
        "slave" => Ok(0xB0),
        other => bail!("Unsupported drive '{other}': expected 'master' or 'slave'"),
    }
}

pub fn handle_eh55_bgo_with_policy<T: Panamon1Transport>(
    args: &Value,
    workspace_root: &Path,
    transport_override: Option<T>,
    policy: &McpSecurityPolicy,
) -> Result<Value> {
    let action = require_non_empty_str(args, "action")?.to_ascii_lowercase();
    match action.as_str() {
        "audit" => {
            let nor_raw = optional_non_empty_str(args, "nor_path")
                .or_else(|| optional_non_empty_str(args, "input_path"))
                .context("Missing 'nor_path' for eh55_bgo audit")?;
            let nor_path = resolve_workspace_path(workspace_root, nor_raw)?;
            let nor = fs::read(&nor_path)?;
            Ok(serde_json::to_value(audit_eh55_bgo_nor_8mb(&nor)?)?)
        }
        "plan" => {
            let nor_raw = optional_non_empty_str(args, "nor_path")
                .or_else(|| optional_non_empty_str(args, "input_path"))
                .context("Missing 'nor_path' for eh55_bgo plan")?;
            let nor_path = resolve_workspace_path(workspace_root, nor_raw)?;
            let out_plan_dst = optional_non_empty_str(args, "out_plan_path")
                .map(|p| resolve_workspace_path(workspace_root, p))
                .transpose()?;
            let cow_dst = optional_non_empty_str(args, "cow_out_path")
                .map(|p| resolve_workspace_path(workspace_root, p))
                .transpose()?;
            if let Some(dst) = out_plan_dst.as_deref() {
                policy.require_file_write("panadiga_eh55_bgo", Some(dst))?;
            }
            if let Some(dst) = cow_dst.as_deref() {
                policy.require_file_write("panadiga_eh55_bgo", Some(dst))?;
                if paths_refer_to_same_file(&nor_path, dst) {
                    bail!("CoW safety violation: cow_out_path must differ from nor_path");
                }
            }

            let nor = fs::read(&nor_path)?;
            let ref_bytes = optional_non_empty_str(args, "reference_nor_path")
                .map(|p| {
                    resolve_workspace_path(workspace_root, p).and_then(|abs| Ok(fs::read(abs)?))
                })
                .transpose()?;
            let plan = build_bgo_repair_plan(&nor, ref_bytes.as_deref())?;

            let mut out_plan_saved = None;
            if let Some(dst) = out_plan_dst.as_deref() {
                ensure_parent_dir(dst)?;
                fs::write(dst, serde_json::to_vec_pretty(&plan)?)?;
                out_plan_saved = Some(dst.display().to_string());
            }
            let mut cow_saved = None;
            if let Some(dst) = cow_dst.as_deref() {
                let img = plan
                    .repaired_nor_image
                    .as_deref()
                    .context("Missing CoW repaired_nor_image")?;
                ensure_parent_dir(dst)?;
                fs::write(dst, img)?;
                cow_saved = Some(dst.display().to_string());
            }
            Ok(json!({
                "out_plan_path": out_plan_saved,
                "cow_out_path": cow_saved,
                "sa0_to_sa10_locked_e05": true,
                "plan": plan,
            }))
        }
        "live_apply" => {
            policy.require_hardware_hitl(
                "panadiga_eh55_bgo",
                optional_non_empty_str(args, "hitl_token"),
            )?;
            let confirm = args
                .get("confirm_upper_nor")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !confirm {
                bail!(
                    "E05 / Upper NOR safety lockout: live_apply requires confirm_upper_nor=true (Layer 1 256 KiB IPL SA0..SA10 remain permanently E05-locked)"
                );
            }
            let plan_path =
                resolve_workspace_path(workspace_root, require_non_empty_str(args, "plan_path")?)?;
            let backup_dir =
                resolve_workspace_path(workspace_root, require_non_empty_str(args, "backup_dir")?)?;
            policy.require_file_write("panadiga_eh55_bgo", Some(&backup_dir))?;
            let plan: BgoRepairPlan = serde_json::from_slice(&fs::read(&plan_path)?)
                .with_context(|| format!("Parsing BgoRepairPlan {}", plan_path.display()))?;
            let no_trace = args
                .get("no_boottrace")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let trace_ms = if no_trace {
                None
            } else {
                Some(parse_u32_arg(args, "boot_trace_ms", 15_000)?)
            };
            let report = run_with_panamon1_client(
                args,
                transport_override,
                |c| {
                    apply_bgo_repair_plan_live(
                        c,
                        &plan,
                        &backup_dir,
                        confirm,
                        trace_ms,
                        &mut |_| {},
                    )
                },
                |c| {
                    apply_bgo_repair_plan_live(
                        c,
                        &plan,
                        &backup_dir,
                        confirm,
                        trace_ms,
                        &mut |_| {},
                    )
                },
            )?;
            Ok(serde_json::to_value(report)?)
        }
        other => bail!("Unsupported panadiga_eh55_bgo action '{other}'"),
    }
}

macro_rules! define_bench_ro_wrappers {
    ($with_transport:ident, $plain:ident, $with_policy:ident) => {
        pub fn $with_transport<T: Panamon1Transport>(
            args: &Value,
            workspace_root: &Path,
            transport_override: Option<T>,
        ) -> Result<Value> {
            $with_policy(
                args,
                workspace_root,
                transport_override,
                &McpSecurityPolicy::read_only(),
            )
        }

        pub fn $plain(args: &Value, workspace_root: &Path) -> Result<Value> {
            $with_transport::<SerialPanamon1Transport>(args, workspace_root, None)
        }
    };
}

define_bench_ro_wrappers!(
    handle_eh55_bgo_with_transport,
    handle_eh55_bgo,
    handle_eh55_bgo_with_policy
);

pub fn handle_davis_probe_with_policy<T: Panamon1Transport>(
    args: &Value,
    workspace_root: &Path,
    transport_override: Option<T>,
    policy: &McpSecurityPolicy,
) -> Result<Value> {
    let stub_va = parse_u32_arg(args, "sdram_stub_va", DAVIS_STUB_DEFAULT_VA)?;
    let out_va = parse_u32_arg(args, "identify_out_va", DAVIS_IDENTIFY_BUF_DEFAULT_VA)?;
    let drive_sel = parse_drive_sel_opt(args)?;
    let lethal_blocked = validate_safe_mmio_read_range(AM34_FORBIDDEN_LETHAL_REG, 4).is_err();
    let mode = require_non_empty_str(args, "action")?.to_ascii_lowercase();

    match mode.as_str() {
        "build_stubs" => {
            let scratch = stub_va & 0xFFFF_0000;
            let fe = build_fe_release_stub(stub_va);
            let ch0 = build_ch0_probe_stub(stub_va, scratch.wrapping_add(0x1000))?;
            let ch1 = build_ch1_probe_stub(
                stub_va,
                AM34_DAVIS_SEC_MMIO_BASE,
                scratch.wrapping_add(0x1100),
                scratch.wrapping_add(0x1140),
                scratch.wrapping_add(0x1180),
            )?;
            let ec = build_ata_identify_ec_stub(AM34_DAVIS_SEC_MMIO_BASE, drive_sel, out_va)?;
            Ok(json!({
                "lethal_0xc000100c_blocked": lethal_blocked,
                "drive_sel": format!("0x{drive_sel:02X}"),
                "stubs": [
                    stub_summary(fe),
                    stub_summary(ch0),
                    stub_summary(ch1),
                    stub_summary(ec),
                ]
            }))
        }
        "parse_ec" => {
            let raw = if let Some(h) = optional_non_empty_str(args, "ec_hex") {
                hex::decode(h.trim())?
            } else {
                let p = resolve_workspace_path(
                    workspace_root,
                    require_non_empty_str(args, "input_path")?,
                )?;
                fs::read(&p)?
            };
            let info = if raw.len() == 1024 {
                parse_ata_identify_pio_buffer(&raw)?
            } else if raw.len() == 512 {
                parse_ata_identify_words(&raw)?
            } else {
                bail!(
                    "Expected 512B raw or 1024B PIO IDENTIFY buffer, got {} B",
                    raw.len()
                );
            };
            Ok(json!({
                "lethal_0xc000100c_blocked": lethal_blocked,
                "buffer_len": raw.len(),
                "identify_info": info
            }))
        }
        "run_live" => {
            policy.require_hardware_hitl(
                "panadiga_davis_probe",
                optional_non_empty_str(args, "hitl_token"),
            )?;
            let cfg = DavisProbeConfig {
                sdram_stub_va: stub_va,
                identify_out_va: out_va,
                drive_sel,
                ..DavisProbeConfig::default()
            };
            let report = run_with_panamon1_client(
                args,
                transport_override,
                |c| DavisProbeEngine::new(c, cfg.clone()).run_probe(),
                |c| DavisProbeEngine::new(c, cfg.clone()).run_probe(),
            )?;
            Ok(json!({
                "lethal_0xc000100c_blocked": lethal_blocked,
                "report": report
            }))
        }
        other => bail!("Unsupported panadiga_davis_probe action '{other}'"),
    }
}

define_bench_ro_wrappers!(
    handle_davis_probe_with_transport,
    handle_davis_probe,
    handle_davis_probe_with_policy
);
