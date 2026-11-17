//! Interactive, single-profile, and non-interactive bench test battery runners.

use anyhow::Result;
use panadiga_core::log_println;
use panadiga_core::pico::{
    BaudScanSummary, BootWakeProfile, GuidedStepEvidence, VfdObservation, WakeTraceSummary,
    execute_baudscan_step_with_wake, execute_wake_trace_step_profile,
};
use serialport::SerialPort;

use super::BenchWizardConfig;
use super::prompts::{
    confirm_arm_wake, parse_single_profile, parse_vfd_choice, prompt_battery_choice,
    prompt_front_panel_chord, prompt_line, prompt_optional_note, prompt_vfd_observation,
    prompt_yes_no,
};

pub(super) type BatteryOutcome = (
    Option<WakeTraceSummary>,
    Option<BaudScanSummary>,
    VfdObservation,
);

/// Executes a single wake/boot trace capture, optionally confirming arming and reading the VFD cue.
pub(super) fn run_wake_stage_with_prompt(
    ser: &mut dyn SerialPort,
    profile: BootWakeProfile,
    window_ms: u32,
    jig_hold_ms: u32,
    wake_timeout_ms: u32,
    note_label: &str,
    interactive: bool,
) -> Result<(WakeTraceSummary, GuidedStepEvidence, Option<VfdObservation>)> {
    let cmd_preview = profile.build_pico_command(window_ms, jig_hold_ms, wake_timeout_ms);
    if interactive {
        confirm_arm_wake(profile, &cmd_preview, wake_timeout_ms)?;
    }

    let (wake, mut ev) = execute_wake_trace_step_profile(
        ser,
        profile,
        window_ms,
        jig_hold_ms,
        wake_timeout_ms,
        note_label,
    );
    log_println!("[RESULTADO] {}", ev.summary);
    if !ev.raw_output.trim().is_empty() {
        log_println!("{}", ev.raw_output);
    }

    let vfd_obs = if interactive {
        let obs = prompt_vfd_observation()?;
        let extra = prompt_optional_note();
        if !extra.is_empty() {
            ev.operator_note = format!("{} | {}", ev.operator_note, extra);
        }
        Some(obs)
    } else {
        None
    };

    Ok((wake, ev, vfd_obs))
}

fn run_baudscan_stage(
    ser: &mut dyn SerialPort,
    scan_ms: u32,
    wake_ms: u32,
    note: &str,
    log_tag: &str,
    steps: &mut Vec<GuidedStepEvidence>,
) -> BaudScanSummary {
    let (baud_res, baud_ev) = execute_baudscan_step_with_wake(ser, scan_ms, true, wake_ms, note);
    log_println!("[{}] {}", log_tag, baud_ev.summary);
    steps.push(baud_ev);
    baud_res
}

pub(super) fn run_non_interactive_battery(
    ser: &mut dyn SerialPort,
    cfg: &BenchWizardConfig,
    wake_ms: u32,
    steps: &mut Vec<GuidedStepEvidence>,
) -> Result<BatteryOutcome> {
    let profile = cfg
        .profile
        .as_deref()
        .map(parse_single_profile)
        .unwrap_or(if cfg.active_jig {
            BootWakeProfile::ActiveJigXreset
        } else {
            BootWakeProfile::PassiveWake
        });
    let (wake, ev, _) = run_wake_stage_with_prompt(
        ser,
        profile,
        cfg.window_ms,
        cfg.jig_hold_ms,
        wake_ms,
        "Non-interactive P7501 bench capture",
        false,
    )?;
    steps.push(ev);

    let scan_ms = cfg.window_ms.clamp(1500, 6000);
    let baud_res = run_baudscan_stage(
        ser,
        scan_ms,
        0,
        "Dual-polarity cluster + PIO IRQ4 framing audit",
        "ETAPA BAUDSCAN",
        steps,
    );
    let obs = cfg
        .vfd_cue
        .as_deref()
        .map(parse_vfd_choice)
        .unwrap_or(VfdObservation::StaysStandbyClock);
    Ok((Some(wake), Some(baud_res), obs))
}

