import { useCallback, useMemo, useState } from 'react';
import { DAVIS_MMIO_SPECS, DEFAULT_BGO_BOOTTRACE_MS } from '@/constants';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type {
  DavisChannelId,
  DavisDriveSelector,
  PicoProbeDavisIdeResponse,
  PipelineStepDescriptor,
} from '@/models';
import { picoProbeDavisIde, subscribeDavisProbeResult } from '@/services/pipelineService';
import { isDavisEh55Compatible, mapDavisReportToRegisters } from './benchHelpers';
import { usePipelineRunner } from './usePipelineRunner';

const DAVIS_STEP_PROGRESS_CURRENT = 1;
const DAVIS_STEP_PROGRESS_TOTAL = 1;

export interface UseDavisProbeViewModelOptions {
  readonly initialPort?: string;
  readonly onOpenHddPipeline?: () => void;
}

export function useDavisProbeViewModel(options?: UseDavisProbeViewModelOptions) {
  const { showToast } = useToast();
  const [portOverride, setPortOverride] = useState<string | null>(null);
  const [channel, setChannel] = useState<DavisChannelId>('ch1');
  const [drive, setDrive] = useState<DavisDriveSelector>('master');
  const [bootMode, setBootMode] = useState<'NORMAL' | 'PASSIVE'>('NORMAL');

  const portName = portOverride ?? (options?.initialPort || 'COM10');

  const subscribeProbe = useCallback(
    (onProgress: Parameters<NonNullable<Parameters<typeof usePipelineRunner>[0]['subscribeProgress']>>[0]) =>
      subscribeDavisProbeResult(ev =>
        onProgress(
          {
            current: DAVIS_STEP_PROGRESS_CURRENT,
            total: DAVIS_STEP_PROGRESS_TOTAL,
            stageLabel: `${ev.port} · ${ev.report.classification}`,
          },
          `[${ev.driveSelHex}] ${ev.report.classification}`,
        ),
      ),
    [],
  );

  const probeRunner = usePipelineRunner<void, PicoProbeDavisIdeResponse>({
    subscribeProgress: subscribeProbe,
    validateFn: () => (!portName.trim() ? translate('pipelines.davisProbe.portRequired') : null),
    executeFn: async () => {
      const res = await picoProbeDavisIde({
        portName: portName.trim(),
        drive,
        bootMode,
        bootTimeoutMs: DEFAULT_BGO_BOOTTRACE_MS,
      });
      showToast({ type: 'success', title: translate('errors.pipelines.davisProbeComplete') });
      return res;
    },
    formatSuccess: res => `${res.driveSelHex} · ${res.report.classification}`,
  });

  const registers = useMemo(
    () => mapDavisReportToRegisters(probeRunner.data?.report ?? null, channel),
    [channel, probeRunner.data?.report],
  );

  const eh55Compatible = useMemo(
    () => isDavisEh55Compatible(probeRunner.data?.report ?? null),
    [probeRunner.data?.report],
  );

  const hasIdentify = Boolean(probeRunner.data?.report.identify_info);
  const steps = useMemo<readonly PipelineStepDescriptor[]>(
    () => [
      {
        id: 'probe',
        label: translate('pipelines.davisProbe.probeBtn'),
        status: probeRunner.isBusy
          ? 'running'
          : probeRunner.error
            ? 'failed'
            : probeRunner.data
              ? 'verified'
              : 'pending',
        description: DAVIS_MMIO_SPECS.lethalRegHex,
      },
      {
        id: 'identify',
        label: translate('pipelines.davisProbe.identify.title'),
        status: probeRunner.isBusy
          ? 'running'
          : hasIdentify
            ? 'verified'
            : probeRunner.data
              ? 'blocked'
              : 'pending',
      },
    ],
    [hasIdentify, probeRunner.data, probeRunner.error, probeRunner.isBusy],
  );

  const openHddPipeline = useCallback(() => {
    options?.onOpenHddPipeline?.();
  }, [options]);

  return {
    portName,
    setPortName: setPortOverride,
    channel,
    setChannel,
    drive,
    setDrive,
    bootMode,
    setBootMode,
    probeResult: probeRunner.data,
    probeBusy: probeRunner.isBusy,
    probeError: probeRunner.error,
    probeSuccess: probeRunner.successMessage,
    probeProgress: probeRunner.progress,
    runProbe: () => probeRunner.execute(),
    resetProbe: probeRunner.reset,
    registers,
    eh55Compatible,
    steps,
    openHddPipeline,
  };
}
