//! Platform IR driver adaptation for Android `ConsumerIrManager`, Samsung Exynos `sec_ir`
//! sysfs (`ice4_fpga` / `ir_remote_con_mc96`), and AOSP microsecond-budget constraints.

use super::kaseikyo::{KASEIKYO_TRANSITION_COUNT, PULSE_BIT_MARK_US};
use super::types::{IrDeviceInfo, IrFrequencyRange, IrPreparedSignal, IrTimingUnit};

pub const MAX_HARDWARE_TRAIL_SPACE_US: u32 = 45_000;
pub const MAX_AOSP_XMIT_TIME_US: u64 = 1_900_000;
pub const MAX_UINT16_VALUE: u32 = 0xFFFF;

/// Known period-dividing iCE40 FPGA devices/SoCs (`ice4_fpga.c` on Exynos 5420 / Galaxy Tab S SM-T805M).
const FPGA_SOC_SIGNATURES: &[&str] = &[
    "5420",
    "5422",
    "5430",
    "5433",
    "universal5420",
    "t805",
    "t800",
    "t705",
    "t700",
    "p600",
    "p605",
    "p900",
    "p905",
    "chagall",
    "klimt",
];

/// Known cycle-register I2C microcontroller devices/SoCs (`ir_remote_con_mc96` on Exynos 4412).
const CYCLE_MCU_SOC_SIGNATURES: &[&str] = &[
    "4x12", "4412", "4210", "smdk4x12", "smdk4412", "n8000", "n8010", "n8013", "n8020", "p4note",
    "p3100", "p5100",
];

fn combined_platform_blob(info: &IrDeviceInfo) -> String {
    format!(
        "{} {} {} {} {}",
        info.hardware, info.board, info.model, info.product, info.device
    )
    .to_ascii_lowercase()
}

/// Checks whether `requested_hz` fits within the overall `[min..max]` hardware carrier span.
#[must_use]
pub fn is_carrier_supported(requested_hz: u32, ranges: &[IrFrequencyRange]) -> bool {
    if ranges.is_empty() || ranges.iter().any(|r| r.contains(requested_hz)) {
        return true;
    }
    let min_supported = ranges.iter().map(|r| r.min_hz).min().unwrap_or(0);
    let max_supported = ranges.iter().map(|r| r.max_hz).max().unwrap_or(0);
    requested_hz >= min_supported && requested_hz <= max_supported
}

/// Preserves exact `requested_hz` when inside overall span; clamps only to nearest boundary if outside.
#[must_use]
pub fn resolve_carrier(requested_hz: u32, ranges: &[IrFrequencyRange]) -> u32 {
    if is_carrier_supported(requested_hz, ranges) {
        return requested_hz;
    }
    ranges
        .iter()
        .flat_map(|r| [r.min_hz, r.max_hz])
        .min_by_key(|&edge| edge.abs_diff(requested_hz))
        .unwrap_or(requested_hz)
}

/// Returns `true` for Exynos iCE40 FPGA platforms where HAL + kernel double-divide durations.
#[must_use]
pub fn is_period_dividing_fpga_driver(info: &IrDeviceInfo, has_fpga_node: bool) -> bool {
    if has_fpga_node {
        return true;
    }
    let blob = combined_platform_blob(info);
    FPGA_SOC_SIGNATURES.iter().any(|sig| blob.contains(sig))
}

/// Returns `true` for Exynos cycle-register `mc96` IR MCU platforms.
#[must_use]
pub fn is_cycle_register_mcu_driver(
    info: &IrDeviceInfo,
    has_sec_ir_node: bool,
    has_fpga_node: bool,
) -> bool {
    if has_sec_ir_node && !has_fpga_node && !is_period_dividing_fpga_driver(info, false) {
        return true;
    }
    let blob = combined_platform_blob(info);
    CYCLE_MCU_SOC_SIGNATURES
        .iter()
        .any(|sig| blob.contains(sig))
}

/// Recommends the optimal timing unit for Android's `ConsumerIrManager.transmit`.
#[must_use]
pub fn recommended_timing_unit(
    info: &IrDeviceInfo,
    has_sec_ir_node: bool,
    has_fpga_node: bool,
) -> IrTimingUnit {
    if is_period_dividing_fpga_driver(info, has_fpga_node) {
        IrTimingUnit::PeriodScaled
    } else if is_cycle_register_mcu_driver(info, has_sec_ir_node, has_fpga_node) {
        IrTimingUnit::CarrierCycles
    } else {
        IrTimingUnit::Microseconds
    }
}

