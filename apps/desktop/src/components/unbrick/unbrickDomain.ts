import type {
  SdDiskInspectionReport,
  SdMediaVerificationReport,
  SdUnbrickKitRequestDto,
} from '@/models';

export interface SdDiskVerificationProps {
  verifyLoading: boolean;
  verifyResult: SdMediaVerificationReport | null;
  verifyError: string | null;
  onVerifyMedia: (sdRootDir: string) => Promise<SdMediaVerificationReport | null>;
  diskLoading?: boolean;
  diskResult?: SdDiskInspectionReport | null;
  diskError?: string | null;
  onInspectSdDisk?: (target: string) => Promise<SdDiskInspectionReport | null>;
}

export type BgoMode = 'auto_format' | 'bypass' | 'uart_monitor' | 'factory';
export type MainPassthroughMode = 'av_passthrough' | 'factory_original';
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
  errorKey: string
): { trimmed: string; errorKey: string | null } {
  const trimmed = value.trim();
  return trimmed ? { trimmed, errorKey: null } : { trimmed: '', errorKey };
}

export interface SdKitFormInputs {
  outputDir: string;
  panaDvdPath: string;
  bgoMode: BgoMode;
  mainMode: MainPassthroughMode;
  includePanaSd: boolean;
  unlockHddDynamic: boolean;
}

export function buildSdKitRequestPayload(inputs: SdKitFormInputs): {
  errorKey: string | null;
  request: SdUnbrickKitRequestDto | null;
} {
  const outCheck = validateRequiredText(inputs.outputDir, 'unbrick.errors.outputDirRequired');
  if (outCheck.errorKey) {
    return { errorKey: outCheck.errorKey, request: null };
  }
  return {
    errorKey: null,
    request: {
      output_dir: outCheck.trimmed,
      pana_dvd_path: inputs.panaDvdPath.trim() || undefined,
      bgo_mode: inputs.bgoMode,
      main_passthrough_mode: inputs.mainMode,
      include_pana_sd: inputs.includePanaSd,
      unlock_hdd_dynamic: inputs.unlockHddDynamic,
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
  inErrorKey: string | null;
  outErrorKey: string | null;
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
