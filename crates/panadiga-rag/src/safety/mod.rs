//! Deterministic hardware safety shield for the Panasonic DIGA DMR-EH55.
//!
//! Enforces physical electrical, bootloader, and protocol invariants to prevent
//! irreversible equipment damage during reverse engineering and diagnostics.

mod patterns;

pub use patterns::extract_voltages_volts;

use panadiga_core::constants::{
    AM34_FLASH_BASE, AM34_FLASH_CRITICAL_IPL_SIZE, AM34_FLASH_UNCACHED_BASE,
    AM34_FORBIDDEN_LETHAL_REG,
};

use super::config::*;
use super::models::SafetyReport;
use patterns::{
    HEX_PATTERN, IPL_ERASE_PATTERN, NEGATION_REGEX, WRITE_ACTION_REGEX, sentence_context_window,
};

const UPPER_NOR_OPTIN_TOKENS: &[&str] = &[
    "confirm_upper_nor",
    "uppernorappunlocked",
    "bgo_unbrick",
    "patch-upper-nor",
];

fn is_critical_boot_ipl_addr(addr: u64) -> bool {
    let crit_span = u64::from(AM34_FLASH_CRITICAL_IPL_SIZE);
    let cached_lo = u64::from(AM34_FLASH_BASE);
    let uncached_lo = u64::from(AM34_FLASH_UNCACHED_BASE);
    addr < crit_span
        || (cached_lo..cached_lo + crit_span).contains(&addr)
        || (uncached_lo..uncached_lo + crit_span).contains(&addr)
}

fn is_upper_nor_bgo_addr(addr: u64) -> bool {
    let crit_span = u64::from(AM34_FLASH_CRITICAL_IPL_SIZE);
    let write_end = FORBIDDEN_FLASH_WRITE_END as u64;
    let cached_lo = u64::from(AM34_FLASH_BASE) + crit_span;
    let uncached_lo = u64::from(AM34_FLASH_UNCACHED_BASE) + crit_span;
    let uncached_end = u64::from(AM34_FLASH_UNCACHED_BASE) + write_end;
    (crit_span..=write_end).contains(&addr)
        || (cached_lo..=u64::from(FORBIDDEN_FLASH_BUS_END)).contains(&addr)
        || (uncached_lo..=uncached_end).contains(&addr)
}

pub struct HardwareSafetyValidator;

impl HardwareSafetyValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, text: &str, is_query: bool) -> SafetyReport {
        let mut violations = Vec::new();
        let mut critical_alerts = Vec::new();
        let mut warnings = Vec::new();

        let lower_text = text.to_lowercase();
        let upper_text = text.to_uppercase();
        let has_upper_nor_optin = UPPER_NOR_OPTIN_TOKENS
            .iter()
            .any(|tok| lower_text.contains(tok));

        for (val, start_idx, end_idx, matched_str) in extract_voltages_volts(&lower_text) {
            if val > MAX_SAFE_VOLTAGE_VOLTS {
                let context_window = sentence_context_window(&lower_text, start_idx, end_idx, 60);
                let has_warning = NEGATION_REGEX.is_match(context_window);

                if is_query || !has_warning {
                    violations.push(format!(
                        "CRITICAL ELECTRICAL HAZARD: Voltage {}V ({}) exceeds maximum safe tolerance of {}V on Matsushita AM34 / VEP79138 digital board.",
                        val, matched_str, MAX_SAFE_VOLTAGE_VOLTS
                    ));
                    critical_alerts.push(format!(
                        "Overvoltage detected ({}V > {}V). Immediate risk of gate breakdown.",
                        val, MAX_SAFE_VOLTAGE_VOLTS
                    ));
                } else {
                    warnings.push(format!(
                        "High voltage {}V referenced in warning or prohibition context.",
                        val
                    ));
                }
            }
        }

