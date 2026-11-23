//! Data-driven builder for granular Pico P7501 CLI command DTOs (`cli/p7501_cli.c`).

use super::catalog::{
    CatalogCommandEntry, LocaleCommandEntry, hardware_catalog, pt_br_ssot_locale,
};
use super::constants::{PicoCommandCategory, catalog_template_tokens, resolve_catalog_tokens};
use super::dto::{PicoCommandArgOptionDto, PicoCommandSpecDto};

fn build_single_command(
    entry: &CatalogCommandEntry,
    text: &LocaleCommandEntry,
    timeout_ms: u64,
    tokens: &[(&str, String)],
) -> PicoCommandSpecDto {
    let category = PicoCommandCategory::try_from(entry.category.as_str())
        .unwrap_or_else(|e| panic!("Invalid category for command {}: {e}", entry.id));

    let resolved_default = entry
        .default_arg
        .as_deref()
        .map(|raw| resolve_catalog_tokens(raw, tokens));

    let arg_placeholder = resolved_default
        .clone()
        .or_else(|| text.arg_placeholder.clone());

    let arg_options = entry
        .options
        .iter()
        .map(|[opt_key, raw_value]| {
            let value = resolve_catalog_tokens(raw_value, tokens);
            let label = text
                .options
                .get(opt_key)
                .cloned()
                .unwrap_or_else(|| value.clone());
            PicoCommandArgOptionDto { label, value }
        })
        .collect();

    PicoCommandSpecDto {
        id: entry.id.clone(),
        category: category.as_str().to_string(),
        label: text.label.clone(),
        description: text.description.clone(),
        syntax: resolve_catalog_tokens(&entry.syntax, tokens),
        arg_label: text.arg_label.clone(),
        arg_placeholder,
        default_arg: resolved_default,
        arg_options,
        dangerous: entry.dangerous,
        timeout_ms,
    }
}

/// Builds the full SSOT Pico diagnostic & UART command catalog from `hardware_catalog.json`.
pub(super) fn build_pico_commands() -> Vec<PicoCommandSpecDto> {
    let catalog = hardware_catalog();
    let locale = pt_br_ssot_locale();
    let tokens = catalog_template_tokens();

    catalog
        .commands
        .iter()
        .map(|entry| {
            let text = locale.commands.get(&entry.id).unwrap_or_else(|| {
                panic!("Missing ssot.commands.{} entry in pt-BR.json", entry.id)
            });
            let timeout_ms = *catalog.timeouts_ms.get(&entry.timeout).unwrap_or_else(|| {
                panic!(
                    "Missing timeout key '{}' for command {}",
                    entry.timeout, entry.id
                )
            });
            build_single_command(entry, text, timeout_ms, &tokens)
        })
        .collect()
}

/// Returns `true` when `head` matches a known Pico CLI verb in `hardware_catalog.json`
/// or `INTERNAL_OR_ALIAS_CLI_COMMANDS`.
#[must_use]
pub fn is_known_pico_command_id(head: &str) -> bool {
    let trimmed = head.trim();
    !trimmed.is_empty()
        && (hardware_catalog()
            .commands
            .iter()
            .any(|c| c.id.eq_ignore_ascii_case(trimmed))
            || super::constants::INTERNAL_OR_ALIAS_CLI_COMMANDS
                .iter()
                .any(|c| c.eq_ignore_ascii_case(trimmed)))
}
