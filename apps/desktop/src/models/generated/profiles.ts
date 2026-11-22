/* AUTO-ALIGNED WITH profiles/schema/*.schema.json (panadiga-profiles v2). Do not hand-invent v1 fields. */

export type EvidenceTier =
  | 'community_unverified'
  | 'service_manual_only'
  | 'official_frm_verified'
  | 'physical_dump_verified'
  | 'bench_verified';

export type Capability =
  | 'inspect_read_only'
  | 'nvram_sanitize_offline_cow'
  | 'nvram_write_live'
  | 'upper_nor_patch_offline_cow'
  | 'upper_nor_write_live'
  | 'official_frm_pass_through'
  | 'custom_recovery_media'
  | 'critical_ipl_or_lethal_mmio';

export type AuthorizationMode = 'verified_policy' | 'lab_override_unverified';

export type EepromFunction =
  | 'optical_calibration'
  | 'timer_main_config'
  | 'system_settings'
  | 'unknown';

export type NorRegionKind =
  | 'ipl'
  | 'erased'
  | 'extrhead'
  | 'main_xip'
  | 'bgo'
  | 'nor_fs'
  | 'eemul'
  | 'vendor_nvram'
  | 'unknown';

export type NorRegionPolicy =
  | 'read_only_permanent'
  | 'gated_upper_nor'
  | 'nvram_sanitize'
  | 'read_only_reference';

export interface EvidenceSource {
  readonly kind: string;
  readonly reference: string;
  readonly sha256?: string | null;
  readonly notes?: string | null;
}

export interface ClaimEvidence {
  readonly tier: EvidenceTier;
  readonly sources?: readonly EvidenceSource[];
  readonly notes?: string | null;
}

export interface LabOverride {
  readonly disclaimerAck: string;
  readonly operatorReason: string;
}

export interface RuntimeFingerprint {
  readonly dumpSizeBytes?: number | null;
  readonly jedecMfgId?: number | null;
  readonly jedecDeviceCycles?: readonly number[];
  readonly loaderBanner?: string | null;
  readonly hasEemulF1Marker?: boolean;
  readonly wpPinDeasserted?: boolean | null;
  readonly frmSha256?: string | null;
  readonly requestedLoaderProfile?: string | null;
}

export interface Authorization {
  readonly modelId: string;
  readonly capability: Capability;
  readonly mode: AuthorizationMode;
  readonly effectiveTier: EvidenceTier;
  readonly warnings: readonly string[];
}

export interface Denial {
  readonly modelId: string;
  readonly capability: Capability;
  readonly reason: string;
  readonly labOverrideEligible: boolean;
}

export interface ProfileAuthorizationOutcome {
  readonly allowed: boolean;
  readonly authorization: Authorization | null;
  readonly denial: Denial | null;
}

export interface SocCoreProfile {
  readonly id: string;
  readonly isa: string;
  readonly endianness: string;
  readonly logic_voltage_mv: number;
  readonly ghidra_language_id?: string | null;
  readonly ghidra_compiler_id?: string | null;
  readonly ghidra_sleigh_provider?: string | null;
  readonly default_base_address?: string | null;
  readonly evidence: ClaimEvidence;
}

export interface BusWindow {
  readonly name: string;
  readonly base: string;
  readonly size: string;
  readonly uncached_alias_base?: string | null;
}

export interface SocProfile {
  readonly id: string;
  readonly part_number: string;
  readonly core_ref: string;
  readonly bus_windows: readonly BusWindow[];
  readonly mmio?: Readonly<Record<string, string>>;
  readonly lethal_mmio?: readonly string[];
  readonly p7501_uart_baud: number;
  readonly p7501_nsw5v_hiz_only: boolean;
  readonly evidence: ClaimEvidence;
}

export interface FlashJedecId {
  readonly mfg: string;
  readonly device_cycles: readonly string[];
}

export interface FlashSectorBank {
  readonly name: string;
  readonly count: number;
  readonly sector_size: string;
}

