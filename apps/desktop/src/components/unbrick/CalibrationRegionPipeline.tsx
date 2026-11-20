import React from 'react';
import { AlertTriangle, Cpu, Search } from 'lucide-react';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Button,
  Card,
  CardContent,
  EmptyState,
  Input,
  Select,
} from '@/components/ui';
import {
  LOADER_VER061_TIMER_MCU_RANGE,
  REGION_CALIBRATION_PRESETS,
  type LbsRegionPresetSlug,
  type TvStandardSlug,
} from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  ApplyRegionCalibrationResponse,
  HardwareRegisterSnapshot,
  PipelinePreflightAndGateProps,
  PipelineStepDescriptor,
  TvCalibrationBundleReport,
} from '@/models';
import { isTvStandardSlug } from '@/utils/typeGuards';
import {
  HexRegisterGrid,
  PipelineCardHeader,
  PipelineStepRunner,
  SafetyConfirmGate,
  SafetyPreflightCard,
} from '@/components/pipelines';
import { CalibrationAuditCards, CalibrationOutputArtifacts } from './CalibrationRegionSubsections';

const DEV_BIN_MODEL_MAX_LEN = 8;

export interface CalibrationPipelineVm extends PipelinePreflightAndGateProps {
  readonly preset: LbsRegionPresetSlug;
  readonly selectPreset: (id: LbsRegionPresetSlug) => void;
  readonly tvStandard: TvStandardSlug;
  readonly setTvStandard: (std: TvStandardSlug) => void;
  readonly devBinPath: string;
  readonly setDevBinPath: (v: string) => void;
  readonly mainPath: string;
  readonly setMainPath: (v: string) => void;
  readonly frmPath: string;
  readonly setFrmPath: (v: string) => void;
  readonly nvramPath: string;
  readonly setNvramPath: (v: string) => void;
  readonly outputDir: string;
  readonly setOutputDir: (v: string) => void;
  readonly modelCode: string;
  readonly setModelCode: (v: string) => void;
  readonly oemChar: string;
  readonly setOemChar: (v: string) => void;
  readonly mainCompatMode: 'bypass' | 'factory_strict';
  readonly setMainCompatMode: (v: 'bypass' | 'factory_strict') => void;
  readonly inspectReport: TvCalibrationBundleReport | null;
  readonly inspectBusy: boolean;
  readonly inspectError: string | null;
  readonly runInspect: () => Promise<TvCalibrationBundleReport | null>;
  readonly applyResult: ApplyRegionCalibrationResponse | null;
  readonly applyBusy: boolean;
  readonly applyError: string | null;
  readonly applySuccess: string | null;
  readonly runApply: () => Promise<ApplyRegionCalibrationResponse | null>;
  readonly resetPipeline: () => void;
  readonly mmioRegisters: readonly HardwareRegisterSnapshot[];
  readonly steps: readonly PipelineStepDescriptor[];
  readonly copiedHash: boolean;
  readonly copyDigest: (digest: string) => void;
}