pub(super) fn run_single_profile_battery(
    ser: &mut dyn SerialPort,
    cfg: &BenchWizardConfig,
    explicit_prof: &str,
    wake_ms: u32,
    steps: &mut Vec<GuidedStepEvidence>,
) -> Result<BatteryOutcome> {
    let prof = parse_single_profile(explicit_prof);
    let (wake, ev, vfd) = run_wake_stage_with_prompt(
        ser,
        prof,
        cfg.window_ms,
        cfg.jig_hold_ms,
        wake_ms,
        "Single-profile interactive standby capture",
        true,
    )?;
    steps.push(ev);
    Ok((
        Some(wake),
        None,
        vfd.unwrap_or(VfdObservation::StaysStandbyClock),
    ))
}

pub(super) fn run_interactive_battery(
    ser: &mut dyn SerialPort,
    cfg: &BenchWizardConfig,
    wake_ms: u32,
    steps: &mut Vec<GuidedStepEvidence>,
) -> Result<BatteryOutcome> {
    let mut latest_wake: Option<WakeTraceSummary> = None;
    let mut latest_baud: Option<BaudScanSummary> = None;
    let mut latest_vfd = VfdObservation::StaysStandbyClock;
    let scan_ms = cfg.window_ms.clamp(1500, 6000);

    loop {
        let choice = prompt_battery_choice()?;
        if matches!(choice.as_str(), "F" | "Q" | "") {
            break;
        }
        if choice == "6" {
            let _ = prompt_line(
                "  Aperte ENTER para armar `BAUDSCAN PASSIVE APPLY WAKE` (em seguida acorde o aparelho se estiver em Standby) > ",
            )?;
            latest_baud = Some(run_baudscan_stage(
                ser,
                scan_ms,
                wake_ms,
                "Standalone Standby-Wake Dual-Polarity BAUDSCAN",
                "BAUDSCAN RESULT",
                steps,
            ));
            continue;
        }

        let spec = match choice.as_str() {
            "1" => Some((
                BootWakeProfile::PassiveWake,
                cfg.window_ms,
                wake_ms,
                "Boot 1: Normal / IR wakeup (SW_UART Hi-Z)".to_string(),
            )),
            "2" => Some((
                BootWakeProfile::PreJigWake,
                cfg.window_ms,
                wake_ms,
                "Boot 2: Standby PREJIG serial boot (SW_UART=LOW sem XRESET)".to_string(),
            )),
            "3" => Some((
                BootWakeProfile::DiagWatchPreJigWake,
                cfg.window_ms.max(15000),
                wake_ms,
                "Boot 3: DIAGWATCH PREJIG WAKE (PANAMON1 / SDDIAG / BGO)".to_string(),
            )),
            "4" => Some((
                BootWakeProfile::PassiveWake,
                cfg.window_ms,
                wake_ms,
                prompt_front_panel_chord()?,
            )),
            "5" => Some((
                BootWakeProfile::ActiveJigXreset,
                cfg.window_ms,
                0,
                "Boot 5: Active SW_UART LOW + XRESET Open-Drain pulse".to_string(),
            )),
            _ => None,
        };

        let Some((profile, window_ms, stage_wake_ms, note)) = spec else {
            log_println!("  Opcao invalida; escolha 1..6 ou F.");
            continue;
        };

        let (wake, ev, vfd) = run_wake_stage_with_prompt(
            ser,
            profile,
            window_ms,
            cfg.jig_hold_ms,
            stage_wake_ms,
            &note,
            true,
        )?;
        steps.push(ev);
        if let Some(obs) = vfd {
            latest_vfd = obs;
        }
        latest_wake = Some(wake);
    }

    if latest_baud.is_none()
        && prompt_yes_no(
            "  Executar um `BAUDSCAN PASSIVE APPLY` rapido agora antes de fechar? [s/N]: ",
        )?
    {
        latest_baud = Some(run_baudscan_stage(
            ser,
            scan_ms,
            0,
            "Post-battery BAUDSCAN check",
            "BAUDSCAN RESULT",
            steps,
        ));
    }

    Ok((latest_wake, latest_baud, latest_vfd))
}
