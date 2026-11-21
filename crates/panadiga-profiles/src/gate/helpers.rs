#![deny(unsafe_code)]

//! Validation helpers for [`super::CapabilityGate`].

use crate::types::{
    Authorization, AuthorizationMode, Capability, Denial, EvidenceTier, FirmwareTrainProfile,
    LAB_OVERRIDE_DISCLAIMER_ACK, LabOverride, NorRegionKind, NorRegionPolicy, ResolvedModelProfile,
    RuntimeFingerprint,
};

pub(super) fn valid_lab_override(ov: Option<&LabOverride>) -> Result<Vec<String>, &'static str> {
    let Some(ov) = ov else {
        return Err("No LabOverride supplied");
    };
    if ov.disclaimer_ack.trim() != LAB_OVERRIDE_DISCLAIMER_ACK {
        return Err("LabOverride missing exact acknowledgment 'I_ACCEPT_UNVERIFIED_HARDWARE_RISK'");
    }
    if ov.operator_reason.trim().len() < 5 {
        return Err("LabOverride requires non-empty operator_reason (>=5 chars)");
    }
    Ok(vec![format!(
        "UNVERIFIED LAB OVERRIDE ACTIVE [{}]: custom R&D CoW/media output carries no bench guarantee; live hardware writes remain locked.",
        ov.operator_reason.trim()
    )])
}

pub(super) fn deny(
    resolved: &ResolvedModelProfile,
    cap: Capability,
    reason: impl Into<String>,
    lab_override_eligible: bool,
) -> Denial {
    Denial {
        model_id: resolved.manifest.model_id.clone(),
        capability: cap,
        reason: reason.into(),
        lab_override_eligible,
    }
}

pub(super) fn verify_dump_size_and_marker(
    resolved: &ResolvedModelProfile,
    cap: Capability,
    fp: &RuntimeFingerprint,
    need_eemul_marker: bool,
) -> Result<(), Denial> {
    if let (Some(chip), Some(dump_sz)) = (&resolved.flash_chip, fp.dump_size_bytes)
        && dump_sz != u64::from(chip.total_bytes)
    {
        return Err(deny(
            resolved,
            cap,
            format!(
                "Dump size ({dump_sz} B) mismatches declared flash '{}' ({} B)",
                chip.part_number, chip.total_bytes
            ),
            false,
        ));
    }
    if need_eemul_marker
        && let Some(layout) = &resolved.nor_layout
        && let Some(eemul) = layout.find_region(NorRegionKind::Eemul)
        && eemul.signature.is_some()
        && !fp.has_eemul_f1_marker
        && fp.dump_size_bytes.is_some()
    {
        return Err(deny(
            resolved,
            cap,
            "Dump lacks EEMUL 0xF1 sector marker required by layout signature (refusing blind tail erase)",
            true,
        ));
    }
    Ok(())
}

pub(super) fn verify_jedec_and_wp(
    resolved: &ResolvedModelProfile,
    cap: Capability,
    fp: &RuntimeFingerprint,
) -> Result<(), Denial> {
    let Some(chip) = &resolved.flash_chip else {
        return Err(deny(resolved, cap, "Flash chip unknown", false));
    };
    if !chip.writable {
        return Err(deny(
            resolved,
            cap,
            format!("Flash '{}' is read-only OTP PROM", chip.part_number),
            false,
        ));
    }
    let Some(jedec) = &chip.jedec else {
        return Err(deny(
            resolved,
            cap,
            format!(
                "Flash '{}' lacks verified JEDEC ID cycles",
                chip.part_number
            ),
            false,
        ));
    };
    if let Some(mfg) = fp.jedec_mfg_id
        && mfg != jedec.mfg
    {
        return Err(deny(
            resolved,
            cap,
            format!(
                "Live JEDEC mfg 0x{mfg:02X} mismatches profile expected 0x{:02X}",
                jedec.mfg
            ),
            false,
        ));
    }
    if !fp.jedec_device_cycles.is_empty() && fp.jedec_device_cycles != jedec.device_cycles {
        return Err(deny(
            resolved,
            cap,
            format!(
                "Live JEDEC device ID {:04X?} mismatches profile {:04X?}",
                fp.jedec_device_cycles, jedec.device_cycles
            ),
            false,
        ));
    }
    if let Some(wp) = &chip.hw_protect
        && wp.affects_top_sector
        && fp.wp_pin_deasserted != Some(true)
    {
        return Err(deny(
            resolved,
            cap,
            format!(
                "Flash '{}' hardware protect pin '{}' covers top sector ('{}'); must verify wp_pin_deasserted=true before live write",
                chip.part_number, wp.pin, wp.protects_region
            ),
            false,
        ));
    }
    Ok(())
}

pub(super) fn verified_auth(
    resolved: &ResolvedModelProfile,
    cap: Capability,
    effective_tier: EvidenceTier,
) -> Authorization {
    Authorization {
        model_id: resolved.manifest.model_id.clone(),
        capability: cap,
        mode: AuthorizationMode::VerifiedPolicy,
        effective_tier,
        warnings: Vec::new(),
    }
}