        for m in HEX_PATTERN.find_iter(&lower_text) {
            let raw_hex = m.as_str();
            let Ok(addr) = u64::from_str_radix(
                raw_hex.trim_start_matches("0x").trim_start_matches("0X"),
                16,
            ) else {
                continue;
            };

            let context_window = sentence_context_window(&lower_text, m.start(), m.end(), 120);
            if addr != 0 && addr < 0x1000 && raw_hex.len() <= 4 {
                continue;
            }

            let address_keywords = [
                "address",
                "offset",
                "flash",
                "starting at",
                "from 0x",
                "sector",
                "sa0",
                "sa133",
                "range",
            ];
            let is_address = raw_hex.len() >= 6
                || addr >= 0x1000
                || address_keywords.iter().any(|k| context_window.contains(k));
            if !is_address {
                continue;
            }

            let is_explicit_sram = addr <= 0x0003_FFFF
                && (context_window.contains("sram") || context_window.contains("ram loader"))
                && !context_window.contains("flash")
                && !context_window.contains("ipl")
                && !context_window.contains("bootloader")
                && !context_window.contains("spansion")
                && !context_window.contains("sa0");
            if is_explicit_sram {
                continue;
            }

            let lethal_lo = u64::from(AM34_FORBIDDEN_LETHAL_REG);
            if (lethal_lo..=lethal_lo + 3).contains(&addr) {
                let has_warning = NEGATION_REGEX.is_match(context_window);
                if !has_warning {
                    violations.push(format!(
                        "CRITICAL MMIO HAZARD: Address 0x{:08X} targets forbidden AM34 lethal register (causes immediate bus/WDT lockup).",
                        AM34_FORBIDDEN_LETHAL_REG
                    ));
                    critical_alerts.push(format!(
                        "Forbidden lethal register access detected at {}.",
                        raw_hex.to_uppercase()
                    ));
                } else {
                    warnings.push(format!(
                        "Lethal register {} referenced in protective warning context.",
                        raw_hex.to_uppercase()
                    ));
                }
                continue;
            }

            if is_critical_boot_ipl_addr(addr) {
                let has_local_write = WRITE_ACTION_REGEX.is_match(context_window);
                let has_warning = NEGATION_REGEX.is_match(context_window);

                if (is_query && has_local_write) || (!is_query && has_local_write && !has_warning) {
                    violations.push(format!(
                        "CRITICAL BOOTLOADER HAZARD: Flash address {} resides in permanently locked Layer 1 Critical Boot IPL (SA0..SA10 / 256 KiB: 0x{:08X}..0x{:08X} / 0x{:08X}..0x{:08X}). Erasing or writing this region permanently bricks the Root of Trust (E05).",
                        raw_hex.to_uppercase(),
                        AM34_FLASH_BASE,
                        AM34_FLASH_BASE + AM34_FLASH_CRITICAL_IPL_SIZE - 1,
                        AM34_FLASH_UNCACHED_BASE,
                        AM34_FLASH_UNCACHED_BASE + AM34_FLASH_CRITICAL_IPL_SIZE - 1
                    ));
                    critical_alerts.push(format!(
                        "Attempted write/erase at Layer 1 Critical Boot IPL address {}. Operation vetoed by air-gap shield.",
                        raw_hex.to_uppercase()
                    ));
                } else if has_warning {
                    warnings.push(format!(
                        "Layer 1 Critical Boot IPL address {} referenced in protective warning context.",
                        raw_hex.to_uppercase()
                    ));
                }
            } else if is_upper_nor_bgo_addr(addr) {
                let has_local_write = WRITE_ACTION_REGEX.is_match(context_window);
                let has_warning = NEGATION_REGEX.is_match(context_window);

                if has_local_write && has_upper_nor_optin {
                    warnings.push(format!(
                        "UPPER NOR / BGO WRITE RISK ACKNOWLEDGED (CONFIRM_UPPER_NOR): Target {} is inside Layer 2 Upper NOR/BGO (0x{:08X}..0x{:08X} / 0x{:08X}..0x{:08X}). Mandatory compliance for bricked U99 recovery: verify a full 8 MiB SHA-256 backup first, wait ~700 ms (poll 0xFF x 32) on 64 KiB Spansion sector erases, and keep Layer 1 Critical IPL (SA0..SA10: 0x{:08X}..0x{:08X}) permanently locked (E05).",
                        raw_hex.to_uppercase(),
                        AM34_FLASH_BASE + AM34_FLASH_CRITICAL_IPL_SIZE,
                        FORBIDDEN_FLASH_BUS_END,
                        AM34_FLASH_UNCACHED_BASE + AM34_FLASH_CRITICAL_IPL_SIZE,
                        AM34_FLASH_UNCACHED_BASE + (FORBIDDEN_FLASH_WRITE_END as u32),
                        AM34_FLASH_BASE,
                        AM34_FLASH_BASE + AM34_FLASH_CRITICAL_IPL_SIZE - 1
                    ));
                } else if (is_query && has_local_write)
                    || (!is_query && has_local_write && !has_warning)
                {
                    violations.push(format!(
                        "CRITICAL BOOTLOADER HAZARD: Flash address {} is within Layer 2 Upper NOR / BGO (SA11..SA133: 0x{:08X}..0x{:08X} / 0x{:08X}..0x{:08X}). Blocked by default (E05) unless CONFIRM_UPPER_NOR + mandatory 8 MiB SHA-256 backup are used for bricked U99 BGO recovery.",
                        raw_hex.to_uppercase(),
                        AM34_FLASH_BASE + AM34_FLASH_CRITICAL_IPL_SIZE,
                        FORBIDDEN_FLASH_BUS_END,
                        AM34_FLASH_UNCACHED_BASE + AM34_FLASH_CRITICAL_IPL_SIZE,
                        AM34_FLASH_UNCACHED_BASE + (FORBIDDEN_FLASH_WRITE_END as u32)
                    ));
                    critical_alerts.push(format!(
                        "Blocked Layer 2 Upper NOR write/erase at {} without explicit CONFIRM_UPPER_NOR opt-in.",
                        raw_hex.to_uppercase()
                    ));
                } else if has_warning {
                    warnings.push(format!(
                        "Upper NOR / BGO address {} referenced in protective warning context.",
                        raw_hex.to_uppercase()
                    ));
                }
            }
        }

