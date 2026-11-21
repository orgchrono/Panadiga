/**
 * @file services/vaultService.ts
 * @brief Typed Tauri IPC wrappers for Segregated Forensic Vaults, Mandatory Mn103Vm Gate,
 *        Optical ISO 9660 (1x) DVD Creator, Streaming Composed HDD Flash, and Relational Hardware Profiles.
 */

import type {
  Capability,
  ComposedHddFlashReport,
  CreateDvdRecoveryDiscRequest,
  DvdBurnProgressEvent,
  DvdCreatorReport,
  FlashComposedHddSafeRequest,
  LabOverride,
  NorFingerprintReport,
  OpticalBurnerDriveInfo,
  PreWriteGateReport,
  ProfileAuthorizationOutcome,
  RegistryValidationReport,
  ResolvedModelProfile,
  RunPreWriteEmuGateRequest,
  RuntimeFingerprint,
  StrictWorkflowStep,
  VaultArtifactRecord,
  VaultIngestFileRequest,
  VaultInitSessionRequest,
  VaultSessionManifest,
} from '@/models';
import { invokeCommand, listenEvent } from './ipc';

export const initVaultSession = (
  request: VaultInitSessionRequest,
): Promise<VaultSessionManifest> =>
  invokeCommand<VaultSessionManifest>('vault_init_session_command', { request });

export const listVaultSessions = (): Promise<VaultSessionManifest[]> =>
  invokeCommand<VaultSessionManifest[]>('vault_list_sessions_command');

export const getVaultSession = (sessionId: string): Promise<VaultSessionManifest> =>
  invokeCommand<VaultSessionManifest>('vault_get_session_command', { sessionId });

export const ingestVaultFile = (
  request: VaultIngestFileRequest,
): Promise<VaultArtifactRecord> =>
  invokeCommand<VaultArtifactRecord>('vault_ingest_file_command', { request });

export const markVaultWorkflowStep = (
  sessionId: string,
  step: StrictWorkflowStep,
): Promise<VaultSessionManifest> =>
  invokeCommand<VaultSessionManifest>('vault_mark_step_command', { sessionId, step });

export const runPreWriteEmuGate = (
  request: RunPreWriteEmuGateRequest,
): Promise<PreWriteGateReport> =>
  invokeCommand<PreWriteGateReport>('vault_run_prewrite_emu_gate_command', { request });

export const flashComposedHddSafe = (
  request: FlashComposedHddSafeRequest,
): Promise<ComposedHddFlashReport> =>
  invokeCommand<ComposedHddFlashReport>('flash_composed_hdd_safe_command', { request });

export const inspectOpticalBurners = (): Promise<OpticalBurnerDriveInfo[]> =>
  invokeCommand<OpticalBurnerDriveInfo[]>('inspect_optical_burners_command');

export const createDvdRecoveryDisc = (
  request: CreateDvdRecoveryDiscRequest,
): Promise<DvdCreatorReport> =>
  invokeCommand<DvdCreatorReport>('create_dvd_recovery_disc_command', { request });

export const subscribeDvdBurnProgress = (
  handler: (payload: DvdBurnProgressEvent) => void,
): Promise<() => void> => listenEvent('dvd://burn-progress', handler);

export const listHardwareProfiles = (): Promise<ResolvedModelProfile[]> =>
  invokeCommand<ResolvedModelProfile[]>('list_hardware_profiles_command');

export const getResolvedModelProfile = (modelId: string): Promise<ResolvedModelProfile> =>
  invokeCommand<ResolvedModelProfile>('get_resolved_model_profile_command', { modelId });

export const validateHardwareProfiles = (): Promise<RegistryValidationReport> =>
  invokeCommand<RegistryValidationReport>('validate_hardware_profiles_command');

export const authorizeProfileCapability = (
  modelId: string,
  capability: Capability,
  fingerprint?: RuntimeFingerprint,
  labOverride?: LabOverride,
): Promise<ProfileAuthorizationOutcome> =>
  invokeCommand<ProfileAuthorizationOutcome>('authorize_profile_capability_command', {
    modelId,
    model_id: modelId,
    capability,
    fingerprint: fingerprint ?? null,
    labOverride: labOverride ?? null,
    lab_override: labOverride ?? null,
  });

export const fingerprintNorDump = (filePath: string): Promise<NorFingerprintReport> =>
  invokeCommand<NorFingerprintReport>('fingerprint_nor_dump_command', {
    filePath,
    file_path: filePath,
  });
