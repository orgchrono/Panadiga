/**
 * @file models/vault.ts
 * @brief Domain models for Strict Segregated Vaults, Mandatory Mn103Vm Pre-Write Gate,
 *        Optical ISO 9660 (1x) DVD Creator, Streaming Composed HDD Flash, & Relational Hardware Profiles.
 */

import type { SafeTargetDiskInfo } from './hddFlashModels';
import type { PicoUf2Variant } from './pico';
import type { SdLoaderProfile } from './unbrick';

export type VaultBucket = 'raw' | 'frm' | 'recompiled';

export type StrictWorkflowStep =
  | 'select_model'
  | 'build_cleanroom_sd'
  | 'setup_pico_probe'
  | 'capture_and_backup'
  | 'organize_and_recompile'
  | 'emulate_and_flash';

export interface PreWriteCheckItem {
  readonly id: string;
  readonly passed: boolean;
  readonly detail: string;
}

export interface PreWriteGateReport {
  readonly passed: boolean;
  readonly artifactKind: string;
  readonly artifactSha256: string;
  readonly iplUntouched: boolean;
  readonly lethalMmioBlocked: boolean;
  readonly executedSteps: number;
  readonly checks: readonly PreWriteCheckItem[];
  readonly notes: readonly string[];
}

export interface VaultArtifactRecord {
  readonly id: string;
  readonly bucket: VaultBucket;
  readonly filename: string;
  readonly relativePath: string;
  readonly sizeBytes: number;
  readonly sha256: string;
  readonly origin: string;
  readonly createdAtEpochS: number;
  readonly emuVerified: boolean;
}

export interface VaultSessionManifest {
  readonly sessionId: string;
  readonly modelId: string;
  readonly loaderProfile: SdLoaderProfile;
  readonly createdAtEpochS: number;
  readonly sessionDir: string;
  readonly completedSteps: readonly StrictWorkflowStep[];
  readonly currentStep: StrictWorkflowStep;
  readonly rawArtifacts: readonly VaultArtifactRecord[];
  readonly frmArtifacts: readonly VaultArtifactRecord[];
  readonly recompiledArtifacts: readonly VaultArtifactRecord[];
  readonly lastEmuGate: PreWriteGateReport | null;
}

export interface VaultInitSessionRequest {
  readonly modelId: string;
  readonly loaderProfile?: SdLoaderProfile;
  readonly sessionId?: string;
}

export interface VaultIngestFileRequest {
  readonly sessionId: string;
  readonly sourcePath: string;
  readonly bucketOverride?: VaultBucket;
  readonly origin?: string;
}

export interface RunPreWriteEmuGateRequest {
  readonly sessionId?: string;
  readonly targetKind:
    | 'cleanroom_digadiag'
    | 'composed_hdd_stream'
    | 'patched_prog061'
    | 'patched_main'
    | 'nor_cow_repair';
  readonly loaderProfile?: SdLoaderProfile;
  readonly artifactPath?: string;
  readonly totalSectors?: number;
  readonly expectPassthrough?: boolean;
  readonly expectCapacityUnlock?: boolean;
  readonly allowUpperNor?: boolean;
}

export interface FlashComposedHddSafeRequest {
  readonly devicePath: string;
  readonly confirmDiskModelToken: string;
  readonly backupDir?: string;
  readonly mainSourcePath?: string;
  readonly drvSourcePath?: string;
  readonly applyDynamicCapacityUnlock?: boolean;
  readonly targetCapacityGb?: number;
  readonly applyAvPassthrough?: boolean;
  readonly expectedMinGb?: number;
  readonly expectedMaxGb?: number;
  readonly allowExtended4tb?: boolean;
  readonly sessionId?: string;
  readonly nonceId?: string;
}

export interface ComposedHddFlashReport {
  readonly devicePath: string;
  readonly diskBytes: number;
  readonly totalSectors: number;
  readonly busTypeId: number;
  readonly mirrorMainLba: number;
  readonly backupBinPath: string;
  readonly backupManifestPath: string;
  readonly backup64kSha256: string;
  readonly layoutSha256: string;
  readonly p1MainBytesWritten: number;
  readonly p1MainSha256: string | null;
  readonly p2DrvBytesWritten: number;
  readonly p2DrvSha256: string | null;
  readonly mirrorMainBytesWritten: number;
  readonly mirrorMainSha256: string | null;
  readonly emuGate: PreWriteGateReport;
  readonly readbackVerified: boolean;
}

export type DvdCreatorStage =
  | 'synthesize_iso'
  | 'emu_gate_verify'
  | 'inspect_iso9660'
  | 'lockout_verify'
  | 'burn_optical1x'
  | 'readback_verify';

export interface DvdBurnProgressEvent {
  readonly target: string;
  readonly stage: DvdCreatorStage;
  readonly percent: number;
  readonly detail: string;
}

export interface OpticalBurnerDriveInfo {
  readonly deviceId: string;
  readonly displayName: string;
  readonly osBackend: string;
  readonly supportedWriteSpeedsKbps: readonly number[];
  readonly minWriteSpeedKbps: number;
  readonly minWriteSpeedLabel: string;
  readonly safeOpticalTarget: boolean;
  readonly expectedConfirmToken: string;
}

