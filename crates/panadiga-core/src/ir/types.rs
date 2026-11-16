//! Domain models for Panasonic IR synthesis, platform driver adaptation, and dynamic keymaps.

use serde::{Deserialize, Serialize};

/// Pulse encoding unit expected by an Android IR transmitter HAL or kernel node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IrTimingUnit {
    /// Standard Android `ConsumerIrManager` contract: alternating mark/space durations in microseconds.
    #[default]
    Microseconds,
    /// Cycle-register I2C microcontroller controllers (`ir_remote_con_mc96` on Exynos 4412).
    CarrierCycles,
    /// Period-scaled compensation (`ice4_fpga` on Exynos 5420 / SM-T805M): pre-multiplies by `1_000_000 / hz`.
    PeriodScaled,
}

/// Inclusive carrier frequency range reported by an Android IR HAL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrFrequencyRange {
    pub min_hz: u32,
    pub max_hz: u32,
}

impl IrFrequencyRange {
    #[must_use]
    pub const fn contains(self, frequency_hz: u32) -> bool {
        frequency_hz >= self.min_hz && frequency_hz <= self.max_hz
    }
}

/// Host hardware metadata passed from Android `Build.*` for driver quirk detection.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IrDeviceInfo {
    pub manufacturer: String,
    pub model: String,
    pub hardware: String,
    pub board: String,
    pub product: String,
    pub device: String,
    pub android_release: String,
    pub sdk_int: u32,
}

/// Prepared IR transmission payload computed in Rust SSOT for either Android CIR HAL or Pico `IRTX`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrPreparedSignal {
    pub carrier_hz: u32,
    pub pattern_us: Vec<u32>,
    pub encoded_pattern: Vec<u32>,
    pub sysfs_payload: String,
    pub expects_cycles_in_sysfs: bool,
    pub recommended_unit: IrTimingUnit,
    pub effective_unit: IrTimingUnit,
    pub wire_hex: String,
    pub pronto_hex: String,
}

/// Hardware device categories inside the offline All-Panasonic IR catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PanasonicCategory {
    #[default]
    All,
    Diga,
    Combo,
    Vcr,
    Dvd,
    Tv,
    Audio,
    Stb,
    Proj,
    Ac,
    Other,
}

impl PanasonicCategory {
    #[must_use]
    pub fn from_code(code: &str) -> Self {
        match code.trim().to_ascii_uppercase().as_str() {
            "ALL" => Self::All,
            "DIGA" => Self::Diga,
            "COMBO" => Self::Combo,
            "VCR" => Self::Vcr,
            "DVD" => Self::Dvd,
            "TV" => Self::Tv,
            "AUDIO" => Self::Audio,
            "STB" => Self::Stb,
            "PROJ" => Self::Proj,
            "AC" => Self::Ac,
            _ => Self::Other,
        }
    }
}

/// Logical layout section for auto-assembled dynamic remote keymaps.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KeySection {
    PowerInput,
    Navigation,
    Transport,
    TuningAudio,
    Numeric,
    RecordingEdit,
    #[default]
    Custom,
}

/// Underlying IR encoding protocol of a remote button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KeyProtocol {
    #[default]
    Kaseikyo,
    ProntoRaw,
}

/// Semantic button visual accent aligned with Tailwind/shadcn UI variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KeyTone {
    #[default]
    Neutral,
    Primary,
    Info,
    Success,
    Warning,
    Danger,
    DangerDark,
    Accent,
}

/// Normalized IR command entry loaded from `panasonic_ir_catalog.dat.gz` or imported `.ir`/`.irplus`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogCommandSpec {
    pub name: String,
    pub protocol: KeyProtocol,
    pub device: u8,
    pub subdevice: u8,
    pub obc: u8,
    pub pronto_hex: Option<String>,
}

/// Deduplicated hardware model codeset grouping models sharing an IR bank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanasonicCatalogCodeset {
    pub id: String,
    pub category: PanasonicCategory,
    pub models: Vec<String>,
    pub commands: Vec<CatalogCommandSpec>,
}

/// Concise codeset search hit serialized across Tauri IPC without shipping large command arrays prematurely.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanasonicCatalogSummary {
    pub id: String,
    pub category: PanasonicCategory,
    pub primary_name: String,
    pub models: Vec<String>,
    pub command_count: usize,
}

/// Runtime-editable button inside an assembled or imported remote profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicRemoteKey {
    pub id: String,
    pub label: String,
    pub section: KeySection,
    pub protocol: KeyProtocol,
    pub device: u8,
    pub subdevice: u8,
    pub obc: u8,
    pub respect_remote_mode: bool,
    pub pronto_hex: Option<String>,
    pub tone: KeyTone,
    pub confirm_required: bool,
    pub wire_preview: String,
}

/// Editable remote profile returned to Android DIGA Commander UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicKeymapProfile {
    pub id: String,
    pub name: String,
    pub source_codeset_id: String,
    pub category: PanasonicCategory,
    pub keys: Vec<DynamicRemoteKey>,
    pub is_user_modified: bool,
}

/// Declarative safety delay specification matching verified hardware service steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrMacroStep {
    pub label: String,
    pub device: u8,
    pub subdevice: u8,
    pub obc: u8,
    pub hold_ms: u32,
    pub post_delay_ms: u32,
}

/// Risk classification matching frontend safety confirmation dialog behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IrMacroRiskLevel {
    #[default]
    Safe,
    Moderate,
    Destructive,
}

/// Verified multi-step IR service/recovery sequence loaded declaratively from SSOT JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedIrMacro {
    pub id: String,
    pub title_pt: String,
    pub title_en: String,
    pub description_pt: String,
    pub description_en: String,
    pub risk: IrMacroRiskLevel,
    pub verification_source: String,
    pub steps: Vec<IrMacroStep>,
}
