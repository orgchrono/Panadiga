import { useCallback, useMemo, useState } from 'react';
import {
  REGION_CALIBRATION_CONFIRM_TOKEN,
  REGION_CALIBRATION_PRESETS,
  type LbsRegionPresetSlug,
  type TvStandardSlug,
} from '@/constants';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type {
  ApplyRegionCalibrationResponse,
  HardwareRegisterSnapshot,
  PipelineStepDescriptor,
  PreflightCheckItem,
  TvCalibrationBundleReport,
} from '@/models';
import { applyRegionCalibration, inspectRegionCalibration } from '@/services/pipelineService';
import { useDigestCopy, usePreflightSelection } from './usePipelineHelpers';
import { usePipelineRunner } from './usePipelineRunner';

const HEX32_PAD = 8;
const HEX16_PAD = 4;
const DEV_BIN_MODEL_MAX_LEN = 8;

function mapMmioToRegisters(report: TvCalibrationBundleReport | null): HardwareRegisterSnapshot[] {
  if (!report) return [];
  return report.profile.mmio_plan.map((op, idx) => ({
    id: `${op.register_name}-${idx}`,
    name: op.register_name,
    addressHex: `0x${op.addr.toString(16).toUpperCase().padStart(HEX32_PAD, '0')}`,
    valueHex: `0x${op.value.toString(16).toUpperCase().padStart(HEX16_PAD, '0')}`,
    widthBits: op.width_bits,
    statusLabel: report.profile.tv_standard.toUpperCase(),
    severity: 'info',
    note: op.note,
  }));
}

