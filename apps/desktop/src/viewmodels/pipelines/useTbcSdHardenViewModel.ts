import { useCallback, useMemo, useState } from 'react';
import {
  KIB_BYTES,
  LBA_SECTOR_SIZE_BYTES,
  MAIN_TBC_BYPASS_SPECS,
  SD_FAT16_MAX_BYTES,
  TBC_BYPASS_CONFIRM_TOKEN,
  type MainBypassId,
  type TvStandardSlug,
} from '@/constants';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type {
  ApplyMainTbcBypassesResponse,
  HardenSdUnbrickKitResponse,
  MainPassthroughMode,
  MainPassthroughRspPlan,
  PipelineStepDescriptor,
  PreflightCheckItem,
  SdDiskInspectionReport,
  TbcApplianceMode,
} from '@/models';
import { applyMainTbcBypasses, hardenSdUnbrickKit } from '@/services/pipelineService';
import { generateMainRspPlan, inspectSdDisk } from '@/services/unbrickService';
import { usePreflightSelection } from './usePipelineHelpers';
import { usePipelineRunner } from './usePipelineRunner';

const ALL_BYPASS_IDS: readonly MainBypassId[] = MAIN_TBC_BYPASS_SPECS.map(s => s.id);
const BYTES_PER_MIB = KIB_BYTES * KIB_BYTES;

export function evaluateSd91006Gate(report: SdDiskInspectionReport | null): {
  readonly blocked: boolean;
  readonly reason: string | null;
} {
  if (!report || !report.bpb) return { blocked: false, reason: null };
  const sizeBytes = report.bpb.total_sectors * LBA_SECTOR_SIZE_BYTES;
  const mb = Math.round(sizeBytes / BYTES_PER_MIB);
  if (sizeBytes > SD_FAT16_MAX_BYTES) {
    return { blocked: true, reason: translate('pipelines.tbcSd.sdSizeBlocked', { mb }) };
  }
  const fs = report.bpb.computed_fat_kind.toUpperCase();
  if (fs !== 'FAT16' && fs !== 'FAT12') {
    return { blocked: true, reason: translate('pipelines.tbcSd.sdFsBlocked', { fs }) };
  }
  return { blocked: false, reason: translate('pipelines.tbcSd.sdGateReady', { fs, mb }) };
}

