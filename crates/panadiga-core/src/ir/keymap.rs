//! Dynamic keymap assembly and bidirectional `.irplus` XML / Flipper `.ir` codec.

use std::collections::{HashMap, HashSet};

use super::classify::{classify_command_name, prettify_command_label};
use super::kaseikyo::{
    decode_flipper_kaseikyo, decode_kaseikyo_from_pronto, effective_subdevice,
    format_kaseikyo_wire_hex, synthesize_kaseikyo_pronto,
};
use super::types::{
    CatalogCommandSpec, DynamicKeymapProfile, DynamicRemoteKey, KeyProtocol, KeyTone,
    PanasonicCatalogCodeset, PanasonicCategory,
};

/// Auto-assembles a sorted, deduplicated [`DynamicKeymapProfile`] from any codeset.
#[must_use]
pub fn assemble_profile_from_codeset(
    codeset: &PanasonicCatalogCodeset,
    preferred_name: Option<&str>,
    is_user_modified: bool,
) -> DynamicKeymapProfile {
    let display_name = preferred_name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            codeset
                .models
                .first()
                .cloned()
                .unwrap_or_else(|| codeset.id.clone())
        });

    let mut seen = HashSet::new();
    let mut scored = Vec::new();
    for (idx, cmd) in codeset.commands.iter().enumerate() {
        let dedup_key = match cmd.protocol {
            KeyProtocol::Kaseikyo => format!(
                "{}|K|{}|{}|{}",
                cmd.name.to_ascii_lowercase(),
                cmd.device,
                cmd.subdevice,
                cmd.obc
            ),
            KeyProtocol::ProntoRaw => format!(
                "{}|P|{}",
                cmd.name.to_ascii_lowercase(),
                cmd.pronto_hex.as_deref().unwrap_or("")
            ),
        };
        if !seen.insert(dedup_key) {
            continue;
        }
        let info = classify_command_name(&cmd.name);
        let respect_mode = cmd.protocol == KeyProtocol::Kaseikyo
            && (cmd.device == 0xB0 || cmd.device == 0x90)
            && (cmd.subdevice & 0xF0) == 0x00;

        let wire_preview = format_preview(cmd, 1, respect_mode);
        let key = DynamicRemoteKey {
            id: format!("{}_{idx}", codeset.id),
            label: prettify_command_label(&cmd.name),
            section: info.section,
            protocol: cmd.protocol,
            device: cmd.device,
            subdevice: cmd.subdevice,
            obc: cmd.obc,
            respect_remote_mode: respect_mode,
            pronto_hex: cmd.pronto_hex.clone(),
            tone: info.tone,
            confirm_required: info.confirm_required,
            wire_preview,
        };
        scored.push((info.sort_key, key));
    }

    scored.sort_by(|(sort_a, ka), (sort_b, kb)| {
        ka.section
            .cmp(&kb.section)
            .then_with(|| sort_a.cmp(sort_b))
            .then_with(|| {
                ka.label
                    .to_ascii_lowercase()
                    .cmp(&kb.label.to_ascii_lowercase())
            })
    });

    DynamicKeymapProfile {
        id: format!("profile_{}", codeset.id),
        name: display_name,
        source_codeset_id: codeset.id.clone(),
        category: codeset.category,
        keys: scored.into_iter().map(|(_, k)| k).collect(),
        is_user_modified,
    }
}

/// Imports an `.irplus` XML profile into an editable [`DynamicKeymapProfile`].
#[must_use]
pub fn import_irplus_xml(xml_text: &str, profile_name: Option<&str>) -> DynamicKeymapProfile {
    let model = extract_attr(xml_text, "model").unwrap_or_default();
    let title = profile_name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            if model.is_empty() {
                "Imported .irplus Profile".to_string()
            } else {
                model
            }
        });

    let mut commands = Vec::new();
    let mut cursor = xml_text;
    while let Some(btn_start) = find_ascii_ignore_case(cursor, "<button") {
        let after_start = &cursor[btn_start + 7..];
        let Some(gt_off) = after_start.find('>') else {
            break;
        };
        let attrs = &after_start[..gt_off];
        let rest = &after_start[gt_off + 1..];
        let Some(btn_end) = find_ascii_ignore_case(rest, "</button>") else {
            break;
        };
        let body = &rest[..btn_end];
        cursor = &rest[btn_end + 9..];

        let label = decode_xml_entities(&extract_attr(attrs, "label").unwrap_or_default());
        let code = decode_xml_entities(extract_tag_inner(body, "code").unwrap_or(body));
        let label = label.trim();
        let code = code.trim();
        if label.is_empty() || code.is_empty() || code.contains(';') {
            continue;
        }
        if let Some((dev, sub, obc)) = decode_kaseikyo_from_pronto(code) {
            commands.push(CatalogCommandSpec {
                name: label.to_string(),
                protocol: KeyProtocol::Kaseikyo,
                device: dev,
                subdevice: sub,
                obc,
                pronto_hex: None,
            });
        } else {
            commands.push(CatalogCommandSpec {
                name: label.to_string(),
                protocol: KeyProtocol::ProntoRaw,
                device: 0,
                subdevice: 0,
                obc: 0,
                pronto_hex: Some(code.to_string()),
            });
        }
    }

    let uid = format!("imp_irplus_{:x}_{}", simple_fnv1a(&title), commands.len());
    assemble_profile_from_codeset(
        &PanasonicCatalogCodeset {
            id: uid,
            category: PanasonicCategory::Other,
            models: vec![title.clone()],
            commands,
        },
        Some(&title),
        true,
    )
}