export const CalibrationRegionPipeline: React.FC<{ readonly vm: CalibrationPipelineVm }> = ({ vm }) => {
  const { t } = useTranslation();
  const auditReport = vm.applyResult ? vm.applyResult.bundleAudit : vm.inspectReport;
  const outputDirInvalid = vm.outputDir.trim().length === 0;
  const modelInvalid = vm.modelCode.trim().length < 1 || vm.modelCode.trim().length > DEV_BIN_MODEL_MAX_LEN;
  const oemInvalid = vm.oemChar.trim().length !== 1;
  const formValid = !outputDirInvalid && !modelInvalid && !oemInvalid;
  const calibrationPresets: readonly (typeof REGION_CALIBRATION_PRESETS)[number][] =
    REGION_CALIBRATION_PRESETS;

  const pathInputs: readonly {
    readonly id: string;
    readonly label: string;
    readonly value: string;
    readonly setter: (v: string) => void;
    readonly invalid?: boolean;
  }[] = [
    { id: 'cal-dev-bin', label: t('pipelines.calibration.inputs.devBinPath'), value: vm.devBinPath, setter: vm.setDevBinPath },
    { id: 'cal-main-path', label: t('pipelines.calibration.inputs.mainPath'), value: vm.mainPath, setter: vm.setMainPath },
    { id: 'cal-frm-path', label: t('pipelines.calibration.inputs.frmPath'), value: vm.frmPath, setter: vm.setFrmPath },
    { id: 'cal-nvram-path', label: t('pipelines.calibration.inputs.nvramPath'), value: vm.nvramPath, setter: vm.setNvramPath },
    { id: 'cal-output-dir', label: t('pipelines.calibration.inputs.outputDir'), value: vm.outputDir, setter: vm.setOutputDir, invalid: outputDirInvalid },
  ];

  return (
    <Card className="bg-bench-900 border-slate-800 space-y-4">
      <PipelineCardHeader
        icon={<Cpu className="w-4 h-4 text-cyan-400" />}
        title={t('pipelines.calibration.title')}
        subtitle={t('pipelines.calibration.subtitle')}
        badge={t('pipelines.calibration.metadataOnlyBadge')}
        badgeVariant="warning"
      />

      <CardContent className="space-y-4">
        <Alert variant="warning">
          <AlertTriangle className="h-4 w-4" />
          <AlertTitle>{t('pipelines.calibration.metadataOnlyBadge')}</AlertTitle>
          <AlertDescription className="space-y-1">
            <p>{t('pipelines.calibration.metadataOnlyAlert')}</p>
            <p className="text-amber-200/90">{t('pipelines.calibration.timerMcuNote')} ({LOADER_VER061_TIMER_MCU_RANGE})</p>
          </AlertDescription>
        </Alert>

        <PipelineStepRunner
          title={t('pipelines.calibration.title')}
          steps={vm.steps}
          isBusy={vm.inspectBusy || vm.applyBusy}
          error={vm.inspectError ?? vm.applyError}
          successMessage={vm.applySuccess}
          onReset={vm.resetPipeline}
        />

        <div className="space-y-2">
          <span className="text-xs font-mono font-bold text-slate-300 block">{t('pipelines.calibration.presetLabel')}</span>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
            {calibrationPresets.length === 0 ? (
              <EmptyState compact title={t('pipelines.calibration.vmodeselEmpty')} />
            ) : (
              calibrationPresets.map(p => (
                <button
                  key={p.id}
                  type="button"
                  data-testid={`calibration-preset-${p.id}`}
                  onClick={() => vm.selectPreset(p.id)}
                  className={`text-left p-3 rounded-lg border text-xs transition-colors ${
                    vm.preset === p.id ? 'bg-cyan-500/10 border-cyan-500/60 text-slate-100' : 'bg-slate-950 border-slate-800 text-slate-300 hover:border-slate-700'
                  }`}
                >
                  <div className="font-mono font-bold">{t(p.labelKey)}</div>
                  <div className="text-[11px] text-slate-400 mt-1">{t(p.descKey)}</div>
                </button>
              ))
            )}
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs font-mono">
          {pathInputs.length === 0 ? null : pathInputs.map(field => (
            <div key={field.id} className="space-y-1">
              <label htmlFor={field.id} className="text-slate-400 block">{field.label}</label>
              <Input id={field.id} data-testid={field.id} type="text" value={field.value} aria-invalid={field.invalid} onChange={e => field.setter(e.target.value)} />
              {field.invalid && <span className="text-[11px] text-rose-400">{t('pipelines.calibration.inputs.outputDirRequired')}</span>}
            </div>
          ))}
          <div className="space-y-1">
            <label htmlFor="cal-tv-std" className="text-slate-400 block">{t('pipelines.calibration.tvStandardLabel')}</label>
            <Select id="cal-tv-std" data-testid="cal-tv-std" value={vm.tvStandard} onChange={e => { if (isTvStandardSlug(e.target.value)) vm.setTvStandard(e.target.value); }}>
              <option value="pal_m">{t('pipelines.calibration.standards.pal_m')}</option>
              <option value="ntsc">{t('pipelines.calibration.standards.ntsc')}</option>
              <option value="pal">{t('pipelines.calibration.standards.pal')}</option>
              <option value="secam">{t('pipelines.calibration.standards.secam')}</option>
            </Select>
          </div>
          <div className="space-y-1">
            <label htmlFor="cal-model-code" className="text-slate-400 block">{t('pipelines.calibration.inputs.modelCode')}</label>
            <Input id="cal-model-code" data-testid="cal-model-code" type="text" value={vm.modelCode} aria-invalid={modelInvalid} onChange={e => vm.setModelCode(e.target.value)} />
            {modelInvalid && <span className="text-[11px] text-rose-400">{t('pipelines.calibration.inputs.modelRequired')}</span>}
          </div>
          <div className="space-y-1">
            <label htmlFor="cal-oem-char" className="text-slate-400 block">{t('pipelines.calibration.inputs.oemChar')}</label>
            <Input id="cal-oem-char" data-testid="cal-oem-char" type="text" value={vm.oemChar} aria-invalid={oemInvalid} onChange={e => vm.setOemChar(e.target.value)} />
            {oemInvalid && <span className="text-[11px] text-rose-400">{t('pipelines.calibration.inputs.oemInvalid')}</span>}
          </div>
          <div className="space-y-1">
            <label htmlFor="cal-main-mode" className="text-slate-400 block">{t('pipelines.calibration.inputs.mainCompatMode')}</label>
            <Select id="cal-main-mode" data-testid="cal-main-mode" value={vm.mainCompatMode} onChange={e => vm.setMainCompatMode(e.target.value === 'factory_strict' ? 'factory_strict' : 'bypass')}>
              <option value="bypass">{t('pipelines.calibration.inputs.modeBypass')}</option>
              <option value="factory_strict">{t('pipelines.calibration.inputs.modeFactory')}</option>
            </Select>
          </div>
        </div>

        <Button type="button" data-testid="calibration-inspect-btn" disabled={vm.inspectBusy} onClick={() => void vm.runInspect()} className="gap-1.5 font-mono text-xs">
          <Search className="w-3.5 h-3.5" />
          <span>{t('pipelines.calibration.actions.inspectBtn')}</span>
        </Button>

        <CalibrationAuditCards report={auditReport} />
        <HexRegisterGrid title={t('pipelines.calibration.vmodeselPreviewTitle')} registers={vm.mmioRegisters} emptyMessage={t('pipelines.calibration.vmodeselEmpty')} />
        <SafetyPreflightCard checks={vm.preflightChecks} checkedIds={vm.checkedPreflightIds} onToggleCheck={vm.togglePreflight} />
        <SafetyConfirmGate
          title={t('pipelines.calibration.actions.gateTitle')}
          description={t('pipelines.calibration.actions.gateDesc')}
          expectedToken={vm.expectedConfirmToken}
          typedValue={vm.confirmTokenInput}
          onChange={vm.setConfirmTokenInput}
          onConfirm={() => void vm.runApply()}
          isBusy={vm.applyBusy}
          preflightReady={vm.preflightReady && formValid}
          actionLabel={t('pipelines.calibration.actions.applyBtn')}
          inputTestId="calibration-confirm-input"
          submitTestId="calibration-apply-btn"
        />
        <CalibrationOutputArtifacts
          applyResult={vm.applyResult}
          devBinPath={vm.devBinPath}
          mainPath={vm.mainPath}
          frmPath={vm.frmPath}
          copiedHash={vm.copiedHash}
          copyDigest={vm.copyDigest}
        />
      </CardContent>
    </Card>
  );
};
