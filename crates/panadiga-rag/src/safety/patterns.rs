//! Compiled regex patterns, UTF-8 context slicing, and voltage extraction for hardware safety validation.

use regex::Regex;
use std::sync::LazyLock;

pub(super) static HEX_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b0x[0-9A-Fa-f]+\b").expect("Invalid regex: HEX_PATTERN"));

pub(super) static IPL_ERASE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:erase|erasing|wipe|wiping|clear|clearing|destroy|overwrite|program|write|writing)\s+(?:the\s+)?(?:flash\s+)?(?:sectors?\s+)?(?:sa0|sa133|ipl)\b")
        .expect("Invalid regex: IPL_ERASE_PATTERN")
});

pub(super) static WRITE_ACTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:erase|erasing|wipe|wiping|write|writing|overwrite|overwriting|program|programming|reflash|flashing|clear|clearing|destroy|destroying|modify|modifying|patch|patching)\b|(?i)\bflash\s+(?:a\s+|new\s+|custom\s+|the\s+)?(?:image|binary|firmware|bootloader|rom|data|bytes)?\s*(?:to|at|into)\b")
        .expect("Invalid regex: WRITE_ACTION_REGEX")
});

pub(super) static NEGATION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:never|not|no|do\s+not|must\s+not|cannot|prohibited|forbidden|avoid|warning|danger|destroy|damage|destruction|fatal|burn|bricks?|hazard|violation|exceeds|blocks|preserve|preserving|immutable)\b")
        .expect("Invalid regex: NEGATION_REGEX")
});

static MV_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[^\w])(\d+(?:[.,]\d+)?)\s*m(?:vdc|vac|v|volts?)(?:_[a-z0-9_]+|\b)")
        .expect("Invalid regex: MV_PATTERN")
});

static RKM_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d+)[vV](\d+)(?:_[a-zA-Z0-9_]+|\b)").expect("Invalid regex: RKM_PATTERN")
});

static STD_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d+(?:[.,]\d+)?)\s*(?:vdc|vac|v|volts?)(?:_[a-zA-Z0-9_]+|\b)")
        .expect("Invalid regex: STD_PATTERN")
});

fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn ceil_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

/// Returns a UTF-8 character-boundary-safe slice of `lower_text` around `[match_start, match_end]`.
pub(super) fn sentence_context_window(
    lower_text: &str,
    match_start: usize,
    match_end: usize,
    radius: usize,
) -> &str {
    let safe_start = floor_char_boundary(lower_text, match_start);
    let safe_end = ceil_char_boundary(lower_text, match_end).max(safe_start);

    let mut start_ctx = floor_char_boundary(lower_text, safe_start.saturating_sub(radius));
    for p in ['.', ';', '!', '\n'] {
        if let Some(idx) = lower_text[start_ctx..safe_start].rfind(p) {
            start_ctx = start_ctx.max(ceil_char_boundary(
                lower_text,
                start_ctx + idx + p.len_utf8(),
            ));
        }
    }

    let mut end_ctx = ceil_char_boundary(lower_text, (safe_end + radius).min(lower_text.len()));
    for p in ['.', ';', '!', '\n'] {
        if let Some(idx) = lower_text[safe_end..end_ctx].find(p) {
            end_ctx = end_ctx.min(floor_char_boundary(lower_text, safe_end + idx));
        }
    }

    &lower_text[start_ctx..end_ctx]
}

/// Extracts all voltage references from text and normalizes them to Volts (f64).
pub fn extract_voltages_volts(text: &str) -> Vec<(f64, usize, usize, String)> {
    let mut results = Vec::new();

    for cap in MV_PATTERN.captures_iter(text) {
        if let Some(m) = cap.get(0)
            && let Some(num_match) = cap.get(1)
        {
            let num_str = num_match.as_str().replace(',', ".");
            if let Ok(num) = num_str.parse::<f64>() {
                results.push((
                    num / 1000.0,
                    m.start(),
                    m.end(),
                    m.as_str().trim().to_string(),
                ));
            }
        }
    }

    for cap in RKM_PATTERN.captures_iter(text) {
        if let Some(m) = cap.get(0)
            && let (Some(major), Some(minor)) = (cap.get(1), cap.get(2))
        {
            let formatted = format!("{}.{}", major.as_str(), minor.as_str());
            if let Ok(val) = formatted.parse::<f64>() {
                results.push((val, m.start(), m.end(), m.as_str().trim().to_string()));
            }
        }
    }

    for cap in STD_PATTERN.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            let start = m.start();
            let end = m.end();
            if results.iter().any(|(_, s, e, _)| *s < end && *e > start) {
                continue;
            }
            if let Some(num_match) = cap.get(1) {
                let num_str = num_match.as_str().replace(',', ".");
                if let Ok(val) = num_str.parse::<f64>() {
                    results.push((val, start, end, m.as_str().trim().to_string()));
                }
            }
        }
    }

    results
}
