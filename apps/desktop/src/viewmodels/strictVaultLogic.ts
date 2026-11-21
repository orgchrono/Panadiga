/**
 * @file viewmodels/strictVaultLogic.ts
 * @brief Pure functional state transitions, request builders, and resource loaders for Strict Vault workflow.
 */

import type {
  OpticalBurnerDriveInfo,
  ResolvedModelProfile,
  SafeTargetDiskInfo,
  SdLoaderProfile,
  StrictWorkflowStep,
  VaultArtifactRecord,
  VaultBucket,
  VaultSessionManifest,
} from '@/models';
import { detectPicoBootselDrives } from '@/services/picoService';
import { inspectSafeTargetDisks } from '@/services/pipelineService';
import { getResolvedModelProfile, inspectOpticalBurners } from '@/services/vaultService';

export const DEFAULT_MODEL_ID = 'DMR-EH55';
export const DEFAULT_DVD_LABEL = 'PANADIGA_REC';
export const DEFAULT_INGEST_SAMPLE = 'ic5001_nor_8mb.bin';
export const SECTOR_COUNT_ST3250 = 488_397_168;

export function appendUniqueStep(
  steps: readonly StrictWorkflowStep[],
  next: StrictWorkflowStep,
): readonly StrictWorkflowStep[] {
  return steps.includes(next) ? steps : [...steps, next];
}

export function appendArtifactRecord(
  session: VaultSessionManifest,
  rec: VaultArtifactRecord,
): VaultSessionManifest {
  const filterByName = (list: readonly VaultArtifactRecord[]) =>
    list.filter((item) => item.filename !== rec.filename);
  const rawArtifacts =
    rec.bucket === 'raw' ? [...filterByName(session.rawArtifacts), rec] : session.rawArtifacts;
  const frmArtifacts =
    rec.bucket === 'frm' ? [...filterByName(session.frmArtifacts), rec] : session.frmArtifacts;
  const recompiledArtifacts =
    rec.bucket === 'recompiled'
      ? [...filterByName(session.recompiledArtifacts), rec]
      : session.recompiledArtifacts;
  return {
    ...session,
    rawArtifacts,
    frmArtifacts,
    recompiledArtifacts,
    completedSteps: appendUniqueStep(session.completedSteps, 'capture_and_backup'),
    currentStep: 'organize_and_recompile',
  };
}

export async function loadStrictVaultDiscovery(modelId: string): Promise<{
  readonly profile: ResolvedModelProfile;
  readonly burners: readonly OpticalBurnerDriveInfo[];
  readonly disks: readonly SafeTargetDiskInfo[];
  readonly drives: readonly string[];
  readonly defaultSafeDiskPath: string | null;
}> {
  const [profile, burners, disks, drives] = await Promise.all([
    getResolvedModelProfile(modelId),
    inspectOpticalBurners(),
    inspectSafeTargetDisks(),
    detectPicoBootselDrives(),
  ]);
  const defaultSafeDiskPath = disks.find(d => d.safe_to_flash)?.device_path ?? null;
  return { profile, burners, disks, drives, defaultSafeDiskPath };
}

export function buildDvdRecoveryPayload(params: {
  readonly dvdTargetMode: 'iso' | 'burner';
  readonly selectedBurnerId: string;
  readonly dvdConfirmToken: string;
  readonly effectiveModel: string;
  readonly loaderProfile: SdLoaderProfile;
  readonly sessionId?: string;
}) {
  return {
    ...(params.dvdTargetMode === 'burner'
      ? {
          burnerId: params.selectedBurnerId.trim(),
          confirmToken: params.dvdConfirmToken.trim(),
        }
      : {}),
    model: params.effectiveModel,
    volumeLabel: DEFAULT_DVD_LABEL,
    loaderProfile: params.loaderProfile,
    cleanroomOnly: true,
    ...(params.sessionId ? { sessionId: params.sessionId } : {}),
  };
}

export function buildVaultIngestPayload(params: {
  readonly sessionId: string;
  readonly cleanPath: string;
  readonly ingestBucketOverride: 'auto' | VaultBucket;
}) {
  return {
    sessionId: params.sessionId,
    sourcePath: params.cleanPath,
    ...(params.ingestBucketOverride !== 'auto'
      ? { bucketOverride: params.ingestBucketOverride }
      : {}),
    origin: 'express_vault_ingest',
  };
}

export function buildPreWriteEmuGatePayload(
  loaderProfile: SdLoaderProfile,
  sessionId?: string,
) {
  return {
    ...(sessionId ? { sessionId } : {}),
    targetKind: 'composed_hdd_stream' as const,
    loaderProfile,
    totalSectors: SECTOR_COUNT_ST3250,
    expectPassthrough: true,
    expectCapacityUnlock: true,
  };
}

export function buildComposedHddPayload(params: {
  readonly hddDevicePath: string;
  readonly hddConfirmToken: string;
  readonly sessionId?: string;
}) {
  return {
    devicePath: params.hddDevicePath.trim(),
    confirmDiskModelToken: params.hddConfirmToken.trim(),
    applyDynamicCapacityUnlock: true,
    applyAvPassthrough: true,
    ...(params.sessionId ? { sessionId: params.sessionId } : {}),
  };
}
