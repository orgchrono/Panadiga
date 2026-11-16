//! Android-only Tauri IPC commands for DIGA Commander IR synthesis, Exynos HAL preparation,
//! offline All-Panasonic catalog browsing, dynamic `.irplus`/Flipper `.ir` keymaps, and
//! declarative verified service macros. Compiled strictly under `target_os = "android"` & tests.

use serde::{Deserialize, Serialize};

use panadiga_core::ir::{
    CARRIER_MATSUSHITA_HZ, DynamicKeymapProfile, IrDeviceInfo, IrFrequencyRange, IrPrepareParams,
    IrPreparedSignal, IrTimingUnit, KeyProtocol, PanasonicCatalogCodeset, PanasonicCatalogSummary,
    PanasonicCategory, VerifiedIrMacro, assemble_profile_from_codeset, decode_kaseikyo_from_pronto,
    effective_subdevice, export_irplus_xml, find_codeset, format_kaseikyo_wire_hex,
    import_flipper_ir, import_irplus_xml, parse_pronto_hex, prepare_transmission, search_codesets,
    synthesize_kaseikyo_pattern, synthesize_kaseikyo_pronto, verified_service_macros,
};

#[derive(Debug, Clone, Serialize)]
pub struct IrCatalogStatsDto {
    pub codeset_count: usize,
    pub command_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssembledCodesetDto {
    pub codeset: PanasonicCatalogCodeset,
    pub profile: DynamicKeymapProfile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IrPrepareTransmissionRequest {
    pub protocol: KeyProtocol,
    pub device: u8,
    pub subdevice: u8,
    pub obc: u8,
    pub respect_remote_mode: bool,
    pub remote_unit: u8,
    pub pronto_hex: Option<String>,
    pub ranges: Vec<IrFrequencyRange>,
    pub device_info: IrDeviceInfo,
    pub has_sec_ir_node: bool,
    pub has_fpga_node: bool,
    pub unit_override: String,
}

fn parse_unit_override(raw: &str) -> Option<IrTimingUnit> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "MICROSECONDS" => Some(IrTimingUnit::Microseconds),
        "CARRIER_CYCLES" => Some(IrTimingUnit::CarrierCycles),
        "PERIOD_SCALED" => Some(IrTimingUnit::PeriodScaled),
        _ => None,
    }
}

#[tauri::command]
pub fn ir_catalog_stats() -> IrCatalogStatsDto {
    let (codeset_count, command_count) = panadiga_core::ir::catalog_stats();
    IrCatalogStatsDto {
        codeset_count,
        command_count,
    }
}

#[tauri::command]
pub fn ir_search_catalog(
    query: String,
    category: PanasonicCategory,
    limit: Option<usize>,
) -> Vec<PanasonicCatalogSummary> {
    search_codesets(&query, category, limit)
}

#[tauri::command]
pub fn ir_assemble_codeset(codeset_id: String) -> Result<AssembledCodesetDto, String> {
    let cs = find_codeset(&codeset_id)
        .ok_or_else(|| format!("Panasonic codeset '{codeset_id}' not found in offline catalog"))?;
    let profile = assemble_profile_from_codeset(cs, None, false);
    Ok(AssembledCodesetDto {
        codeset: cs.clone(),
        profile,
    })
}

#[tauri::command]
pub fn ir_import_profile(
    format: String,
    content: String,
    fallback_name: Option<String>,
) -> Result<DynamicKeymapProfile, String> {
    let name = fallback_name
        .as_deref()
        .unwrap_or("Imported Panasonic Profile");
    match format.trim().to_ascii_lowercase().as_str() {
        "irplus" | "xml" => Ok(import_irplus_xml(&content, fallback_name.as_deref())),
        "flipper" | "ir" => Ok(import_flipper_ir(&content, name)),
        other => Err(format!("Unsupported IR profile format: {other}")),
    }
}

#[tauri::command]
pub fn ir_export_irplus(profile: DynamicKeymapProfile) -> String {
    export_irplus_xml(&profile, 2)
}

