#![deny(unsafe_code)]

//! Bilingual SSOT localization catalog (`pt-BR` / `en-US`) with fail-closed placeholder
//! interpolation, cross-locale parity checks, and anti-hardcoded-32-bit-address guards.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::ProfilesBundle;

/// Supported SSOT catalog locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum LocaleId {
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "en-US")]
    EnUs,
}

impl LocaleId {
    /// Canonical BCP-47 locale tag matching filenames under `profiles/locales/`.
    #[must_use]
    pub const fn as_tag(self) -> &'static str {
        match self {
            Self::PtBr => "pt-BR",
            Self::EnUs => "en-US",
        }
    }
}

/// Single localized template map loaded from `profiles/locales/<lang>.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocaleFile {
    pub schema_version: u32,
    pub locale: String,
    pub entries: BTreeMap<String, String>,
}

/// Paired bilingual catalog (`pt-BR` and `en-US`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LocaleCatalog {
    pub pt_br: LocaleFile,
    pub en_us: LocaleFile,
}

impl LocaleCatalog {
    /// Selects a localized map reference by [`LocaleId`].
    #[must_use]
    pub const fn file(&self, locale: LocaleId) -> &LocaleFile {
        match locale {
            LocaleId::PtBr => &self.pt_br,
            LocaleId::EnUs => &self.en_us,
        }
    }

    /// Looks up `key` in `locale` and interpolates all `{placeholder}` tokens fail-closed.
    pub fn render(
        &self,
        locale: LocaleId,
        key: &str,
        params: &BTreeMap<&str, String>,
    ) -> Result<String> {
        let map = &self.file(locale).entries;
        let template = map
            .get(key)
            .with_context(|| format!("Missing SSOT locale key '{key}' for {}", locale.as_tag()))?;
        interpolate_ssot_template(template, params)
    }
}

/// Single physical/geometry/decompiler trap record in `profiles/traps/geometry_and_binary_traps.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrapRecord {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub wrong_assumption: String,
    pub verified_ground_truth: String,
    pub affected_entities: Vec<String>,
    pub automated_guard: String,
    pub evidence_refs: Vec<String>,
}

/// Trap catalog deserialized from `profiles/traps/geometry_and_binary_traps.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrapCatalog {
    pub schema_version: u32,
    pub description: String,
    pub traps: Vec<TrapRecord>,
}

/// Actionable locale validation failure emitted by [`validate_locale_catalog`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFailure {
    pub rule: String,
    pub key: Option<String>,
    pub message: String,
}

impl fmt::Display for ValidationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.key {
            Some(k) => write!(f, "[{}] ({k}): {}", self.rule, self.message),
            None => write!(f, "[{}]: {}", self.rule, self.message),
        }
    }
}

const LOCALE_SCHEMA_VERSION: u32 = 1;
const TRAP_FIELDS: [&str; 4] = [
    "title",
    "false_premise",
    "physical_truth",
    "preventive_guard",
];
const TRAIN_REQUIRED_KEYS: [&str; 3] = [
    "train.loader_ver061_toc48_main57.notes",
    "train.loader_ver061_toc48_main57.sata_bridge_notes",
    "train.loader_ver061_toc48_main57.bgo_retarget_notes",
];

fn load_single_locale_file(root: &Path, expected: LocaleId) -> Result<LocaleFile> {
    let tag = expected.as_tag();
    let path = root.join("locales").join(format!("{tag}.json"));
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Reading SSOT locale file {}", path.display()))?;
    let parsed: LocaleFile = serde_json::from_str(&raw)
        .with_context(|| format!("Parsing SSOT locale file {}", path.display()))?;
    if parsed.schema_version != LOCALE_SCHEMA_VERSION {
        bail!(
            "Unsupported locale schema_version {} in {} (expected {LOCALE_SCHEMA_VERSION})",
            parsed.schema_version,
            path.display()
        );
    }
    if parsed.locale != tag {
        bail!(
            "Locale mismatch in {}: declared '{}' != expected '{tag}'",
            path.display(),
            parsed.locale
        );
    }
    Ok(parsed)
}

