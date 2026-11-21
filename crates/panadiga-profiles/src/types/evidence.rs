#![deny(unsafe_code)]

//! Evidence tiers, provenance claims, hardware capabilities, runtime fingerprints, and audited lab overrides.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Operator acknowledgment literal required to unlock unverified offline Copy-on-Write (`LabOverride`).
pub const LAB_OVERRIDE_DISCLAIMER_ACK: &str = "I_ACCEPT_UNVERIFIED_HARDWARE_RISK";

/// Empirical confidence tier attached to each individual hardware/firmware claim.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTier {
    /// T0: Forum report, size heuristic, or unverified note (C0 read-only unless offline LabOverride).
    CommunityUnverified = 0,
    /// T1: Verified against official Matsushita/Panasonic service manuals or schematics (no binary dump).
    ServiceManualOnly = 1,
    /// T2: Verified from an official `.FRM` / CDN firmware package (proves train/TOC/load VA, not NOR layout).
    OfficialFrmVerified = 2,
    /// T3: Verified from a full physical chip dump with SHA-256 and static layout analysis.
    PhysicalDumpVerified = 3,
    /// T4: Operated and verified live on bench hardware with captured telemetry and before/after hashes.
    BenchVerified = 4,
}

impl EvidenceTier {
    /// Stable short tag (`T0`..`T4`).
    #[must_use]
    pub const fn short_tag(self) -> &'static str {
        match self {
            Self::CommunityUnverified => "T0",
            Self::ServiceManualOnly => "T1",
            Self::OfficialFrmVerified => "T2",
            Self::PhysicalDumpVerified => "T3",
            Self::BenchVerified => "T4",
        }
    }
}

/// Primary source reference justifying a claim in a hardware profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSource {
    pub kind: String,
    pub reference: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Evidence bundle bound to one architectural entity or model claim axis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimEvidence {
    pub tier: EvidenceTier,
    #[serde(default)]
    pub sources: Vec<EvidenceSource>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl ClaimEvidence {
    /// Helper constructor for unverified claims (`T0`).
    #[must_use]
    pub fn unverified(note: impl Into<String>) -> Self {
        Self {
            tier: EvidenceTier::CommunityUnverified,
            sources: Vec::new(),
            notes: Some(note.into()),
        }
    }
}

/// Discrete hardware/forensic actions gated by [`crate::gate::CapabilityGate`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// C0: Read-only analysis, parsing, entropy inspection, and sector breakdown.
    InspectReadOnly,
    /// C1: Offline Copy-on-Write erasure/patch confined to verified `Eemul` NVRAM microsectors.
    NvramSanitizeOfflineCow,
    /// C2: Live P7501 Pico in-circuit write to writable `Eemul` microsectors.
    NvramWriteLive,
    /// C3a: Offline Copy-on-Write patch in Upper NOR (`Bgo` / `NorFs`).
    UpperNorPatchOfflineCow,
    /// C3b: Live P7501 Pico write in Upper NOR (`Bgo` / `NorFs`).
    UpperNorWriteLive,
    /// C4a: Bit-exact official `.FRM` pass-through optical/SD media synthesis.
    OfficialFrmPassThrough,
    /// C4b: Custom or patched recovery media synthesis (`DIGADIAG.FRM`, `PANA__SD.FRM`, custom homebrew).
    CustomRecoveryMedia,
    /// C5: Critical IPL boot sector write or forbidden MMIO (`0xC000_100C`) access — ALWAYS DENIED.
    CriticalIplOrLethalMmio,
}

/// Explicit, auditable operator override unlocking offline CoW or custom media synthesis for R&D.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LabOverride {
    pub disclaimer_ack: String,
    pub operator_reason: String,
}

/// Runtime probes passed into [`crate::gate::CapabilityGate::authorize`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeFingerprint {
    #[serde(default)]
    pub dump_size_bytes: Option<u64>,
    #[serde(default)]
    pub jedec_mfg_id: Option<u32>,
    #[serde(default)]
    pub jedec_device_cycles: Vec<u32>,
    #[serde(default)]
    pub loader_banner: Option<String>,
    #[serde(default)]
    pub has_eemul_f1_marker: bool,
    #[serde(default)]
    pub wp_pin_deasserted: Option<bool>,
    #[serde(default)]
    pub frm_sha256: Option<String>,
    #[serde(default)]
    pub requested_loader_profile: Option<String>,
}

/// Authorization mode granted by [`crate::gate::CapabilityGate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationMode {
    VerifiedPolicy,
    LabOverrideUnverified,
}

/// Affirmative capability decision returned with provenance metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Authorization {
    pub model_id: String,
    pub capability: Capability,
    pub mode: AuthorizationMode,
    pub effective_tier: EvidenceTier,
    pub warnings: Vec<String>,
}

/// Fail-closed denial returned when claims, probes, or invariants do not support the action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Denial {
    pub model_id: String,
    pub capability: Capability,
    pub reason: String,
    pub lab_override_eligible: bool,
}
