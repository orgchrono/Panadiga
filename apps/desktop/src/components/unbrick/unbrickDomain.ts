import type { TranslationKey } from '@/i18n/translations';
import type {
  BgoMode,
  DriveRoleFilter,
  HddCapacityTargetOption,
  MainPassthroughMode,
  PhysicalDriveInfo,
  SdDiskInspectionReport,
  SdLoaderProfile,
  SdMediaVerificationReport,
  SdUnbrickKitRequestDto,
} from '@/models';

export type { BgoMode, HddCapacityTargetOption, MainPassthroughMode, SdLoaderProfile };

export function resolveHddCapacitySelection(target: HddCapacityTargetOption): {
  unlockHddDynamic: boolean;
  hddTargetGb: number | null;
} {
  if (target === 'dynamic') {
    return { unlockHddDynamic: true, hddTargetGb: null };
  }
  return { unlockHddDynamic: false, hddTargetGb: Number(target) };
}

export interface SdDiskVerificationProps {
  verifyLoading: boolean;
  verifyResult: SdMediaVerificationReport | null;
  verifyError: string | null;
  onVerifyMedia: (sdRootDir: string) => Promise<SdMediaVerificationReport | null>;
  diskLoading?: boolean;
  diskResult?: SdDiskInspectionReport | null;
  diskError?: string | null;
  onInspectSdDisk?: (target: string) => Promise<SdDiskInspectionReport | null>;
  sdDrives?: readonly PhysicalDriveInfo[];
  sdDriveFilter?: DriveRoleFilter;
  onChangeSdDriveFilter?: (filter: DriveRoleFilter) => void;
  sdDrivesLoading?: boolean;
  onRefreshSdDrives?: () => void;
  autoSdDiskTarget?: string | null;
  autoSdMountDir?: string | null;
}

export type BadgeVariant = 'success' | 'warning' | 'destructive' | 'secondary';

const SEVERITY_BADGE: Record<string, BadgeVariant> = {
  pass: 'success',
  warn: 'warning',
  fail: 'destructive',
};

export function resolveSeverityBadgeVariant(severity: string): BadgeVariant {
  return SEVERITY_BADGE[severity] ?? 'secondary';
}

export function resolveSdOverallBadgeVariant(status?: string | null): BadgeVariant {
  if (status === 'FAIL' || status === 'INCOMPLETE_OR_INVALID') {
    return 'destructive';
  }
  return 'success';
}

export function resolveTbcModeBadgeVariant(mode?: string | null): BadgeVariant {
  return mode === 'AV_PASSTHROUGH' ? 'success' : 'secondary';
}

export function validateRequiredText(
  value: string,
  errorKey: TranslationKey,
): { trimmed: string; errorKey: TranslationKey | null } {
  const trimmed = value.trim();
  return trimmed ? { trimmed, errorKey: null } : { trimmed: '', errorKey };
}

export interface SdKitFormInputs {
  outputDir: string;
  panaDvdPath: string;
  mainPath?: string;
  bgoMode: BgoMode;
  mainMode: MainPassthroughMode;
  loaderProfile?: SdLoaderProfile;
  includePanaSd: boolean;
  unlockHddDynamic: boolean;
  hddTargetGb?: number | null;
}

export function buildSdKitRequestPayload(inputs: SdKitFormInputs): {
  errorKey: TranslationKey | null;
  request: SdUnbrickKitRequestDto | null;
} {
  const outCheck = validateRequiredText(inputs.outputDir, 'unbrick.errors.outputDirRequired');
  if (outCheck.errorKey) {
    return { errorKey: outCheck.errorKey, request: null };
  }
  const hasExplicitTargetGb = typeof inputs.hddTargetGb === 'number';
  const trimmedMainPath = inputs.mainPath?.trim();
  return {
    errorKey: null,
    request: {
      output_dir: outCheck.trimmed,
      pana_dvd_path: inputs.panaDvdPath.trim() || undefined,
      ...(trimmedMainPath ? { main_path: trimmedMainPath } : {}),
      bgo_mode: inputs.bgoMode,
      main_passthrough_mode: inputs.mainMode,
      ...(inputs.loaderProfile ? { loader_profile: inputs.loaderProfile } : {}),
      include_pana_sd: inputs.includePanaSd,
      unlock_hdd_dynamic: hasExplicitTargetGb ? false : inputs.unlockHddDynamic,
      ...(hasExplicitTargetGb ? { hdd_target_gb: inputs.hddTargetGb } : {}),
    },
  };
}

export interface TbcActionInputs {
  action: 'inspect' | 'patch';
  mainInput: string;
  mainOutput: string;
  mode: MainPassthroughMode;
}

export interface TbcActionPayload {
  action: 'inspect' | 'patch';
  inputPath: string;
  outPath?: string;
  mode: MainPassthroughMode;
}

export function buildTbcActionParams(inputs: TbcActionInputs): {
  inErrorKey: TranslationKey | null;
  outErrorKey: TranslationKey | null;
  params: TbcActionPayload | null;
} {
  const inCheck = validateRequiredText(inputs.mainInput, 'unbrick.errors.mainPathRequired');
  if (inCheck.errorKey) {
    return { inErrorKey: inCheck.errorKey, outErrorKey: null, params: null };
  }

  if (inputs.action === 'patch') {
    const outCheck = validateRequiredText(inputs.mainOutput, 'unbrick.errors.mainOutRequired');
    if (outCheck.errorKey) {
      return { inErrorKey: null, outErrorKey: outCheck.errorKey, params: null };
    }
    return {
      inErrorKey: null,
      outErrorKey: null,
      params: {
        action: 'patch',
        inputPath: inCheck.trimmed,
        outPath: outCheck.trimmed,
        mode: inputs.mode,
      },
    };
  }

  return {
    inErrorKey: null,
    outErrorKey: null,
    params: {
      action: 'inspect',
      inputPath: inCheck.trimmed,
      mode: inputs.mode,
    },
  };
}