export function useCalibrationViewModel() {
  const { showToast } = useToast();
  const [preset, setPreset] = useState<LbsRegionPresetSlug>('br_pal_m_r4');
  const [tvStandard, setTvStandard] = useState<TvStandardSlug>('pal_m');
  const [devBinPath, setDevBinPath] = useState('firmware_and_images/extracted_partitions/ifodev2.bin');
  const [mainPath, setMainPath] = useState(
    'firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin',
  );
  const [frmPath, setFrmPath] = useState('');
  const [nvramPath, setNvramPath] = useState('');
  const [outputDir, setOutputDir] = useState('unbrick_out/calibrated');
  const [modelCode, setModelCode] = useState('EH55LB');
  const [oemChar, setOemChar] = useState('M');
  const [mainCompatMode, setMainCompatMode] = useState<'bypass' | 'factory_strict'>('bypass');
  const { checkedPreflightIds, togglePreflight } = usePreflightSelection();
  const [confirmTokenInput, setConfirmTokenInput] = useState('');
  const { copiedHash, copyDigest } = useDigestCopy();

  const selectPreset = useCallback((nextId: LbsRegionPresetSlug) => {
    setPreset(nextId);
    const spec = REGION_CALIBRATION_PRESETS.find(item => item.id === nextId);
    if (spec) {
      setTvStandard(spec.tvStandard);
      setModelCode(spec.defaultModelCode);
      setOemChar(spec.defaultOemChar);
    }
  }, []);

  const inspectRunner = usePipelineRunner<void, TvCalibrationBundleReport>({
    executeFn: () =>
      inspectRegionCalibration({
        preset,
        tvStandard,
        devBinPath: devBinPath.trim() || undefined,
        mainPath: mainPath.trim() || undefined,
        frmPath: frmPath.trim() || undefined,
        nvramPath: nvramPath.trim() || undefined,
      }),
    formatSuccess: () => translate('pipelines.calibration.steps.inspectArtifacts'),
  });

  const applyRunner = usePipelineRunner<void, ApplyRegionCalibrationResponse>({
    validateFn: () => {
      if (!outputDir.trim()) return translate('pipelines.calibration.inputs.outputDirRequired');
      if (modelCode.trim().length < 1 || modelCode.trim().length > DEV_BIN_MODEL_MAX_LEN) {
        return translate('pipelines.calibration.inputs.modelRequired');
      }
      if (oemChar.trim().length !== 1) return translate('pipelines.calibration.inputs.oemInvalid');
      return null;
    },
    executeFn: async () => {
      const applied = await applyRegionCalibration({
        preset,
        tvStandard,
        outputDir: outputDir.trim(),
        devBinPath: devBinPath.trim() || undefined,
        model: modelCode.trim(),
        oem: oemChar.trim(),
        mainPath: mainPath.trim() || undefined,
        mainCompatMode,
        frmPath: frmPath.trim() || undefined,
        confirmToken: confirmTokenInput.trim(),
      });
      await inspectRegionCalibration({
        preset,
        tvStandard,
        devBinPath: applied.devBinOutputPath ?? undefined,
        mainPath: applied.mainOutputPath ?? undefined,
        frmPath: applied.frmOutputPath ?? undefined,
      });
      showToast({ type: 'success', title: translate('errors.pipelines.calibrationApplied') });
      return applied;
    },
    formatSuccess: res => res.outputDir,
  });

  const preflightChecks = useMemo<readonly PreflightCheckItem[]>(
    () => [
      {
        id: 'distinctCow',
        title: translate('pipelines.calibration.preflight.distinctCow'),
        detail: translate('pipelines.calibration.preflight.distinctCowDetail'),
        severity: 'warning',
      },
      {
        id: 'backupReady',
        title: translate('pipelines.calibration.preflight.backupReady'),
        detail: translate('pipelines.calibration.preflight.backupReadyDetail'),
        severity: 'critical',
      },
      {
        id: 'timerMcuAck',
        title: translate('pipelines.calibration.preflight.timerMcuAck'),
        detail: translate('pipelines.calibration.preflight.timerMcuAckDetail'),
        severity: 'info',
      },
    ],
    [],
  );

  const preflightReady = preflightChecks.every(c => checkedPreflightIds.includes(c.id));
  const mmioRegisters = useMemo(
    () => mapMmioToRegisters(applyRunner.data?.bundleAudit ?? inspectRunner.data),
    [applyRunner.data?.bundleAudit, inspectRunner.data],
  );

  const steps = useMemo<readonly PipelineStepDescriptor[]>(
    () => [
      { id: 'selectPreset', label: translate('pipelines.calibration.steps.selectPreset'), status: 'verified' },
      {
        id: 'inspectArtifacts',
        label: translate('pipelines.calibration.steps.inspectArtifacts'),
        status: inspectRunner.isBusy ? 'running' : inspectRunner.error ? 'failed' : inspectRunner.data ? 'verified' : 'pending',
      },
      {
        id: 'cowBuild',
        label: translate('pipelines.calibration.steps.cowBuild'),
        status: applyRunner.isBusy ? 'running' : applyRunner.error ? 'failed' : applyRunner.data ? 'verified' : preflightReady ? 'pending' : 'blocked',
      },
      {
        id: 'postVerify',
        label: translate('pipelines.calibration.steps.postVerify'),
        status: applyRunner.data ? 'verified' : 'pending',
      },
    ],
    [applyRunner.data, applyRunner.error, applyRunner.isBusy, inspectRunner.data, inspectRunner.error, inspectRunner.isBusy, preflightReady],
  );

  return {
    preset,
    selectPreset,
    tvStandard,
    setTvStandard,
    devBinPath,
    setDevBinPath,
    mainPath,
    setMainPath,
    frmPath,
    setFrmPath,
    nvramPath,
    setNvramPath,
    outputDir,
    setOutputDir,
    modelCode,
    setModelCode,
    oemChar,
    setOemChar,
    mainCompatMode,
    setMainCompatMode,
    checkedPreflightIds,
    togglePreflight,
    preflightChecks,
    preflightReady,
    confirmTokenInput,
    setConfirmTokenInput,
    expectedConfirmToken: REGION_CALIBRATION_CONFIRM_TOKEN,
    inspectReport: inspectRunner.data,
    inspectBusy: inspectRunner.isBusy,
    inspectError: inspectRunner.error,
    runInspect: () => inspectRunner.execute(),
    applyResult: applyRunner.data,
    applyBusy: applyRunner.isBusy,
    applyError: applyRunner.error,
    applySuccess: applyRunner.successMessage,
    runApply: () => applyRunner.execute(),
    resetPipeline: () => {
      inspectRunner.reset();
      applyRunner.reset();
      setConfirmTokenInput('');
    },
    mmioRegisters,
    steps,
    copiedHash,
    copyDigest,
  };
}