/// Loads `profiles/locales/pt-BR.json` and `profiles/locales/en-US.json` from `root`.
pub fn load_locales_from_dir(root: &Path) -> Result<LocaleCatalog> {
    Ok(LocaleCatalog {
        pt_br: load_single_locale_file(root, LocaleId::PtBr)?,
        en_us: load_single_locale_file(root, LocaleId::EnUs)?,
    })
}

/// Loads `profiles/traps/geometry_and_binary_traps.json` if present under `root`.
pub fn load_optional_traps_from_dir(root: &Path) -> Result<Option<TrapCatalog>> {
    let path = root.join("traps").join("geometry_and_binary_traps.json");
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Reading traps catalog {}", path.display()))?;
    let parsed: TrapCatalog = serde_json::from_str(&raw)
        .with_context(|| format!("Parsing traps catalog {}", path.display()))?;
    Ok(Some(parsed))
}

fn extract_placeholders(template: &str) -> std::result::Result<BTreeSet<String>, String> {
    let mut tokens = BTreeSet::new();
    let mut chars = template.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        match ch {
            '{' => {
                let start = idx + 1;
                let mut end = None;
                for (j, inner) in chars.by_ref() {
                    if inner == '}' {
                        end = Some(j);
                        break;
                    }
                    if inner == '{' {
                        return Err(format!("Nested '{{' at byte offset {j}"));
                    }
                }
                let Some(end_idx) = end else {
                    return Err(format!("Unclosed '{{' starting at byte offset {idx}"));
                };
                let name = &template[start..end_idx];
                if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    return Err(format!("Invalid placeholder token '{{{name}}}'"));
                }
                tokens.insert(name.to_string());
            }
            '}' => {
                return Err(format!("Unmatched '}}' at byte offset {idx}"));
            }
            _ => {}
        }
    }
    Ok(tokens)
}