/// Encodes microsecond mark/space intervals according to `unit` with hardware overflow guards.
#[must_use]
pub fn encode_pattern(pattern_us: &[u32], carrier_hz: u32, unit: IrTimingUnit) -> Vec<u32> {
    if pattern_us.is_empty() {
        return Vec::new();
    }
    let safe_carrier = carrier_hz.max(1);
    let last_idx = pattern_us.len() - 1;

    match unit {
        IrTimingUnit::Microseconds => pattern_us
            .iter()
            .enumerate()
            .map(|(idx, &raw)| {
                let us = raw.max(1);
                if idx == last_idx && pattern_us.len() >= KASEIKYO_TRANSITION_COUNT {
                    us.min(MAX_HARDWARE_TRAIL_SPACE_US)
                } else {
                    us
                }
            })
            .collect(),
        IrTimingUnit::CarrierCycles => pattern_us
            .iter()
            .map(|&raw| {
                let cycles = (u64::from(raw) * u64::from(safe_carrier) + 500_000) / 1_000_000;
                (cycles as u32).clamp(1, MAX_UINT16_VALUE)
            })
            .collect(),
        IrTimingUnit::PeriodScaled => {
            let factor = (1_000_000 / safe_carrier).max(1);
            let mut out = Vec::with_capacity(pattern_us.len());
            let mut running_sum = 0u64;
            for &raw in &pattern_us[..last_idx] {
                let scaled = raw.max(1).saturating_mul(factor);
                out.push(scaled);
                running_sum += u64::from(scaled);
            }
            let min_trail = PULSE_BIT_MARK_US.saturating_mul(factor);
            let desired_trail = pattern_us[last_idx]
                .min(MAX_HARDWARE_TRAIL_SPACE_US)
                .saturating_mul(factor);
            let budget = MAX_AOSP_XMIT_TIME_US
                .saturating_sub(running_sum)
                .min(u64::from(u32::MAX)) as u32;
            out.push(desired_trail.min(budget.max(min_trail)));
            out
        }
    }
}

/// Formats comma-separated carrier + pulse array for direct `/sys/class/sec/sec_ir/ir_send` writes.
#[must_use]
pub fn format_sysfs_payload(
    pattern_us: &[u32],
    carrier_hz: u32,
    expects_cycles_in_sysfs: bool,
) -> String {
    let unit = if expects_cycles_in_sysfs {
        IrTimingUnit::CarrierCycles
    } else {
        IrTimingUnit::Microseconds
    };
    let encoded = encode_pattern(pattern_us, carrier_hz, unit);
    let mut out = String::with_capacity(encoded.len() * 6 + 12);
    out.push_str(&carrier_hz.to_string());
    for val in encoded {
        out.push(',');
        out.push_str(&val.to_string());
    }
    out
}

/// Input parameters for [`prepare_transmission`] across Exynos sysfs and `ConsumerIrManager`.
#[derive(Debug, Clone)]
pub struct IrPrepareParams<'a> {
    pub raw_carrier_hz: u32,
    pub pattern_us: Vec<u32>,
    pub wire_hex: String,
    pub pronto_hex: String,
    pub ranges: &'a [IrFrequencyRange],
    pub device_info: &'a IrDeviceInfo,
    pub has_sec_ir_node: bool,
    pub has_fpga_node: bool,
    pub unit_override: Option<IrTimingUnit>,
}

/// Builds a complete prepared transmission packet for Android HAL execution.
#[must_use]
pub fn prepare_transmission(params: IrPrepareParams<'_>) -> IrPreparedSignal {
    let IrPrepareParams {
        raw_carrier_hz,
        pattern_us,
        wire_hex,
        pronto_hex,
        ranges,
        device_info,
        has_sec_ir_node,
        has_fpga_node,
        unit_override,
    } = params;
    let carrier_hz = resolve_carrier(raw_carrier_hz, ranges);
    let recommended = recommended_timing_unit(device_info, has_sec_ir_node, has_fpga_node);
    let effective_unit = unit_override.unwrap_or(recommended);
    let is_fpga = is_period_dividing_fpga_driver(device_info, has_fpga_node);
    let expects_cycles_in_sysfs = !is_fpga;
    let encoded_pattern = encode_pattern(&pattern_us, carrier_hz, effective_unit);
    let sysfs_payload = format_sysfs_payload(&pattern_us, carrier_hz, expects_cycles_in_sysfs);

    IrPreparedSignal {
        carrier_hz,
        pattern_us,
        encoded_pattern,
        sysfs_payload,
        expects_cycles_in_sysfs,
        recommended_unit: recommended,
        effective_unit,
        wire_hex,
        pronto_hex,
    }
}
