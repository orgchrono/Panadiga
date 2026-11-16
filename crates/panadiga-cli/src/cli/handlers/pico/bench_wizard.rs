//! CLI runner for `panadiga pico bench-wizard` and `panadiga pico error-lookup`.

use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use panadiga_core::pico::cdc::open_cdc;
use panadiga_core::pico::{
    GuidedBenchReport, VfdObservation, evaluate_guided_bench, execute_baseline_safety_step,
    execute_baudscan_step, execute_wake_trace_step, format_guided_bench_markdown,
    list_pico_devices, write_guided_bench_reports,
};
use panadiga_core::re::error_dict::{all_firmware_errors, lookup_firmware_errors};
use panadiga_core::{log_info, log_println};

fn prompt_line(msg: &str) -> Result<String> {
    print!("{msg}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

fn parse_vfd_choice(raw: &str) -> VfdObservation {
    match raw.trim() {
        "1" => VfdObservation::StaysStandbyClock,
        "2" => VfdObservation::HelloPleaseWaitU99Under2s,
        "3" => VfdObservation::ImmediateU99,
        "4" => VfdObservation::U99AfterLongWait,
        "5" => VfdObservation::ReadyNormal,
        "6" => VfdObservation::PowerOffBye,
        "7" => VfdObservation::BlankDead,
        "" => VfdObservation::HelloPleaseWaitU99Under2s,
        other => {
            let up = other.to_ascii_uppercase();
            if up.contains("HELLO") && up.contains("U99") {
                VfdObservation::HelloPleaseWaitU99Under2s
            } else if up.contains("READY") {
                VfdObservation::ReadyNormal
            } else if up.contains("0:00") || up.contains("STANDBY") {
                VfdObservation::StaysStandbyClock
            } else {
                VfdObservation::Custom(other.to_string())
            }
        }
    }
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

pub(super) fn run_bench_wizard(
    port_opt: Option<String>,
    baud: u32,
    window_ms: u32,
    non_interactive: bool,
    vfd_cue: Option<String>,
    active_jig: bool,
    out_dir: Option<PathBuf>,
) -> Result<()> {
    let port_name = resolve_target_port(port_opt)?;
    log_println!("====================================================================");
    log_println!("   PANADIGA: BATERIA GUIADA DE TESTES P7501 & DICIONARIO DE ERROS");
    log_println!(
        "   Porta Sonda : {} @ {} bps | Dicionario: {} entradas",
        port_name,
        baud,
        all_firmware_errors().len()
    );
    log_println!("   Invariante  : NSW5V(GP3)=Hi-Z | XRESET=Open-Drain | Zero escrita IPL");
    log_println!("====================================================================");

    let mut ser = open_cdc(&port_name, baud, Duration::from_millis(1200))?;
    let mut steps = Vec::new();

    // Etapa 1: Baseline de Standby e Seguranca
    let (initial_vdd, base_ev) = execute_baseline_safety_step(&mut *ser);
    log_println!("[ETAPA 1] {}", base_ev.summary);
    steps.push(base_ev);

    // Etapa 2: Acordar do Standby & Captura Synchronized BOOTTRACE
    let wake_note = if non_interactive {
        "Non-interactive passive wakeup capture".to_string()
    } else {
        log_println!();
        log_println!("[ETAPA 2 — ACORDAR DO STANDBY & CAPTURA DE BOOT]");
        log_println!("  Prepare-se para pressionar [POWER] no painel frontal ou controle remoto");
        log_println!(
            "  IMEDIATAMENTE apos apertar ENTER aqui (janela de {} ms).",
            window_ms
        );
        let _ = prompt_line(
            "  Pressione ENTER para armar o Pico e em seguida aperte [POWER] no aparelho > ",
        )?;
        log_println!(
            "  >> Pico ARMADO (BOOTTRACE {} PASSIVE)... Aperte [POWER] AGORA!",
            window_ms
        );
        prompt_line("  (Observacao opcional apos finalizar, ou ENTER): ")
            .unwrap_or_else(|_| "Standby wakeup triggered by operator".to_string())
    };
    let (wake, wake_ev) = execute_wake_trace_step(&mut *ser, window_ms, false, &wake_note);
    log_println!("[ETAPA 2 RESULT] {}", wake_ev.summary);
    steps.push(wake_ev);

    let vfd_obs = if let Some(cue) = vfd_cue {
        parse_vfd_choice(&cue)
    } else if non_interactive {
        VfdObservation::StaysStandbyClock
    } else {
        log_println!();
        log_println!("  O que apareceu no visor VFD frontal durante/apos a janela?");
        log_println!("   1) Ficou em 0:00 (Standby)");
        log_println!("   2) HELLO -> PLEASE WAIT (~2s) -> U99");
        log_println!("   3) U99 imediato (<2s, sem HELLO)");
        log_println!("   4) PLEASE WAIT longo (>10s) -> U99");
        log_println!("   5) Ligou normal (Ready / HDD / DVD)");
        log_println!("   6) BYE -> voltou ao Standby");
        log_println!("   7) Visor completamente apagado");
        log_println!("   8) Outro texto/codigo (digite livremente ex.: U81, F58)");
        let ans = prompt_line("  Escolha [1-8 ou texto]: ")?;
        parse_vfd_choice(&ans)
    };

    // Etapa 3: Verificacao de Baud Cluster + Framing PIO1
    let scan_ms = window_ms.clamp(1500, 6000);
    if !non_interactive {
        log_println!();
        log_println!("[ETAPA 3 — DETECCAO DE BAUD & FRAMING ({} ms)]", scan_ms);
        let _ = prompt_line(
            "  Aperte ENTER para iniciar BAUDSCAN dual-polarity (se precisar, gere trafego serial agora) > ",
        )?;
    }
    let (baud_res, baud_ev) = execute_baudscan_step(
        &mut *ser,
        scan_ms,
        true,
        "Dual-polarity pulse + PIO IRQ4 framing audit",
    );
    log_println!("[ETAPA 3 RESULT] {}", baud_ev.summary);
    steps.push(baud_ev);

    // Etapa 4: Sonda Ativa SW_UART (JIG) + XRESET Open-Drain (Somente Opt-In!)
    let run_jig = if non_interactive {
        active_jig
    } else {
        log_println!();
        let ans = prompt_line(
            "[ETAPA 4 OPCIONAL] Deseja pulsar XRESET Open-Drain com SW_UART=LOW (JIG serial boot mode)? [s/N]: ",
        )?;
        matches!(
            ans.trim().to_ascii_lowercase().as_str(),
            "s" | "sim" | "y" | "yes"
        )
    };
    if run_jig {
        log_println!("  >> Executando BOOTTRACE {} JIG...", window_ms);
        let (_jwake, jig_ev) = execute_wake_trace_step(
            &mut *ser,
            window_ms,
            true,
            "Active SW_UART LOW + Open-Drain XRESET boot probe",
        );
        log_println!("[ETAPA 4 RESULT] {}", jig_ev.summary);
        steps.push(jig_ev);
    }

    let report: GuidedBenchReport = evaluate_guided_bench(
        &port_name,
        initial_vdd,
        Some(wake),
        Some(baud_res),
        vfd_obs,
        steps,
    );

    let dest_dir = out_dir.unwrap_or_else(|| PathBuf::from("docs/debug_logs/bench_reports"));
    let (md_path, json_path) = write_guided_bench_reports(&dest_dir, &report)?;

    log_println!();
    log_println!("{}", format_guided_bench_markdown(&report));
    log_info!(
        "[OK] Relatorios gravados: {} | {}",
        md_path.display(),
        json_path.display()
    );
    Ok(())
}

pub(super) fn run_error_lookup(query: &str, json_out: Option<PathBuf>) -> Result<()> {
    let hits = lookup_firmware_errors(query);
    if let Some(out) = json_out {
        super::super::json_io::emit_json_output(&hits, Some(out.as_path()))?;
        return Ok(());
    }
    log_println!("====================================================================");
    log_println!(
        "   PANADIGA FIRMWARE ERROR DICTIONARY ({} registros) | Busca: '{}'",
        all_firmware_errors().len(),
        query
    );
    log_println!("====================================================================");
    if hits.is_empty() {
        log_println!("Nenhum registro encontrado para '{}'.", query);
        return Ok(());
    }
    for (idx, h) in hits.iter().take(25).enumerate() {
        log_println!(
            "{}. [{}] {} ({:?} @ {})",
            idx + 1,
            h.code,
            h.title,
            h.domain,
            h.address
        );
        log_println!("   Original   : {}", h.native_text);
        log_println!("   Traducao   : {}", h.translation);
        for act in &h.actions {
            log_println!("   -> Acao    : {}", act);
        }
    }
    if hits.len() > 25 {
        log_println!(
            "... ({} mais resultados; refine a busca ou use --json -)",
            hits.len() - 25
        );
    }
    Ok(())
}