pub(super) fn allow_override_if_eligible(
    resolved: &ResolvedModelProfile,
    cap: Capability,
    policy_denial: Denial,
    lab_override: Option<&LabOverride>,
) -> Result<Authorization, Denial> {
    if !policy_denial.lab_override_eligible {
        return Err(policy_denial);
    }
    let writable_ok = resolved.flash_chip.as_ref().is_none_or(|c| c.writable);
    if !writable_ok {
        return Err(policy_denial);
    }
    let warnings = match valid_lab_override(lab_override) {
        Ok(w) => w,
        Err(_) => return Err(policy_denial),
    };
    let eff = resolved
        .manifest
        .claims
        .nor_layout
        .tier
        .min(resolved.manifest.claims.train.tier);
    Ok(Authorization {
        model_id: resolved.manifest.model_id.clone(),
        capability: cap,
        mode: AuthorizationMode::LabOverrideUnverified,
        effective_tier: eff,
        warnings,
    })
}

pub(super) fn finish_verified_or_override(
    resolved: &ResolvedModelProfile,
    cap: Capability,
    res: Result<EvidenceTier, Denial>,
    lab_override: Option<&LabOverride>,
) -> Result<Authorization, Denial> {
    res.map(|eff| verified_auth(resolved, cap, eff))
        .or_else(|d| allow_override_if_eligible(resolved, cap, d, lab_override))
}

pub(super) fn authorize_live_nor(
    resolved: &ResolvedModelProfile,
    cap: Capability,
    fp: &RuntimeFingerprint,
    eff: EvidenceTier,
    min_req: bool,
    err_msg: String,
    require_eemul: bool,
) -> Result<Authorization, Denial> {
    if !min_req {
        return Err(deny(resolved, cap, err_msg, false));
    }
    if require_eemul
        && resolved
            .nor_layout
            .as_ref()
            .and_then(|l| l.find_region(NorRegionKind::Eemul))
            .is_none()
    {
        return Err(deny(resolved, cap, "No EEMUL region declared", false));
    }
    verify_jedec_and_wp(resolved, cap, fp)?;
    Ok(verified_auth(resolved, cap, eff))
}

pub(super) fn require_verified_train<'a>(
    resolved: &'a ResolvedModelProfile,
    cap: Capability,
    min_tier: EvidenceTier,
    label: &str,
) -> Result<(&'a FirmwareTrainProfile, EvidenceTier), Denial> {
    let train = resolved
        .firmware_train
        .as_ref()
        .ok_or_else(|| deny(resolved, cap, "Firmware train unknown", true))?;
    let tier = resolved.manifest.claims.train.tier;
    if tier < min_tier {
        return Err(deny(
            resolved,
            cap,
            format!(
                "{label} requires train >= {} (got {})",
                min_tier.short_tag(),
                tier.short_tag()
            ),
            true,
        ));
    }
    Ok((train, tier))
}

pub(super) fn eval_nvram_sanitize_offline(
    resolved: &ResolvedModelProfile,
    cap: Capability,
    fp: &RuntimeFingerprint,
) -> Result<EvidenceTier, Denial> {
    let claims = &resolved.manifest.claims;
    let chip = resolved.flash_chip.as_ref().ok_or_else(|| {
        deny(
            resolved,
            cap,
            "Flash chip geometry not established for model",
            true,
        )
    })?;
    if !chip.writable {
        return Err(deny(
            resolved,
            cap,
            "Flash chip is non-writable OTP PROM",
            false,
        ));
    }
    let layout = resolved.nor_layout.as_ref().ok_or_else(|| {
        deny(
            resolved,
            cap,
            "NOR layout unknown; cannot locate EEMUL NVRAM",
            true,
        )
    })?;
    let eemul = layout.find_region(NorRegionKind::Eemul).ok_or_else(|| {
        deny(
            resolved,
            cap,
            "Model NOR layout has NO Panasonic EEMUL NVRAM partition (e.g. non-MN103S / vendor NV)",
            false,
        )
    })?;
    if eemul.policy != NorRegionPolicy::NvramSanitize {
        return Err(deny(
            resolved,
            cap,
            "EEMUL policy does not permit NvramSanitize",
            false,
        ));
    }
    let eff = claims.flash_chip.tier.min(claims.nor_layout.tier);
    if eff < EvidenceTier::PhysicalDumpVerified {
        return Err(deny(
            resolved,
            cap,
            format!(
                "Insufficient provenance {} for offline NVRAM sanitize (requires >= T3 physical_dump_verified)",
                eff.short_tag()
            ),
            true,
        ));
    }
    verify_dump_size_and_marker(resolved, cap, fp, true)?;
    Ok(eff)
}

pub(super) fn eval_upper_nor_offline(
    resolved: &ResolvedModelProfile,
    cap: Capability,
    fp: &RuntimeFingerprint,
) -> Result<EvidenceTier, Denial> {
    let claims = &resolved.manifest.claims;
    let layout = resolved
        .nor_layout
        .as_ref()
        .ok_or_else(|| deny(resolved, cap, "NOR layout unknown", true))?;
    let has_upper = layout.regions.iter().any(|r| {
        matches!(r.kind, NorRegionKind::Bgo | NorRegionKind::NorFs)
            && r.policy == NorRegionPolicy::GatedUpperNor
    });
    if !has_upper {
        return Err(deny(
            resolved,
            cap,
            "NOR layout lacks GatedUpperNor (BGO/NOR-FS) writable span",
            true,
        ));
    }
    let eff = claims.nor_layout.tier.min(claims.flash_chip.tier);
    if eff < EvidenceTier::PhysicalDumpVerified {
        return Err(deny(
            resolved,
            cap,
            format!(
                "Upper NOR CoW patch requires >= T3 physical_dump_verified (got {})",
                eff.short_tag()
            ),
            true,
        ));
    }
    verify_dump_size_and_marker(resolved, cap, fp, false)?;
    Ok(eff)
}
