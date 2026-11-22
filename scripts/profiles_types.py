"""Strict Python 3.10+ TypedDict & Literal mirror of `profiles/schema/*.schema.json` (panadiga-profiles v2)."""

from __future__ import annotations

from typing import Literal, TypedDict

EvidenceTier = Literal[
    "community_unverified",
    "service_manual_only",
    "official_frm_verified",
    "physical_dump_verified",
    "bench_verified",
]

Capability = Literal[
    "inspect_read_only",
    "nvram_sanitize_offline_cow",
    "nvram_write_live",
    "upper_nor_patch_offline_cow",
    "upper_nor_write_live",
    "official_frm_pass_through",
    "custom_recovery_media",
    "critical_ipl_or_lethal_mmio",
]

AuthorizationMode = Literal["verified_policy", "lab_override_unverified"]


class _EvidenceSourceOptional(TypedDict, total=False):
    sha256: str | None
    notes: str | None


class EvidenceSourceDict(_EvidenceSourceOptional):
    kind: str
    reference: str


class _ClaimEvidenceOptional(TypedDict, total=False):
    sources: list[EvidenceSourceDict]
    notes: str | None


class ClaimEvidenceDict(_ClaimEvidenceOptional):
    tier: EvidenceTier


class ModelClaimsDict(TypedDict):
    board: ClaimEvidenceDict
    flash_chip: ClaimEvidenceDict
    nor_layout: ClaimEvidenceDict
    train: ClaimEvidenceDict
    eeprom: ClaimEvidenceDict


class _ModelProfileManifestOptional(TypedDict, total=False):
    region_suffix: str | None
    tv_standard: str | None
    hw_code: str | None
    board_ref: str | None
    train_ref: str | None
    nor_layout_ref: str | None
    exact_aliases: list[str]
    fixture_ref: str | None


class ModelProfileManifestDict(_ModelProfileManifestOptional):
    model_id: str
    family_label: str
    series: str
    claims: ModelClaimsDict


class _InterpolatedRegionVariantOptional(TypedDict, total=False):
    interpolatedHwCode: str | None


class InterpolatedRegionVariantDict(_InterpolatedRegionVariantOptional):
    regionCode: str
    tvStandard: str
    videoLineSystem: str
    aiofVmodeselHex: str
    dvdRegionCode: int
    oemMarker: str
    acMainsSpec: str
    tunerSystem: str


class RegistryValidationReportDict(TypedDict):
    valid: bool
    socCoreCount: int
    socCount: int
    flashChipCount: int
    eepromChipCount: int
    firmwareTrainCount: int
    norLayoutCount: int
    mediaFormatCount: int
    boardCount: int
    modelCount: int
    checksRun: int
    checksFailed: int
    safetyChecksPassed: int
    errors: list[str]
