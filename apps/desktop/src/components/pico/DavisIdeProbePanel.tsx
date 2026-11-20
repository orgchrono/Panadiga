import React from 'react';
import { Activity, Cpu, ShieldCheck } from 'lucide-react';
import { HexRegisterGrid, PipelineCardHeader, PipelineStepRunner } from '@/components/pipelines';
import {
  Alert,
  AlertDescription,
  Button,
  Card,
  CardContent,
  Input,
  Select,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  DavisChannelId,
  DavisDriveSelector,
  HardwareRegisterSnapshot,
  PicoProbeDavisIdeResponse,
  PipelineProgressState,
  PipelineStepDescriptor,
} from '@/models';
import { isDavisChannelId, isDavisDriveSelector } from '@/utils/typeGuards';
import { DavisIdentifySection } from './DavisIdentifySection';

export interface DavisProbeVm {
  readonly portName: string;
  readonly setPortName: (v: string) => void;
  readonly channel: DavisChannelId;
  readonly setChannel: (ch: DavisChannelId) => void;
  readonly drive: DavisDriveSelector;
  readonly setDrive: (d: DavisDriveSelector) => void;
  readonly bootMode: 'NORMAL' | 'PASSIVE';
  readonly setBootMode: (m: 'NORMAL' | 'PASSIVE') => void;
  readonly probeResult: PicoProbeDavisIdeResponse | null;
  readonly probeBusy: boolean;
  readonly probeError: string | null;
  readonly probeSuccess: string | null;
  readonly probeProgress: PipelineProgressState | null;
  readonly runProbe: () => Promise<PicoProbeDavisIdeResponse | null>;
  readonly resetProbe: () => void;
  readonly registers: readonly HardwareRegisterSnapshot[];
  readonly eh55Compatible: boolean;
  readonly steps: readonly PipelineStepDescriptor[];
  readonly openHddPipeline: () => void;
}

const ERROR_CLASSIFICATIONS = new Set([
  'floating_bus',
  'reset_stuck_or_unclocked',
  'bsy_iordy_hang',
  'cs_jumper_or_slave_contention',
]);

export const DavisIdeProbePanel: React.FC<{ readonly vm: DavisProbeVm }> = ({ vm }) => {
  const { t } = useTranslation();
  const portMissing = !vm.portName.trim();
  const report = vm.probeResult?.report ?? null;
  const info = report?.identify_info ?? null;
  const isErrorClass = Boolean(report && ERROR_CLASSIFICATIONS.has(report.classification));
  const afterHex = report ? `0x${report.c0001008_after.toString(16).toUpperCase()}` : '';

  return (
    <Card className="bg-bench-900 border-slate-800 space-y-4">
      <PipelineCardHeader
        icon={<Cpu className="w-4 h-4 text-cyan-400" />}
        title={t('pipelines.davisProbe.title')}
        subtitle={t('pipelines.davisProbe.subtitle')}
        badge={t('pipelines.davisProbe.lethalGuardBadge')}
        badgeVariant="success"
      />

      <CardContent className="space-y-4 font-mono text-xs">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-3 items-end">
          <div className="space-y-1">
            <label htmlFor="davis-port-input" className="text-slate-400 block">
              {t('pipelines.davisProbe.portLabel')}
            </label>
            <Input
              id="davis-port-input"
              data-testid="davis-port-input"
              type="text"
              value={vm.portName}
              aria-invalid={portMissing}
              onChange={e => vm.setPortName(e.target.value)}
            />
            {portMissing && (
              <span className="text-[11px] text-rose-400">{t('pipelines.davisProbe.portRequired')}</span>
            )}
          </div>
          <div className="space-y-1">
            <label htmlFor="davis-channel-select" className="text-slate-400 block">
              {t('pipelines.davisProbe.channelLabel')}
            </label>
            <Select
              id="davis-channel-select"
              data-testid="davis-channel-select"
              value={vm.channel}
              onChange={e => {
                if (isDavisChannelId(e.target.value)) vm.setChannel(e.target.value);
              }}
            >
              <option value="ch1">{t('pipelines.davisProbe.ch1Label')}</option>
              <option value="ch0">{t('pipelines.davisProbe.ch0Label')}</option>
            </Select>
          </div>
          <div className="space-y-1">
            <label htmlFor="davis-drive-select" className="text-slate-400 block">
              {t('pipelines.davisProbe.driveLabel')}
            </label>
            <Select
              id="davis-drive-select"
              data-testid="davis-drive-select"
              value={vm.drive}
              onChange={e => {
                if (isDavisDriveSelector(e.target.value)) vm.setDrive(e.target.value);
              }}
            >
              <option value="master">{t('pipelines.davisProbe.masterOpt')}</option>
              <option value="slave">{t('pipelines.davisProbe.slaveOpt')}</option>
            </Select>
          </div>
          <div className="space-y-1">
            <label htmlFor="davis-boot-select" className="text-slate-400 block">
              {t('pipelines.davisProbe.bootModeLabel')}
            </label>
            <Select
              id="davis-boot-select"
              value={vm.bootMode}
              onChange={e => vm.setBootMode(e.target.value === 'PASSIVE' ? 'PASSIVE' : 'NORMAL')}
            >
              <option value="NORMAL">{t('pipelines.davisProbe.bootNormal')}</option>
              <option value="PASSIVE">{t('pipelines.davisProbe.bootPassive')}</option>
            </Select>
          </div>
        </div>

        <Button
          type="button"
          size="sm"
          data-testid="davis-probe-btn"
          disabled={vm.probeBusy || portMissing}
          onClick={() => void vm.runProbe()}
          className="gap-1.5"
        >
          <Activity className="w-3.5 h-3.5" />
          <span>{t('pipelines.davisProbe.probeBtn')}</span>
        </Button>

        <PipelineStepRunner
          title={t('pipelines.davisProbe.title')}
          steps={vm.steps}
          isBusy={vm.probeBusy}
          error={vm.probeError}
          successMessage={vm.probeSuccess}
          progress={vm.probeProgress}
          onReset={vm.resetProbe}
        />

        {report && (
          <Alert variant={isErrorClass ? 'destructive' : 'success'}>
            <ShieldCheck className="h-4 w-4" />
            <AlertDescription className="space-y-1">
              <div className="font-bold">
                {t(`pipelines.davisProbe.classifications.${report.classification}`)}
              </div>
              <div>
                {report.fe_released
                  ? t('pipelines.davisProbe.feReleasedOk', { after: afterHex })
                  : t('pipelines.davisProbe.feReleasedPending', { after: afterHex })}
              </div>
            </AlertDescription>
          </Alert>
        )}

        <HexRegisterGrid
          title={t('pipelines.davisProbe.registersTitle')}
          registers={vm.registers}
          emptyMessage={t('pipelines.davisProbe.registersEmpty')}
        />

        <DavisIdentifySection
          info={info}
          eh55Compatible={vm.eh55Compatible}
          stepLogs={report?.step_logs ?? []}
          onOpenHddPipeline={vm.openHddPipeline}
        />
      </CardContent>
    </Card>
  );
};
