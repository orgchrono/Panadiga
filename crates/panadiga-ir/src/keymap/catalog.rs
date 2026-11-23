//! Codeset catalog-to-keymap profile assembly and wire-preview formatting.

use std::collections::HashSet;

use super::super::classify::{classify_command_name, prettify_command_label};
use super::super::kaseikyo::{effective_subdevice, format_kaseikyo_wire_hex};
use super::super::types::{
    CatalogCommandSpec, DynamicKeymapProfile, DynamicRemoteKey, KeyProtocol,
    PanasonicCatalogCodeset,
};

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