export interface Iso9660PlacedFileReport {
  readonly name_8_3: string;
  readonly extent_lba: number;
  readonly size_bytes: number;
  readonly sector_count: number;
  readonly sha256: string;
}

export interface Iso9660InspectionReport {
  readonly valid_pvd: boolean;
  readonly valid_terminator: boolean;
  readonly volume_id: string;
  readonly logical_block_size: number;
  readonly volume_space_sectors: number;
  readonly files: readonly Iso9660PlacedFileReport[];
  readonly overall_status: string;
}

export interface CreateDvdRecoveryDiscRequest {
  readonly burnerId?: string;
  readonly confirmToken?: string;
  readonly outputIsoPath?: string;
  readonly model?: string;
  readonly volumeLabel?: string;
  readonly loaderProfile?: SdLoaderProfile;
  readonly digadiagPath?: string;
  readonly panaDvdPath?: string;
  readonly cleanroomOnly?: boolean;
  readonly dryRun?: boolean;
  readonly sessionId?: string;
  readonly nonceId?: string;
}

export interface DvdCreatorReport {
  readonly target: string;
  readonly targetKind: string;
  readonly model: string;
  readonly volumeLabel: string;
  readonly cleanroomOnly: boolean;
  readonly bytesWritten: number;
  readonly isoSha256: string;
  readonly writeSpeedPolicy: string;
  readonly burnCommandPreview: string | null;
  readonly files: readonly Iso9660PlacedFileReport[];
  readonly emuGate: PreWriteGateReport;
  readonly inspection: Iso9660InspectionReport;
  readonly nextSteps: readonly string[];
}

import type { ResolvedModelProfile } from './generated/profiles';

export type {
  Authorization,
  AuthorizationMode,
  BoardProfile,
  BusWindow,
  Capability,
  CapabilitySummary,
  ClaimEvidence,
  Denial,
  EepromChipProfile,
  EepromFunction,
  EvidenceSource,
  EvidenceTier,
  FingerprintCandidate,
  FirmwareTrainProfile,
  FlashChipProfile,
  FlashHwProtect,
  FlashJedecId,
  FlashRwBank,
  FlashSectorBank,
  FrmContainerSpec,
  LabOverride,
  LoaderFingerprint,
  MediaFormatProfile,
  ModelClaims,
  ModelProfileManifest,
  NorFingerprintReport,
  NorLayoutProfile,
  NorRegion,
  NorRegionKind,
  NorRegionPolicy,
  ProfileAuthorizationOutcome,
  RegionSignature,
  RegistryValidationReport,
  ResolvedModelProfile,
  RuntimeFingerprint,
  SocCoreProfile,
  SocProfile,
} from './generated/profiles';

export interface StrictVaultMissionState {
  readonly loaderProfile: SdLoaderProfile;
  readonly setLoaderProfile: (profile: SdLoaderProfile) => void;
  readonly resolvedProfile: ResolvedModelProfile | null;
  readonly isLoading: boolean;
  readonly activeSession: VaultSessionManifest | null;
  readonly sessionBusy: boolean;
  readonly sessionError: string | null;
  readonly initializeVaultSession: () => Promise<void>;
  readonly opticalBurners: readonly OpticalBurnerDriveInfo[];
  readonly dvdTargetMode: 'iso' | 'burner';
  readonly setDvdTargetMode: (mode: 'iso' | 'burner') => void;
  readonly selectedBurnerId: string;
  readonly setSelectedBurnerId: (id: string) => void;
  readonly dvdConfirmToken: string;
  readonly setDvdConfirmToken: (tok: string) => void;
  readonly dvdBusy: boolean;
  readonly dvdProgress: DvdBurnProgressEvent | null;
  readonly dvdReport: DvdCreatorReport | null;
  readonly dvdError: string | null;
  readonly refreshBurners: () => Promise<void>;
  readonly executeDvdCreate: () => Promise<void>;
  readonly bootselDrives: readonly string[];
  readonly uf2Variant: PicoUf2Variant;
  readonly setUf2Variant: (v: PicoUf2Variant) => void;
  readonly uf2Busy: boolean;
  readonly uf2Report: { readonly message: string } | null;
  readonly uf2Error: string | null;
  readonly detectBootsel: () => Promise<void>;
  readonly flashPicoUf2: () => Promise<void>;
  readonly ingestPathInput: string;
  readonly setIngestPathInput: (p: string) => void;
  readonly ingestBucketOverride: 'auto' | VaultBucket;
  readonly setIngestBucketOverride: (b: 'auto' | VaultBucket) => void;
  readonly ingestBusy: boolean;
  readonly ingestError: string | null;
  readonly ingestIntoVault: () => Promise<void>;
  readonly emuGateBusy: boolean;
  readonly emuGateReport: PreWriteGateReport | null;
  readonly emuGateError: string | null;
  readonly executeMandatoryEmuGate: () => Promise<void>;
  readonly hddTargets: readonly SafeTargetDiskInfo[];
  readonly hddDevicePath: string;
  readonly setHddDevicePath: (p: string) => void;
  readonly hddConfirmToken: string;
  readonly setHddConfirmToken: (t: string) => void;
  readonly hddBusy: boolean;
  readonly hddReport: ComposedHddFlashReport | null;
  readonly hddError: string | null;
  readonly executeComposedHddFlash: () => Promise<void>;
}