/// Imports a Flipper Zero `.ir` capture (`protocol: Kaseikyo`) into an editable profile.
#[must_use]
pub fn import_flipper_ir(ir_text: &str, profile_name: &str) -> DynamicKeymapProfile {
    let mut commands = Vec::new();
    let mut current: HashMap<String, String> = HashMap::new();

    for raw_line in ir_text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            commit_flipper_entry(&mut current, &mut commands);
        } else if let Some((k, v)) = line.split_once(':') {
            current.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    commit_flipper_entry(&mut current, &mut commands);

    let title = if profile_name.trim().is_empty() {
        "Flipper Zero (.ir)"
    } else {
        profile_name.trim()
    };
    let uid = format!("imp_flipper_{:x}_{}", simple_fnv1a(title), commands.len());
    assemble_profile_from_codeset(
        &PanasonicCatalogCodeset {
            id: uid,
            category: PanasonicCategory::Other,
            models: vec![title.to_string()],
            commands,
        },
        Some(title),
        true,
    )
}

/// Exports any [`DynamicKeymapProfile`] back into `.irplus` XML preserving active `SET 1/2/3` mode.
#[must_use]
pub fn export_irplus_xml(profile: &DynamicKeymapProfile, remote_mode: u8) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<irplus>\n");
    out.push_str(&format!(
        "    <device manufacturer=\"Panasonic\" model=\"{}\">\n",
        escape_xml(&profile.name)
    ));
    for key in &profile.keys {
        let pronto = match key.protocol {
            KeyProtocol::Kaseikyo => {
                let sub = effective_subdevice(key.subdevice, remote_mode, key.respect_remote_mode);
                synthesize_kaseikyo_pronto(key.device, sub, key.obc)
            }
            KeyProtocol::ProntoRaw => key.pronto_hex.clone().unwrap_or_default(),
        };
        if pronto.trim().is_empty() {
            continue;
        }
        out.push_str(&format!(
            "        <button label=\"{}\" color=\"{}\">\n            <code>{}</code>\n        </button>\n",
            escape_xml(&key.label),
            tone_to_irplus_color(key.tone),
            pronto
        ));
    }
    out.push_str("    </device>\n</irplus>\n");
    out
}

fn format_preview(cmd: &CatalogCommandSpec, mode: u8, respect_mode: bool) -> String {
    match cmd.protocol {
        KeyProtocol::Kaseikyo => {
            let sub = effective_subdevice(cmd.subdevice, mode, respect_mode);
            format_kaseikyo_wire_hex(cmd.device, sub, cmd.obc)
        }
        KeyProtocol::ProntoRaw => {
            let wc = cmd
                .pronto_hex
                .as_deref()
                .map(|p| p.split_whitespace().count())
                .unwrap_or(0);
            format!("PRONTO ({wc}w)")
        }
    }
}

fn commit_flipper_entry(
    entry: &mut HashMap<String, String>,
    commands: &mut Vec<CatalogCommandSpec>,
) {
    let name = entry.get("name").map(String::as_str).unwrap_or("").trim();
    let prot = entry
        .get("protocol")
        .map(String::as_str)
        .unwrap_or("")
        .trim();
    if !name.is_empty() && prot.eq_ignore_ascii_case("Kaseikyo") {
        let addr = entry.get("address").map(String::as_str).unwrap_or("");
        let cmd = entry.get("command").map(String::as_str).unwrap_or("");
        if let Some((dev, sub, obc)) = decode_flipper_kaseikyo(addr, cmd) {
            commands.push(CatalogCommandSpec {
                name: name.to_string(),
                protocol: KeyProtocol::Kaseikyo,
                device: dev,
                subdevice: sub,
                obc,
                pronto_hex: None,
            });
        }
    }
    entry.clear();
}

fn find_ascii_ignore_case(haystack: &str, needle: &str) -> Option<usize> {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || h.len() < n.len() {
        return None;
    }
    h.windows(n.len()).position(|w| w.eq_ignore_ascii_case(n))
}

fn extract_attr(attrs: &str, attr_name: &str) -> Option<String> {
    let needle = format!("{attr_name}=\"");
    let pos = find_ascii_ignore_case(attrs, &needle)?;
    let rest = &attrs[pos + needle.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_tag_inner<'a>(body: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let s = find_ascii_ignore_case(body, &open)?;
    let after = &body[s + open.len()..];
    let e = find_ascii_ignore_case(after, &close)?;
    Some(&after[..e])
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

const fn tone_to_irplus_color(tone: KeyTone) -> &'static str {
    match tone {
        KeyTone::Neutral => "#FF37474F",
        KeyTone::Primary => "#FF0277BD",
        KeyTone::Info => "#FF0288D1",
        KeyTone::Success => "#FF2E7D32",
        KeyTone::Warning => "#FFF57C00",
        KeyTone::Danger => "#FFD32F2F",
        KeyTone::DangerDark => "#FFC62828",
        KeyTone::Accent => "#FF7B1FA2",
    }
}

fn simple_fnv1a(s: &str) -> u32 {
    let mut h = 0x811C_9DC5u32;
    for b in s.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}