#[tauri::command]
pub fn ir_prepare_transmission(
    request: IrPrepareTransmissionRequest,
) -> Result<IrPreparedSignal, String> {
    let unit_override = parse_unit_override(&request.unit_override);
    match request.protocol {
        KeyProtocol::Kaseikyo => {
            let eff_sub = effective_subdevice(
                request.subdevice,
                request.remote_unit,
                request.respect_remote_mode,
            );
            let pattern_us = synthesize_kaseikyo_pattern(request.device, eff_sub, request.obc);
            let wire_hex = format_kaseikyo_wire_hex(request.device, eff_sub, request.obc);
            let pronto_hex = synthesize_kaseikyo_pronto(request.device, eff_sub, request.obc);
            Ok(prepare_transmission(IrPrepareParams {
                raw_carrier_hz: CARRIER_MATSUSHITA_HZ,
                pattern_us,
                wire_hex,
                pronto_hex,
                ranges: &request.ranges,
                device_info: &request.device_info,
                has_sec_ir_node: request.has_sec_ir_node,
                has_fpga_node: request.has_fpga_node,
                unit_override,
            }))
        }
        KeyProtocol::ProntoRaw => {
            let raw_pronto = request
                .pronto_hex
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Missing Pronto Hex string for PRONTO_RAW command".to_string())?;
            let (raw_carrier_hz, pattern_us) =
                parse_pronto_hex(raw_pronto).map_err(|e| e.to_string())?;
            let wire_hex = decode_kaseikyo_from_pronto(raw_pronto)
                .map(|(dev, sub, obc)| format_kaseikyo_wire_hex(dev, sub, obc))
                .unwrap_or_default();
            Ok(prepare_transmission(IrPrepareParams {
                raw_carrier_hz,
                pattern_us,
                wire_hex,
                pronto_hex: raw_pronto.to_string(),
                ranges: &request.ranges,
                device_info: &request.device_info,
                has_sec_ir_node: request.has_sec_ir_node,
                has_fpga_node: request.has_fpga_node,
                unit_override,
            }))
        }
    }
}

#[tauri::command]
pub fn ir_get_verified_macros() -> Vec<VerifiedIrMacro> {
    verified_service_macros().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_ir_commands_prepare_exynos_signals_and_assemble_eh55() {
        let stats = ir_catalog_stats();
        assert!(stats.codeset_count > 1500);

        let hits = ir_search_catalog("DMR-EH55".to_string(), PanasonicCategory::Diga, Some(5));
        assert_eq!(hits[0].id, "curated_diga_eh55_eur7659");

        let assembled = ir_assemble_codeset("curated_diga_eh55_eur7659".to_string())
            .expect("assemble curated EH55");
        assert!(!assembled.profile.keys.is_empty());

        let prep = ir_prepare_transmission(IrPrepareTransmissionRequest {
            protocol: KeyProtocol::Kaseikyo,
            device: 0xB0,
            subdevice: 0x00,
            obc: 0x3D,
            respect_remote_mode: true,
            remote_unit: 2,
            pronto_hex: None,
            ranges: vec![IrFrequencyRange {
                min_hz: 36_000,
                max_hz: 40_000,
            }],
            device_info: IrDeviceInfo {
                manufacturer: "samsung".into(),
                model: "SM-T805M".into(),
                hardware: "universal5420".into(),
                board: "chagalllte".into(),
                product: "chagalllteub".into(),
                device: "chagalllte".into(),
                android_release: "13".into(),
                sdk_int: 33,
            },
            has_sec_ir_node: true,
            has_fpga_node: false,
            unit_override: "AUTO".into(),
        })
        .expect("prepare Exynos signal");
        assert_eq!(prep.effective_unit, IrTimingUnit::PeriodScaled);
        assert_eq!(prep.wire_hex, "02 20 B0 10 3D 9D");

        let macros = ir_get_verified_macros();
        assert!(macros.len() >= 12);
    }
}
