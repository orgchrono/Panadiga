//! Classification & layout helpers for auto-assembled Panasonic dynamic remote buttons.

use super::types::{KeySection, KeyTone};

pub struct CommandClassification {
    pub section: KeySection,
    pub sort_key: u16,
    pub tone: KeyTone,
    pub confirm_required: bool,
}

/// Classifies a Panasonic IR button name into logical layout section, sort priority, and tone.
#[must_use]
pub fn classify_command_name(raw_name: &str) -> CommandClassification {
    let low = raw_name.trim().to_ascii_lowercase();
    let compact: String = low.chars().filter(char::is_ascii_alphanumeric).collect();

    if ["format", "reset", "erase"]
        .iter()
        .any(|kw| low.contains(kw))
        || matches!(compact.as_str(), "delete" | "dvderase")
    {
        return cls(KeySection::RecordingEdit, 90, KeyTone::DangerDark, true);
    }

    if let Some(digit_ch) = parse_numeric_button(&compact) {
        let ord = if digit_ch == '0' {
            10
        } else {
            u16::from(digit_ch as u8 - b'0')
        };
        return cls(KeySection::Numeric, ord, KeyTone::Neutral, false);
    }

    if compact.contains("power") || matches!(compact.as_str(), "standby" | "sleep" | "off" | "on") {
        let ord = if compact.contains("off") {
            2
        } else if compact.contains("on") {
            1
        } else {
            0
        };
        return cls(KeySection::PowerInput, ord, KeyTone::Danger, false);
    }
    if compact.contains("eject") || compact.contains("openclose") {
        return cls(KeySection::PowerInput, 10, KeyTone::Neutral, false);
    }
    if ["drive", "input", "vhs", "hdmi"]
        .iter()
        .any(|k| compact.contains(k))
        || compact.contains("vcrtv")
    {
        return cls(KeySection::PowerInput, 20, KeyTone::Primary, false);
    }
    if matches!(compact.as_str(), "ok" | "enter" | "select") {
        return cls(KeySection::Navigation, 0, KeyTone::Primary, false);
    }
    if let Some(nav) = classify_direction_or_tuning(&compact, &low) {
        return nav;
    }
    if matches!(
        compact.as_str(),
        "return" | "back" | "exit" | "cancel" | "clear"
    ) {
        return cls(KeySection::Navigation, 10, KeyTone::Neutral, false);
    }
    if [
        "directnav",
        "topmenu",
        "functions",
        "menu",
        "setup",
        "option",
        "guide",
    ]
    .iter()
    .any(|k| compact.contains(k))
    {
        return cls(KeySection::Navigation, 20, KeyTone::Info, false);
    }

    classify_transport_or_custom(&compact, &low)
}

fn classify_direction_or_tuning(compact: &str, low: &str) -> Option<CommandClassification> {
    let has_dir = ["up", "down", "left", "right"]
        .iter()
        .any(|d| compact.contains(d));
    if !has_dir {
        return None;
    }
    if ["vol", "ch", "track", "page"]
        .iter()
        .any(|k| compact.contains(k))
    {
        return Some(classify_non_dpad(compact, low));
    }
    let ord = if compact.contains("up") {
        1
    } else if compact.contains("down") {
        2
    } else if compact.contains("left") {
        3
    } else {
        4
    };
    Some(cls(KeySection::Navigation, ord, KeyTone::Neutral, false))
}

fn classify_transport_or_custom(compact: &str, low: &str) -> CommandClassification {
    if compact == "play" || compact.starts_with("dvdplay") || compact.starts_with("vcrplay") {
        return cls(KeySection::Transport, 0, KeyTone::Success, false);
    }
    if compact.contains("pause") {
        return cls(KeySection::Transport, 1, KeyTone::Neutral, false);
    }
    if compact.contains("stop") {
        return cls(KeySection::Transport, 2, KeyTone::DangerDark, false);
    }
    if compact.contains("rew") || compact.contains("fastback") {
        return cls(KeySection::Transport, 3, KeyTone::Neutral, false);
    }
    if compact.contains("fastf") || matches!(compact, "ff" | "fwd") {
        return cls(KeySection::Transport, 4, KeyTone::Neutral, false);
    }
    if compact.contains("prev") || compact.contains("skipb") || compact.contains("chapterprev") {
        return cls(KeySection::Transport, 5, KeyTone::Neutral, false);
    }
    if compact.contains("next") || compact.contains("skipf") || compact.contains("chapternext") {
        return cls(KeySection::Transport, 6, KeyTone::Neutral, false);
    }
    if compact.contains("rec")
        && !compact.contains("mode")
        && !compact.contains("check")
        && !compact.contains("list")
    {
        return cls(KeySection::Transport, 7, KeyTone::Danger, false);
    }
    classify_non_dpad(compact, low)
}

fn classify_non_dpad(compact: &str, low: &str) -> CommandClassification {
    if compact.contains("vol") || compact.contains("mute") {
        let ord = if compact.contains("mute") {
            2
        } else if compact.contains("up") {
            0
        } else {
            1
        };
        return cls(KeySection::TuningAudio, ord, KeyTone::Info, false);
    }
    if compact.contains("channel") || compact.starts_with("ch") {
        return cls(KeySection::TuningAudio, 10, KeyTone::Neutral, false);
    }
    if ["audio", "subtitle", "sap", "angle"]
        .iter()
        .any(|k| compact.contains(k))
    {
        return cls(KeySection::TuningAudio, 20, KeyTone::Neutral, false);
    }
    if compact.contains("track") {
        return cls(KeySection::RecordingEdit, 5, KeyTone::Primary, false);
    }
    if [
        "dub", "timeslip", "skip", "replay", "chapter", "schedule", "timer", "recmode", "frec",
        "flex", "extlink", "status", "display", "info",
    ]
    .iter()
    .any(|k| compact.contains(k))
    {
        return cls(KeySection::RecordingEdit, 10, KeyTone::Warning, false);
    }
    if matches!(compact, "red" | "green" | "yellow" | "blue") {
        return cls(KeySection::Custom, 0, KeyTone::Accent, false);
    }
    if ["text", "jdm", "internet", "viera", "netflix"]
        .iter()
        .any(|k| low.contains(k))
    {
        return cls(KeySection::Custom, 10, KeyTone::Accent, false);
    }
    cls(KeySection::Custom, 50, KeyTone::Neutral, false)
}

fn parse_numeric_button(compact: &str) -> Option<char> {
    if compact.len() == 1 && compact.chars().all(|c| c.is_ascii_digit()) {
        return compact.chars().next();
    }
    if let Some(rest) = compact.strip_prefix("digit")
        && !rest.is_empty()
        && rest.chars().all(|c| c.is_ascii_digit())
    {
        return rest.chars().last();
    }
    None
}

const fn cls(
    section: KeySection,
    sort_key: u16,
    tone: KeyTone,
    confirm_required: bool,
) -> CommandClassification {
    CommandClassification {
        section,
        sort_key,
        tone,
        confirm_required,
    }
}

/// Converts snake/CamelCase catalog command names into concise UI labels (max 28 chars).
#[must_use]
pub fn prettify_command_label(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 4);
    let mut prev_lower = false;
    for ch in raw.chars() {
        if ch == '_' {
            if !out.ends_with(' ') {
                out.push(' ');
            }
            prev_lower = false;
            continue;
        }
        if prev_lower && ch.is_ascii_uppercase() && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push(ch);
        prev_lower = ch.is_ascii_lowercase();
    }
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(28)
        .collect()
}
