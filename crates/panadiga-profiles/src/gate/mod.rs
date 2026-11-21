#![deny(unsafe_code)]

//! Policy-gated authorization engine (`CapabilityGate`) evaluating claim provenance,
//! runtime hardware fingerprints, silicon write-protection pins, and explicit audited lab overrides.

mod helpers;

use crate::types::{
    Authorization, AuthorizationMode, Capability, CapabilitySummary, Denial, EvidenceTier,
    LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride, ResolvedModelProfile, RuntimeFingerprint,
};
use helpers::{
    authorize_live_nor, deny, eval_nvram_sanitize_offline, eval_upper_nor_offline,
    finish_verified_or_override, require_verified_train,
};

/// Stateless capability gate deciding whether a specific action is safe on a resolved DIGA profile.
pub struct CapabilityGate;

impl CapabilityGate {
    /// Evaluates fail-closed authorization for `cap` on a resolved model manifest + runtime probes.
    pub fn authorize(
        resolved: &ResolvedModelProfile,
        cap: Capability,
        fp: &RuntimeFingerprint,
        lab_override: Option<&LabOverride>,
    ) -> Result<Authorization, Denial> {
        let claims = &resolved.manifest.claims;
        match cap {
            Capability::CriticalIplOrLethalMmio => Err(deny(
                resolved,
                cap,
                "Layer 1 Critical Boot IPL (>=256 KiB RO) and lethal MMIO 0xC000100C are permanently locked across all tiers and overrides",
                false,
            )),
            Capability::InspectReadOnly => Ok(Authorization {
                model_id: resolved.manifest.model_id.clone(),
                capability: cap,
                mode: AuthorizationMode::VerifiedPolicy,
                effective_tier: EvidenceTier::CommunityUnverified,
                warnings: Vec::new(),
            }),
            Capability::NvramSanitizeOfflineCow => {
                let res = eval_nvram_sanitize_offline(resolved, cap, fp);
                finish_verified_or_override(resolved, cap, res, lab_override)
            }
            Capability::NvramWriteLive => {
                let eff = claims
                    .board
                    .tier
                    .min(claims.nor_layout.tier)
                    .min(claims.flash_chip.tier);
                let ok = claims.board.tier >= EvidenceTier::BenchVerified
                    && claims.nor_layout.tier >= EvidenceTier::PhysicalDumpVerified
                    && claims.flash_chip.tier >= EvidenceTier::PhysicalDumpVerified;
                authorize_live_nor(
                    resolved,
                    cap,
                    fp,
                    eff,
                    ok,
                    format!(
                        "Live P7501 NVRAM write locked: requires board>=T4, flash>=T3, nor_layout>=T3 (got board={}, flash={}, layout={})",
                        claims.board.tier.short_tag(),
                        claims.flash_chip.tier.short_tag(),
                        claims.nor_layout.tier.short_tag(),
                    ),
                    true,
                )
            }
            Capability::UpperNorPatchOfflineCow => {
                let res = eval_upper_nor_offline(resolved, cap, fp);
                finish_verified_or_override(resolved, cap, res, lab_override)
            }
            Capability::UpperNorWriteLive => {
                let eff = claims
                    .board
                    .tier
                    .min(claims.flash_chip.tier)
                    .min(claims.nor_layout.tier)
                    .min(claims.train.tier);
                authorize_live_nor(
                    resolved,
                    cap,
                    fp,
                    eff,
                    eff >= EvidenceTier::BenchVerified,
                    format!(
                        "Live Upper NOR write requires ALL claims at T4 bench_verified (effective={})",
                        eff.short_tag()
                    ),
                    false,
                )
            }
            Capability::OfficialFrmPassThrough => {
                let res = (|| -> Result<EvidenceTier, Denial> {
                    let (train, tier) = require_verified_train(
                        resolved,
                        cap,
                        EvidenceTier::OfficialFrmVerified,
                        "Official FRM pass-through",
                    )?;
                    if let (Some(expected), Some(got)) = (
                        train
                            .frm
                            .as_ref()
                            .and_then(|f| f.official_frm_sha256.as_ref()),
                        fp.frm_sha256.as_ref(),
                    ) && !expected.eq_ignore_ascii_case(got)
                    {
                        return Err(deny(
                            resolved,
                            cap,
                            format!(
                                "Official FRM SHA-256 '{got}' mismatches manifest '{expected}'"
                            ),
                            false,
                        ));
                    }
                    Ok(tier)
                })();
                finish_verified_or_override(resolved, cap, res, lab_override)
            }
            Capability::CustomRecoveryMedia => {
                let res = (|| -> Result<EvidenceTier, Denial> {
                    let (train, tier) = require_verified_train(
                        resolved,
                        cap,
                        EvidenceTier::BenchVerified,
                        "Custom/patched recovery media (use LabOverride for R&D builds)",
                    )?;
                    if let Some(req_prof) = &fp.requested_loader_profile
                        && !req_prof.eq_ignore_ascii_case(&train.loader_profile_key)
                    {
                        return Err(deny(
                            resolved,
                            cap,
                            format!(
                                "Requested loader_profile '{req_prof}' conflicts with train '{}'",
                                train.loader_profile_key
                            ),
                            true,
                        ));
                    }
                    Ok(tier)
                })();
                finish_verified_or_override(resolved, cap, res, lab_override)
            }
        }
    }

    /// Computes static UI/API capability matrix for a resolved profile.
    #[must_use]
    pub fn summarize(resolved: &ResolvedModelProfile) -> CapabilitySummary {
        let empty_fp = RuntimeFingerprint::default();
        let wp_blocked_fp = RuntimeFingerprint {
            wp_pin_deasserted: Some(false),
            ..Default::default()
        };
        let lab = LabOverride {
            disclaimer_ack: LAB_OVERRIDE_DISCLAIMER_ACK.to_string(),
            operator_reason: "capability_probe".to_string(),
        };
        let can = |cap, fp, ov| Self::authorize(resolved, cap, fp, ov).is_ok();
        CapabilitySummary {
            inspect_read_only: true,
            nvram_sanitize_offline_cow: can(Capability::NvramSanitizeOfflineCow, &empty_fp, None),
            nvram_write_live: can(Capability::NvramWriteLive, &wp_blocked_fp, None),
            upper_nor_patch_offline_cow: can(Capability::UpperNorPatchOfflineCow, &empty_fp, None),
            upper_nor_write_live: can(Capability::UpperNorWriteLive, &wp_blocked_fp, None),
            official_frm_pass_through: can(Capability::OfficialFrmPassThrough, &empty_fp, None),
            custom_recovery_media: can(Capability::CustomRecoveryMedia, &empty_fp, None),
            lab_override_offline_cow_eligible: resolved
                .flash_chip
                .as_ref()
                .is_none_or(|c| c.writable)
                && (can(Capability::NvramSanitizeOfflineCow, &empty_fp, Some(&lab))
                    || can(Capability::UpperNorPatchOfflineCow, &empty_fp, Some(&lab))
                    || can(Capability::CustomRecoveryMedia, &empty_fp, Some(&lab))),
        }
    }
}