fn find_hardcoded_32bit_hex_literal(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i + 2 <= bytes.len() {
        if bytes[i] == b'0' && matches!(bytes.get(i + 1), Some(b'x' | b'X')) {
            let prev_is_alnum =
                i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
            if !prev_is_alnum {
                let mut j = i + 2;
                let mut hex_digits = 0usize;
                while let Some(&b) = bytes.get(j) {
                    if b.is_ascii_hexdigit() {
                        hex_digits += 1;
                        j += 1;
                    } else if b == b'_'
                        && hex_digits > 0
                        && bytes.get(j + 1).is_some_and(u8::is_ascii_hexdigit)
                    {
                        j += 1;
                    } else {
                        break;
                    }
                }
                if hex_digits >= 8 {
                    return Some(text[i..j].to_string());
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    None
}

fn push_failure(
    out: &mut Vec<ValidationFailure>,
    rule: &str,
    key: Option<&str>,
    message: impl Into<String>,
) {
    out.push(ValidationFailure {
        rule: rule.to_string(),
        key: key.map(ToOwned::to_owned),
        message: message.into(),
    });
}

fn check_no_hardcoded_hex_for_locale(
    locale: LocaleId,
    file: &LocaleFile,
    failures: &mut Vec<ValidationFailure>,
) {
    for (k, v) in &file.entries {
        if let Some(lit) = find_hardcoded_32bit_hex_literal(v) {
            push_failure(
                failures,
                "locale_no_hardcoded_hex_address",
                Some(k),
                format!(
                    "{} template contains hardcoded 32-bit address literal '{lit}'; move to SSOT placeholder",
                    locale.as_tag()
                ),
            );
        }
    }
}

/// Validates key parity, placeholder parity, anti-hardcoded-32bit-hex guard, and trap/train coverage.
pub fn validate_locale_catalog(
    catalog: &LocaleCatalog,
    bundle: &ProfilesBundle,
) -> std::result::Result<(), Vec<ValidationFailure>> {
    let mut failures = Vec::new();

    for key in catalog.pt_br.entries.keys() {
        if !catalog.en_us.entries.contains_key(key) {
            push_failure(
                &mut failures,
                "locale_key_parity",
                Some(key),
                "Present in pt-BR.json but missing from en-US.json",
            );
        }
    }
    for key in catalog.en_us.entries.keys() {
        if !catalog.pt_br.entries.contains_key(key) {
            push_failure(
                &mut failures,
                "locale_key_parity",
                Some(key),
                "Present in en-US.json but missing from pt-BR.json",
            );
        }
    }

    for (key, pt_val) in &catalog.pt_br.entries {
        let pt_tokens = match extract_placeholders(pt_val) {
            Ok(toks) => Some(toks),
            Err(err) => {
                push_failure(
                    &mut failures,
                    "locale_placeholder_parity",
                    Some(key),
                    format!("pt-BR placeholder syntax error: {err}"),
                );
                None
            }
        };
        if let Some(en_val) = catalog.en_us.entries.get(key) {
            let en_tokens = match extract_placeholders(en_val) {
                Ok(toks) => Some(toks),
                Err(err) => {
                    push_failure(
                        &mut failures,
                        "locale_placeholder_parity",
                        Some(key),
                        format!("en-US placeholder syntax error: {err}"),
                    );
                    None
                }
            };
            if let (Some(pt), Some(en)) = (pt_tokens, en_tokens)
                && pt != en
            {
                push_failure(
                    &mut failures,
                    "locale_placeholder_parity",
                    Some(key),
                    format!("Placeholder mismatch: pt-BR={pt:?}, en-US={en:?}"),
                );
            }
        }
    }

    check_no_hardcoded_hex_for_locale(LocaleId::PtBr, &catalog.pt_br, &mut failures);
    check_no_hardcoded_hex_for_locale(LocaleId::EnUs, &catalog.en_us, &mut failures);

    if let Some(traps) = &bundle.traps {
        for trap in &traps.traps {
            for field in TRAP_FIELDS {
                let req_key = format!("trap.{}.{field}", trap.id);
                if !catalog.pt_br.entries.contains_key(&req_key)
                    || !catalog.en_us.entries.contains_key(&req_key)
                {
                    push_failure(
                        &mut failures,
                        "locale_trap_coverage",
                        Some(&req_key),
                        format!("Trap '{}' requires '{req_key}' in both locales", trap.id),
                    );
                }
            }
        }
    }

    if bundle
        .firmware_trains
        .contains_key("loader_ver061_toc48_main57")
    {
        for req_key in TRAIN_REQUIRED_KEYS {
            if !catalog.pt_br.entries.contains_key(req_key)
                || !catalog.en_us.entries.contains_key(req_key)
            {
                push_failure(
                    &mut failures,
                    "locale_trap_coverage",
                    Some(req_key),
                    format!(
                        "Train 'loader_ver061_toc48_main57' requires '{req_key}' in both locales"
                    ),
                );
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

/// Interpolates `{placeholder}` tokens into `template`, failing closed if any token is missing or invalid.
pub fn interpolate_ssot_template(
    template: &str,
    params: &BTreeMap<&str, String>,
) -> Result<String> {
    let _ = extract_placeholders(template)
        .map_err(|err| anyhow::anyhow!("Invalid SSOT template placeholder syntax: {err}"))?;
    let mut out = String::with_capacity(template.len() + 32);
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let close = after_open
            .find('}')
            .ok_or_else(|| anyhow::anyhow!("Unclosed '{{' in SSOT template"))?;
        let key = &after_open[..close];
        let val = params
            .get(key)
            .with_context(|| format!("Unresolved SSOT placeholder '{{{key}}}' in template"))?;
        out.push_str(val);
        rest = &after_open[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}