export interface FlashRwBank {
  readonly start_sa: number;
  readonly end_sa: number;
}

export interface FlashHwProtect {
  readonly pin: string;
  readonly protects_region: string;
  readonly affects_top_sector: boolean;
}

export interface FlashChipProfile {
  readonly id: string;
  readonly vendor: string;
  readonly part_number: string;
  readonly package: string;
  readonly voltage_min_mv: number;
  readonly voltage_max_mv: number;
  readonly nominal_voltage_mv: number;
  readonly writable: boolean;
  readonly total_bytes: string;
  readonly jedec?: FlashJedecId | null;
  readonly sector_banks?: readonly FlashSectorBank[];
  readonly rw_banks?: readonly FlashRwBank[];
  readonly hw_protect?: FlashHwProtect | null;
  readonly evidence: ClaimEvidence;
}

export interface EepromChipProfile {
  readonly id: string;
  readonly vendor: string;
  readonly part_number: string;
  readonly service_replacement_part?: string | null;
  readonly bus: string;
  readonly function: EepromFunction;
  readonly i2c_addr_7bit: string;
  readonly total_bytes: string;
  readonly page_size: number;
  readonly voltage_mv: number;
  readonly evidence: ClaimEvidence;
}

export interface LoaderFingerprint {
  readonly offset_start: string;
  readonly offset_end: string;
  readonly ascii_marker: string;
}

export interface FrmContainerSpec {
  readonly toc_entry_bytes?: number | null;
  readonly toc_size?: string | null;
  readonly cipher?: string | null;
  readonly key_id?: string | null;
  readonly official_frm_sha256?: string | null;
}

export type DigadiagTrustModel = 'checksum32_only' | 'encrypted_toc' | 'rsa_signed' | 'unknown';

export interface DigadiagSpec {
  readonly format: string;
  readonly filename: string;
  readonly exact_bytes: number;
  readonly load_va: string;
  readonly entry_va: string;
  readonly panamon1_entry_va?: string | null;
  readonly bgo_geometry_target: string;
  readonly fvu_decoupled_mode: string;
  readonly sd_media_requirement: string;
  readonly trust_model?: DigadiagTrustModel | null;
}

export interface FirmwareTrainProfile {
  readonly id: string;
  readonly soc_core_ref?: string | null;
  readonly loader_version: string;
  readonly loader_date?: string | null;
  readonly loader_profile_key: string;
  readonly loader_fingerprint?: LoaderFingerprint | null;
  readonly frm?: FrmContainerSpec | null;
  readonly prog_load_va?: string | null;
  readonly main_load_va?: string | null;
  readonly drv_load_va?: string | null;
  readonly symbols?: Readonly<Record<string, string>>;
  readonly sd_boot_filenames?: readonly string[];
  readonly digadiag_raw_bytes?: number | null;
  readonly media_refs?: readonly string[];
  readonly digadiag_spec?: DigadiagSpec | null;
  readonly evidence: ClaimEvidence;
}

export interface RegionSignature {
  readonly marker_byte_offset: number;
  readonly marker_value: string;
}

export interface NorRegion {
  readonly kind: NorRegionKind;
  readonly start_offset: string;
  readonly end_offset: string;
  readonly policy: NorRegionPolicy;
  readonly tag?: string | null;
  readonly signature?: RegionSignature | null;
}

export interface NorLayoutProfile {
  readonly id: string;
  readonly flash_ref: string;
  readonly train_ref: string;
  readonly regions: readonly NorRegion[];
  readonly evidence: ClaimEvidence;
}

export interface MediaFormatProfile {
  readonly id: string;
  readonly media_kind: string;
  readonly filesystem: string;
  readonly partition_type: string;
  readonly sector_size: number;
  readonly start_lba: number;
  readonly default_volume_mb: number;
  readonly root_filenames: readonly string[];
  readonly magic_hex: string;
  readonly write_speed_policy: string;
  readonly evidence: ClaimEvidence;
}