export function useTbcSdHardenViewModel() {
  const { showToast } = useToast();
  const [applianceMode, setApplianceMode] = useState<TbcApplianceMode>('stage15_digadiag_tbc');
  const [experimentalRamAck, setExperimentalRamAck] = useState(false);
  const [tvStandard, setTvStandard] = useState<TvStandardSlug>('ntsc');
  const [passthroughMode, setPassthroughMode] = useState<MainPassthroughMode>('av_passthrough');
  const [enabledBypasses, setEnabledBypasses] = useState<readonly MainBypassId[]>(ALL_BYPASS_IDS);
  const [inputMainPath, setInputMainPath] = useState(
    'firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin',
  );
  const [outputMainPath, setOutputMainPath] = useState('unbrick_out/MAIN3410PQ_STANDALONE_TBC.bin');
  const [tbcConfirmToken, setTbcConfirmToken] = useState('');
  const [rspPlan, setRspPlan] = useState<MainPassthroughRspPlan | null>(null);
  const [sdOutputDir, setSdOutputDir] = useState('firmware_and_images/sd_unbrick_hardened');
  const [panaDvdPath, setPanaDvdPath] = useState('firmware_and_images/PANA_DVD.FRM');
  const [prog061Path, setProg061Path] = useState('');
  const [allowFactoryProg061, setAllowFactoryProg061] = useState(false);
  const [sdDiskTarget, setSdDiskTarget] = useState('E:');
  const [sdDiskReport, setSdDiskReport] = useState<SdDiskInspectionReport | null>(null);
  const [sdDiskBusy, setSdDiskBusy] = useState(false);
  const { checkedPreflightIds, togglePreflight } = usePreflightSelection();

  const applyStandaloneTbcPreset = useCallback(() => {
    setApplianceMode('stage15_digadiag_tbc');
    setPassthroughMode('av_passthrough');
    setEnabledBypasses(ALL_BYPASS_IDS);
  }, []);

  const toggleBypass = useCallback((id: MainBypassId) => {
    setEnabledBypasses(prev => (prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id]));
  }, []);

  const tbcRunner = usePipelineRunner<void, ApplyMainTbcBypassesResponse>({
    validateFn: () => {
      if (inputMainPath.trim() && !outputMainPath.trim()) {
        return translate('pipelines.tbcSd.cowOutRequired');
      }
      return null;
    },
    executeFn: async () => {
      const res = await applyMainTbcBypasses({
        inputMainPath: inputMainPath.trim() || undefined,
        outputMainPath: outputMainPath.trim() || undefined,
        passthroughMode,
        applianceMode,
        tvStandard,
        experimentalRamChainload: experimentalRamAck,
        confirmToken: tbcConfirmToken.trim(),
      });
      showToast({ type: 'success', title: translate('errors.pipelines.tbcApplied') });
      return res;
    },
    formatSuccess: res => res.cowOutputPath ?? res.appliancePlan.mode,
  });

  const hardenRunner = usePipelineRunner<void, HardenSdUnbrickKitResponse>({
    validateFn: () => (!sdOutputDir.trim() ? translate('unbrick.errors.outputDirRequired') : null),
    executeFn: async () => {
      const res = await hardenSdUnbrickKit({
        outputDir: sdOutputDir.trim(),
        panaDvdPath: panaDvdPath.trim() || undefined,
        prog061Path: prog061Path.trim() || undefined,
        mainPath: inputMainPath.trim() || undefined,
        mainPassthroughMode: passthroughMode,
        unlockHddDynamic: enabledBypasses.includes('hdd_cmd'),
        allowFactoryProg061,
      });
      showToast({ type: 'success', title: translate('errors.pipelines.sdHardened') });
      return res;
    },
    formatSuccess: res => res.kitReport.digadiag_path,
  });

  const fetchRspPlan = useCallback(async () => {
    const plan = await generateMainRspPlan();
    setRspPlan(plan);
    return plan;
  }, []);

  const auditSdDisk = useCallback(async () => {
    if (!sdDiskTarget.trim()) return null;
    setSdDiskBusy(true);
    try {
      const rep = await inspectSdDisk(sdDiskTarget.trim());
      setSdDiskReport(rep);
      return rep;
    } finally {
      setSdDiskBusy(false);
    }
  }, [sdDiskTarget]);

  const sdGate = useMemo(() => evaluateSd91006Gate(sdDiskReport), [sdDiskReport]);
  const preflightChecks = useMemo<readonly PreflightCheckItem[]>(
    () => [
      {
        id: 'autoFormatD4D5',
        title: translate('pipelines.tbcSd.preflight.autoFormatD4D5'),
        detail: translate('pipelines.tbcSd.preflight.autoFormatD4D5Detail'),
        severity: 'critical',
      },
      {
        id: 'sd91006Fat16',
        title: translate('pipelines.tbcSd.preflight.sd91006Fat16'),
        detail: translate('pipelines.tbcSd.preflight.sd91006Fat16Detail'),
        severity: 'critical',
        lockedReason: sdGate.blocked ? sdGate.reason ?? undefined : undefined,
      },
      {
        id: 'p59001P7402',
        title: translate('pipelines.tbcSd.preflight.p59001P7402'),
        detail: translate('pipelines.tbcSd.preflight.p59001P7402Detail'),
        severity: 'warning',
      },
    ],
    [sdGate.blocked, sdGate.reason],
  );

  const preflightReady =
    !sdGate.blocked && preflightChecks.every(c => checkedPreflightIds.includes(c.id));

  const steps = useMemo<readonly PipelineStepDescriptor[]>(
    () => [
      { id: 'planTbc', label: translate('pipelines.tbcSd.steps.planTbc'), status: 'verified' },
      {
        id: 'patchMain',
        label: translate('pipelines.tbcSd.steps.patchMain'),
        status: tbcRunner.isBusy ? 'running' : tbcRunner.error ? 'failed' : tbcRunner.data ? 'verified' : 'pending',
      },
      {
        id: 'sdGate',
        label: translate('pipelines.tbcSd.steps.sdGate'),
        status: sdGate.blocked ? 'blocked' : preflightReady ? 'verified' : 'pending',
      },
      {
        id: 'hardenKit',
        label: translate('pipelines.tbcSd.steps.hardenKit'),
        status: hardenRunner.isBusy ? 'running' : hardenRunner.error ? 'failed' : hardenRunner.data ? 'verified' : preflightReady ? 'pending' : 'blocked',
      },
    ],
    [hardenRunner.data, hardenRunner.error, hardenRunner.isBusy, preflightReady, sdGate.blocked, tbcRunner.data, tbcRunner.error, tbcRunner.isBusy],
  );

  return {
    applianceMode,
    setApplianceMode,
    experimentalRamAck,
    setExperimentalRamAck,
    tvStandard,
    setTvStandard,
    passthroughMode,
    setPassthroughMode,
    enabledBypasses,
    toggleBypass,
    applyStandaloneTbcPreset,
    inputMainPath,
    setInputMainPath,
    outputMainPath,
    setOutputMainPath,
    tbcConfirmToken,
    setTbcConfirmToken,
    expectedTbcToken: TBC_BYPASS_CONFIRM_TOKEN,
    tbcResult: tbcRunner.data,
    tbcBusy: tbcRunner.isBusy,
    tbcError: tbcRunner.error,
    runTbcBypasses: () => tbcRunner.execute(),
    rspPlan,
    fetchRspPlan,
    sdOutputDir,
    setSdOutputDir,
    panaDvdPath,
    setPanaDvdPath,
    prog061Path,
    setProg061Path,
    allowFactoryProg061,
    setAllowFactoryProg061,
    sdDiskTarget,
    setSdDiskTarget,
    sdDiskReport,
    sdDiskBusy,
    auditSdDisk,
    sdGate,
    checkedPreflightIds,
    togglePreflight,
    preflightChecks,
    preflightReady,
    hardenResult: hardenRunner.data,
    hardenBusy: hardenRunner.isBusy,
    hardenError: hardenRunner.error,
    hardenSuccess: hardenRunner.successMessage,
    runHardenSdKit: () => hardenRunner.execute(),
    resetAll: () => {
      tbcRunner.reset();
      hardenRunner.reset();
      setTbcConfirmToken('');
    },
    steps,
  };
}