        for m in IPL_ERASE_PATTERN.find_iter(&lower_text) {
            let context_window = sentence_context_window(&lower_text, m.start(), m.end(), 80);
            if is_query || !NEGATION_REGEX.is_match(context_window) {
                violations.push(format!(
                    "CRITICAL BOOTLOADER HAZARD: Direct instruction to modify IPL detected ('{}'). Erasing IPL sectors SA0..SA133 is strictly prohibited.",
                    m.as_str()
                ));
            }
        }

        if upper_text.contains("P7501") {
            let forbidden_pins = ["TDI", "TDO", "TMS", "TRST", "TCK"];
            let mentioned_jtag: Vec<&str> = forbidden_pins
                .iter()
                .filter(|&&pin| upper_text.contains(pin))
                .copied()
                .collect();

            if !mentioned_jtag.is_empty() && (is_query || !NEGATION_REGEX.is_match(&lower_text)) {
                violations.push(format!(
                    "CRITICAL PROTOCOL HAZARD: P7501 is a proprietary 6-pin Panasonic asynchronous UART (115200 8N1) & discrete boot/reset header (SW_UART, XRESET, UARTM2P_T, GND, NSW5V, UARTP2M_T). Standard JTAG pins ({}) do NOT exist on P7501.",
                    mentioned_jtag.join(", ")
                ));
                critical_alerts.push(format!(
                    "Forbidden JTAG pin assignment on P7501: {}",
                    mentioned_jtag.join(", ")
                ));
            }
        }

        for util in [
            "diskpart",
            "fdisk",
            "format c:",
            "format d:",
            "format e:",
            "format /fs:ntfs",
            "format /fs:fat32",
        ] {
            if let Some(pos) = lower_text.find(util) {
                let context_window =
                    sentence_context_window(&lower_text, pos, pos + util.len(), 80);
                if is_query || !NEGATION_REGEX.is_match(context_window) {
                    violations.push(format!(
                        "CRITICAL STORAGE HAZARD: PC utility '{}' destroys MEIHDFS 'SH' signature and firmware partitions.",
                        util
                    ));
                    critical_alerts.push(format!(
                        "Destructive disk formatting utility detected: {}",
                        util
                    ));
                }
            }
        }

        let mut mkfs_idx = 0;
        while let Some(pos) = lower_text[mkfs_idx..].find("mkfs") {
            let actual_pos = mkfs_idx + pos;
            let remainder = &lower_text[actual_pos..];
            if !remainder.starts_with("mkfs.meihdfs") && !remainder.starts_with("mkfs_meihdfs") {
                let context_window =
                    sentence_context_window(&lower_text, actual_pos, actual_pos + 4, 80);
                if is_query || !NEGATION_REGEX.is_match(context_window) {
                    violations.push(
                        "CRITICAL STORAGE HAZARD: PC utility 'mkfs' destroys MEIHDFS 'SH' signature and firmware partitions.".to_string(),
                    );
                    critical_alerts
                        .push("Destructive disk formatting utility detected: mkfs".to_string());
                    break;
                }
            }
            mkfs_idx = actual_pos + 4;
        }

        SafetyReport {
            is_safe: violations.is_empty(),
            violations,
            critical_alerts,
            warnings,
        }
    }
}

impl Default for HardwareSafetyValidator {
    fn default() -> Self {
        Self::new()
    }
}