export interface BoardProfile {
  readonly id: string;
  readonly pcb_code: string;
  readonly service_module_code?: string | null;
  readonly soc_ref?: string | null;
  readonly flash_ref?: string | null;
  readonly eeprom_refs?: readonly string[];
  readonly p7501_header_present: boolean;
  readonly has_hdd_interface: boolean;
  readonly has_sd_slot: boolean;
  readonly evidence: ClaimEvidence;
}

export interface ModelClaims {
  readonly board: ClaimEvidence;
  readonly flash_chip: ClaimEvidence;
  readonly nor_layout: ClaimEvidence;
  readonly train: ClaimEvidence;
  readonly eeprom: ClaimEvidence;
}

export interface ModelProfileManifest {
  readonly model_id: string;
  readonly family_label: string;
  readonly series: string;
  readonly region_suffix?: string | null;
  readonly tv_standard?: string | null;
  readonly hw_code?: string | null;
  readonly board_ref?: string | null;
  readonly train_ref?: string | null;
  readonly nor_layout_ref?: string | null;
  readonly exact_aliases?: readonly string[];
  readonly fixture_ref?: string | null;
  readonly claims: ModelClaims;
}

export interface CapabilitySummary {
  readonly inspectReadOnly: boolean;
  readonly nvramSanitizeOfflineCow: boolean;
  readonly nvramWriteLive: boolean;
  readonly upperNorPatchOfflineCow: boolean;
  readonly upperNorWriteLive: boolean;
  readonly officialFrmPassThrough: boolean;
  readonly customRecoveryMedia: boolean;
  readonly labOverrideOfflineCowEligible: boolean;
}

export interface InterpolatedRegionVariant {
  readonly regionCode: string;
  readonly tvStandard: string;
  readonly videoLineSystem: string;
  readonly aiofVmodeselHex: string;
  readonly dvdRegionCode: number;
  readonly oemMarker: string;
  readonly acMainsSpec: string;
  readonly tunerSystem: string;
  readonly interpolatedHwCode?: string | null;
}

export interface ResolvedModelProfile {
  readonly manifest: ModelProfileManifest;
  readonly board: BoardProfile | null;
  readonly socCore: SocCoreProfile | null;
  readonly soc: SocProfile | null;
  readonly flashChip: FlashChipProfile | null;
  readonly eepromChips: readonly EepromChipProfile[];
  readonly firmwareTrain: FirmwareTrainProfile | null;
  readonly norLayout: NorLayoutProfile | null;
  readonly mediaFormats: readonly MediaFormatProfile[];
  readonly nvramCachedVaHex: string | null;
  readonly nvramUncachedVaHex: string | null;
  readonly iplProtectedRangeHex: string | null;
  readonly bgoSectorRangeDerived: string | null;
  readonly nvramSectorRangeDerived: string | null;
  readonly derivedSectorGeometry: string | null;
  readonly regionalVariants?: readonly InterpolatedRegionVariant[];
  readonly activeRegion?: InterpolatedRegionVariant | null;
  readonly effectiveCapabilities: CapabilitySummary;
}

export interface RegistryValidationReport {
  readonly valid: boolean;
  readonly socCoreCount: number;
  readonly socCount: number;
  readonly flashChipCount: number;
  readonly eepromChipCount: number;
  readonly firmwareTrainCount: number;
  readonly norLayoutCount: number;
  readonly mediaFormatCount: number;
  readonly boardCount: number;
  readonly modelCount: number;
  readonly checksRun: number;
  readonly checksFailed: number;
  readonly safetyChecksPassed: number;
  readonly errors: readonly string[];
}

export interface FingerprintCandidate {
  readonly modelId: string;
  readonly matchedProbe: string;
  readonly confidenceBps: number;
}

export interface NorFingerprintReport {
  readonly detectedModel: string | null;
  readonly displayLabel: string;
  readonly isSizeOnly: boolean;
  readonly matchedProbe: string | null;
  readonly hasEemulF1Marker: boolean;
  readonly dumpSizeBytes: number;
  readonly candidates: readonly FingerprintCandidate[];
}
