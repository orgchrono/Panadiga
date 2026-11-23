//! Declarative Single Source of Truth loader for manual- & binary-verified DIGA service macros.

use std::sync::OnceLock;

use super::types::VerifiedIrMacro;

const VERIFIED_MACROS_JSON: &str = include_str!("ir_macros_ssot.json");

static PARSED_MACROS: OnceLock<Vec<VerifiedIrMacro>> = OnceLock::new();

/// Returns all hardware- & service-manual-verified DIGA IR macros loaded from `ir_macros_ssot.json`.
pub fn verified_service_macros() -> &'static [VerifiedIrMacro] {
    PARSED_MACROS
        .get_or_init(|| {
            serde_json::from_str::<Vec<VerifiedIrMacro>>(VERIFIED_MACROS_JSON)
                .expect("ir_macros_ssot.json must be valid VerifiedIrMacro array")
        })
        .as_slice()
}
