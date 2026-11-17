//! Interactive VFD prompts, profile parser, and operator confirmation helpers for `bench-wizard`.

use std::io::{self, Write};

use anyhow::Result;
use panadiga_core::log_println;
use panadiga_core::pico::{BootWakeProfile, VfdObservation};

pub(super) fn prompt_line(msg: &str) -> Result<String> {
    print!("{msg}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

pub(super) fn prompt_yes_no(msg: &str) -> Result<bool> {
    let ans = prompt_line(msg)?;
    Ok(matches!(
        ans.trim().to_ascii_lowercase().as_str(),
        "s" | "sim" | "y" | "yes"
    ))
}

/// Parses an operator VFD cue (`1..9`, empty ENTER default -> `PleaseWaitThenU99Under2s`, or free text).
pub(super) fn parse_vfd_choice(raw: &str) -> VfdObservation {
    match raw.trim() {
        "1" => VfdObservation::StaysStandbyClock,
        "2" | "" => VfdObservation::PleaseWaitThenU99Under2s,
        "3" => VfdObservation::HelloThenU99Under2s,
        "4" => VfdObservation::ImmediateU99,
        "5" => VfdObservation::U99AfterLongWait,
        "6" => VfdObservation::ReadyNormal,
        "7" => VfdObservation::PowerOffBye,
        "8" => VfdObservation::BlankDead,
        "9" => VfdObservation::Custom("CUSTOM".to_string()),
        other => parse_vfd_free_text(other),
    }
}

fn parse_vfd_free_text(other: &str) -> VfdObservation {
    let up = other.to_ascii_uppercase();
    if up.contains("HELLO") && up.contains("U99") {
        VfdObservation::HelloThenU99Under2s
    } else if (up.contains("PLEASE WAIT") || up.contains("PLEASEWAIT")) && up.contains("U99") {
        if up.contains(">10") || up.contains("LONG") {
            VfdObservation::U99AfterLongWait
        } else {
            VfdObservation::PleaseWaitThenU99Under2s
        }
    } else if up.contains("DIRECT U99")
        || up.contains("IMMEDIATE U99")
        || up.contains("U99 IMEDIATO")
    {
        VfdObservation::ImmediateU99
    } else if up.contains("READY") {
        VfdObservation::ReadyNormal
    } else if up.contains("BYE") {
        VfdObservation::PowerOffBye
    } else if up.contains("BLANK") || up.contains("APAGADO") {
        VfdObservation::BlankDead
    } else if up.contains("0:00") || up.contains("--:--") || up.contains("STANDBY") {
        VfdObservation::StaysStandbyClock
    } else {
        VfdObservation::Custom(other.trim().to_string())
    }
}

/// Prompts the operator to record what the front-panel VFD displayed during capture (`1..9`).
pub(super) fn prompt_vfd_observation() -> Result<VfdObservation> {
    log_println!();
    log_println!("  O que apareceu no visor VFD frontal durante/apos a captura?");
    log_println!("   1) Ficou em 0:00 / --:-- (Standby)");
    log_println!("   2) PLEASE WAIT (~2s) -> U99 (loop pos-AC / auto-restart) [padrao ENTER]");
    log_println!("   3) HELLO (~2s) -> U99 (acordou do Standby estabilizado -> U99)");
    log_println!("   4) U99 imediato (<2s, sem HELLO / sem PLEASE WAIT)");
    log_println!("   5) PLEASE WAIT longo (>10s) -> U99");
    log_println!("   6) Ligou normal (Ready / HDD / DVD / SERV)");
    log_println!("   7) BYE -> voltou ao Standby");
    log_println!("   8) Visor completamente apagado");
    log_println!(
        "   9) Outro texto/codigo (digite livremente ex.: U61, U81, F58, HDD ERR, SELF CHECK)"
    );
    let ans = prompt_line("  Escolha [1-9 ou texto; ENTER=2]: ")?;
    if ans.trim() == "9" {
        let custom = prompt_line("  Digite o texto/codigo exibido no VFD: ")?;
        let trimmed = custom.trim();
        return Ok(if trimmed.is_empty() {
            VfdObservation::Custom("CUSTOM".to_string())
        } else {
            VfdObservation::Custom(trimmed.to_string())
        });
    }
    Ok(parse_vfd_choice(&ans))
}

pub(super) fn parse_single_profile(raw: &str) -> BootWakeProfile {
    match raw.trim().to_ascii_lowercase().as_str() {
        "prejig" | "standby-jig" | "serial" => BootWakeProfile::PreJigWake,
        "diagwatch" | "watch" => BootWakeProfile::DiagWatchPreJigWake,
        "jig" | "active-jig" | "xreset" => BootWakeProfile::ActiveJigXreset,
        _ => BootWakeProfile::PassiveWake,
    }
}

pub(super) fn confirm_arm_wake(
    profile: BootWakeProfile,
    cmd_preview: &str,
    wake_timeout_ms: u32,
) -> Result<()> {
    log_println!();
    log_println!("  Preparando perfil: {}", profile.title());
    log_println!("  Comando Pico     : `{}`", cmd_preview);
    if wake_timeout_ms > 0 {
        log_println!(
            "  >> Ao apertar ENTER aqui, o Pico ficara ARMADO por {} ms esperando o wake (NSW5V subir).",
            wake_timeout_ms
        );
        let _ = prompt_line(
            "  Com o aparelho em Standby (ou pronto), aperte ENTER para ARMAR AGORA > ",
        )?;
        log_println!(
            "  >> [ARMADO] Execute agora a acao no aparelho ([POWER], acorde IR ou combo frontal)!"
        );
    }
    Ok(())
}

pub(super) fn prompt_battery_choice() -> Result<String> {
    log_println!();
    log_println!("--------------------------------------------------------------------");
    log_println!("  MENU MULTI-BOOT A PARTIR DO STANDBY (escolha o proximo ensaio):");
    log_println!("   1) Boot Normal Passivo (`PASSIVE WAKE` — Hi-Z; use [POWER] ou Controle IR)");
    log_println!(
        "   2) Boot Serial PREJIG sem pulso XRESET (`PREJIG WAKE` — segura Pin 1 LOW em Standby)"
    );
    log_println!(
        "   3) Interceptacao Autonoma DIAGWATCH (`DIAGWATCH PREJIG WAKE` — PANAMON1>/SDDIAG)"
    );
    log_println!(
        "   4) Boot Combo Painel Frontal (`[REC]+[CH+]+[OPEN]` 5s SERV ou `[STOP]+[CH+]+[OPEN]` TEST)"
    );
    log_println!("   5) Probe Ativa JIG + XRESET Open-Drain (`BOOTTRACE JIG` imediato)");
    log_println!("   6) Medicao isolada de Baudrate Dual-Polarity (`BAUDSCAN APPLY WAKE`)");
    log_println!("   F) Finalizar ensaios e gerar relatorio correlacionado (MD + JSON)");
    let choice = prompt_line("  Opcao [1-6 ou F]: ")?;
    Ok(choice.trim().to_ascii_uppercase())
}

pub(super) fn prompt_front_panel_chord() -> Result<String> {
    let chord = prompt_line(
        "  Qual sequencia vai segurar por 5s? [SERV (REC+CH+OPEN) / TEST (STOP+CH+OPEN)]: ",
    )?;
    let label = if chord.is_empty() {
        "SERV/TEST"
    } else {
        chord.as_str()
    };
    Ok(format!("Boot 4: Front-panel chord ({label})"))
}

pub(super) fn prompt_optional_note() -> String {
    prompt_line("  Observacao opcional deste boot (ENTER para pular): ").unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_nine_vfd_options_and_text_cues() {
        assert_eq!(parse_vfd_choice("1"), VfdObservation::StaysStandbyClock);
        assert_eq!(
            parse_vfd_choice("2"),
            VfdObservation::PleaseWaitThenU99Under2s
        );
        assert_eq!(
            parse_vfd_choice(""),
            VfdObservation::PleaseWaitThenU99Under2s
        );
        assert_eq!(
            parse_vfd_choice("   "),
            VfdObservation::PleaseWaitThenU99Under2s
        );
        assert_eq!(parse_vfd_choice("3"), VfdObservation::HelloThenU99Under2s);
        assert_eq!(parse_vfd_choice("4"), VfdObservation::ImmediateU99);
        assert_eq!(parse_vfd_choice("5"), VfdObservation::U99AfterLongWait);
        assert_eq!(parse_vfd_choice("6"), VfdObservation::ReadyNormal);
        assert_eq!(parse_vfd_choice("7"), VfdObservation::PowerOffBye);
        assert_eq!(parse_vfd_choice("8"), VfdObservation::BlankDead);
        assert_eq!(
            parse_vfd_choice("9"),
            VfdObservation::Custom("CUSTOM".to_string())
        );

        assert_eq!(
            parse_vfd_choice("HELLO (~2s) -> U99"),
            VfdObservation::HelloThenU99Under2s
        );
        assert_eq!(
            parse_vfd_choice("PLEASE WAIT (~2s) -> U99"),
            VfdObservation::PleaseWaitThenU99Under2s
        );
        assert_eq!(
            parse_vfd_choice("PLEASE WAIT (>10s) -> U99"),
            VfdObservation::U99AfterLongWait
        );
        assert_eq!(parse_vfd_choice("READY"), VfdObservation::ReadyNormal);
        assert_eq!(
            parse_vfd_choice("0:00 STANDBY"),
            VfdObservation::StaysStandbyClock
        );
        assert_eq!(
            parse_vfd_choice("U61"),
            VfdObservation::Custom("U61".to_string())
        );
    }

    #[test]
    fn parses_boot_wake_profile_aliases() {
        assert_eq!(parse_single_profile("prejig"), BootWakeProfile::PreJigWake);
        assert_eq!(
            parse_single_profile("diagwatch"),
            BootWakeProfile::DiagWatchPreJigWake
        );
        assert_eq!(
            parse_single_profile("jig"),
            BootWakeProfile::ActiveJigXreset
        );
        assert_eq!(
            parse_single_profile("passive"),
            BootWakeProfile::PassiveWake
        );
    }
}
