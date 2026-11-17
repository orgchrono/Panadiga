//! CLI runner and report exporter for interactive multi-boot `panadiga pico bench-wizard`.

mod battery;
mod prompts;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use panadiga_core::pico::cdc::open_cdc;
use panadiga_core::pico::{
    GuidedBenchReport, evaluate_guided_bench, execute_baseline_safety_step,
    format_guided_bench_markdown, list_pico_devices, write_guided_bench_reports,
};
use panadiga_core::re::error_dict::all_firmware_errors;
use panadiga_core::{log_info, log_println};

use battery::{run_interactive_battery, run_non_interactive_battery, run_single_profile_battery};

/// Configuration options for `panadiga pico bench-wizard`.
pub struct BenchWizardConfig {
    pub port_opt: Option<String>,
    pub baud: u32,
    pub window_ms: u32,
    pub profile: Option<String>,
    pub wake_timeout_ms: Option<u32>,
    pub jig_hold_ms: u32,
    pub non_interactive: bool,
    pub vfd_cue: Option<String>,
    pub active_jig: bool,
    pub out_dir: Option<PathBuf>,
}

fn resolve_target_port(explicit: Option<String>) -> Result<String> {
    if let Some(p) = explicit.filter(|s| !s.trim().is_empty()) {
        return Ok(p);
    }
    let devices = list_pico_devices()?;
    if let Some(pico) = devices
        .iter()
        .find(|d| d.vid == 0x2E8A || d.description.to_ascii_uppercase().contains("PICO"))
        .or_else(|| devices.first())
    {
        return Ok(pico.port_name.clone());
    }
    anyhow::bail!("Nenhum dispositivo Pico USB-CDC detectado; informe --port COMx explicitamente.")
}

fn export_bench_reports(out_dir: Option<PathBuf>, report: &GuidedBenchReport) -> Result<()> {
    let dest_dir = out_dir.unwrap_or_else(|| PathBuf::from("docs/debug_logs/bench_reports"));
    let (md_path, json_path) = write_guided_bench_reports(&dest_dir, report)?;
    log_println!();
    log_println!("{}", format_guided_bench_markdown(report));
    log_info!(
        "[OK] Relatorios gravados: {} | {}",
        md_path.display(),
        json_path.display()
    );
    Ok(())
}

/// Runs the guided multi-boot P7501 bench battery and writes correlated Markdown + JSON reports.
pub fn handle_bench_wizard(cfg: BenchWizardConfig) -> Result<()> {
    let port_name = resolve_target_port(cfg.port_opt.clone())?;
    let wake_ms = cfg
        .wake_timeout_ms
        .unwrap_or(if cfg.non_interactive { 0 } else { 45000 });

    log_println!("====================================================================");
    log_println!("   PANADIGA: BATERIA MULTI-BOOT P7501 & DICIONARIO DE ERROS");
    log_println!(
        "   Porta Sonda : {} @ {} bps | Dicionario: {} registros",
        port_name,
        cfg.baud,
        all_firmware_errors().len()
    );
    log_println!("   Invariante  : NSW5V(GP3)=Hi-Z | XRESET=Open-Drain | Zero escrita IPL");
    log_println!("====================================================================");

    let mut ser = open_cdc(&port_name, cfg.baud, Duration::from_millis(1200))?;
    let mut steps = Vec::new();

    // Etapa 1: Baseline de Standby e Seguranca (sem alterar estado dos pinos)
    let (initial_vdd, base_ev) = execute_baseline_safety_step(&mut *ser);
    log_println!("[ETAPA 1] {}", base_ev.summary);
    steps.push(base_ev);

    let (wake_opt, baud_opt, vfd_obs) = if cfg.non_interactive {
        run_non_interactive_battery(&mut *ser, &cfg, wake_ms, &mut steps)?
    } else if let Some(explicit_prof) = cfg
        .profile
        .as_deref()
        .filter(|p| !p.eq_ignore_ascii_case("all"))
    {
        run_single_profile_battery(&mut *ser, &cfg, explicit_prof, wake_ms, &mut steps)?
    } else {
        run_interactive_battery(&mut *ser, &cfg, wake_ms, &mut steps)?
    };

    let report = evaluate_guided_bench(&port_name, initial_vdd, wake_opt, baud_opt, vfd_obs, steps);
    export_bench_reports(cfg.out_dir, &report)
}
