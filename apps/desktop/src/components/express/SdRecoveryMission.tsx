import React from 'react';
import {
  AlertTriangle,
  CheckCircle2,
  Disc3,
  HardDrive,
  Loader2,
  RefreshCw,
} from 'lucide-react';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  EmptyState,
  Input,
  Select,
} from '@/components/ui';
import { SD_MBR_FAT16B_TYPE_HEX, SD_MBR_START_LBA } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  DigaModelProfile,
  SdCardCreatorReport,
  SdFlashProgressEvent,
  SdTargetDriveInfo,
} from '@/models';
import { DigaModelSelect } from './DigaModelSelect';
import { ExpressModeChoiceGroup } from './ExpressModeChoiceGroup';
import { RecorderDisplayGuide } from './RecorderDisplayGuide';
import { TechnicalDetailsDrawer, type TechnicalDetailRow } from './TechnicalDetailsDrawer';

const PCT_FULL = 100;

export interface SdRecoveryMissionProps {
  readonly modelCatalog: readonly DigaModelProfile[];
  readonly selectedModel: string;
  readonly onSelectModel: (model: string) => void;
  readonly includePanaSd: boolean;
  readonly onToggleIncludePanaSd: (checked: boolean) => void;
  readonly sdTargets: readonly SdTargetDriveInfo[];
  readonly sdTargetsLoading: boolean;
  readonly sdTargetMode: 'image' | 'physical';
  readonly onSelectTargetMode: (mode: 'image' | 'physical') => void;
  readonly selectedDevicePath: string;
  readonly onSelectDevicePath: (path: string) => void;
  readonly selectedSdDrive: SdTargetDriveInfo | null;
  readonly sdConfirmToken: string;
  readonly onChangeConfirmToken: (token: string) => void;
  readonly sdFlashing: boolean;
  readonly sdProgress: SdFlashProgressEvent | null;
  readonly sdError: string | null;
  readonly sdReport: SdCardCreatorReport | null;
  readonly onRefreshTargets: () => void;
  readonly onExecuteFlash: () => void;
  readonly onOpenEngineering: () => void;
}

const SdPhysicalDrivePicker: React.FC<{
  readonly sdTargets: readonly SdTargetDriveInfo[];
  readonly selectedDevicePath: string;
  readonly onSelectDevicePath: (path: string) => void;
  readonly selectedSdDrive: SdTargetDriveInfo | null;
  readonly sdConfirmToken: string;
  readonly onChangeConfirmToken: (token: string) => void;
}> = ({
  sdTargets,
  selectedDevicePath,
  onSelectDevicePath,
  selectedSdDrive,
  sdConfirmToken,
  onChangeConfirmToken,
}) => {
  const { t } = useTranslation();
  if (sdTargets.length === 0) {
    return <EmptyState compact title={t('express.sdMission.noPhysicalSd')} />;
  }
  return (
    <div className="space-y-2 pt-1">
      <Select
        aria-label={t('express.sdMission.targetModePhysical')}
        data-testid="express-sd-drive-select"
        value={selectedDevicePath}
        onChange={e => onSelectDevicePath(e.target.value)}
      >
        {sdTargets.map(drive => (
          <option key={drive.device_path} value={drive.device_path}>
            {drive.display_name} ({drive.size_gb} GB • {drive.advisory.card_class})
          </option>
        ))}
      </Select>
      {selectedSdDrive?.advisory.is_sdxc_warning && (
        <Alert variant="warning" className="py-2">
          <AlertTriangle className="h-4 w-4" />
          <AlertTitle>{t('express.sdMission.sdxcWarningTitle')}</AlertTitle>
          <AlertDescription className="text-[11px]">{selectedSdDrive.advisory.detail}</AlertDescription>
        </Alert>
      )}
      {selectedSdDrive && (
        <div className="space-y-1">
          <label htmlFor="express-sd-confirm" className="text-[11px] font-mono text-amber-300 block">
            {t('express.sdMission.confirmTokenLabel', {
              expected: selectedSdDrive.expected_confirm_token,
            })}
          </label>
          <Input
            id="express-sd-confirm"
            data-testid="express-sd-confirm-input"
            value={sdConfirmToken}
            onChange={e => onChangeConfirmToken(e.target.value)}
            placeholder={t('express.sdMission.confirmTokenPlaceholder')}
          />
        </div>
      )}
    </div>
  );
};

export const SdRecoveryMission: React.FC<SdRecoveryMissionProps> = ({
  modelCatalog,
  selectedModel,
  onSelectModel,
  includePanaSd,
  onToggleIncludePanaSd,
  sdTargets,
  sdTargetsLoading,
  sdTargetMode,
  onSelectTargetMode,
  selectedDevicePath,
  onSelectDevicePath,
  selectedSdDrive,
  sdConfirmToken,
  onChangeConfirmToken,
  sdFlashing,
  sdProgress,
  sdError,
  sdReport,
  onRefreshTargets,
  onExecuteFlash,
  onOpenEngineering,
}) => {
  const { t } = useTranslation();
  const progressPct =
    sdProgress && sdProgress.bytesTotal > 0
      ? Math.min(PCT_FULL, Math.round((sdProgress.bytesDone / sdProgress.bytesTotal) * PCT_FULL))
      : 0;
  const confirmMismatch =
    sdTargetMode === 'physical' &&
    (!selectedSdDrive || sdConfirmToken.trim() !== selectedSdDrive.expected_confirm_token);
  const techRows: readonly TechnicalDetailRow[] = sdReport
    ? [
        { label: t('express.technical.sha256Label'), value: sdReport.image_sha256 },
        {
          label: t('express.technical.geometryLabel'),
          value: `MBR ${SD_MBR_FAT16B_TYPE_HEX} • ${sdReport.volume_mb} MiB • LBA ${sdReport.inspection.active_partition_lba ?? SD_MBR_START_LBA}`,
        },
        {
          label: 'Mn103Vm Gate',
          value: `${sdReport.emu_gate.passed ? 'PASS' : 'FAIL'} (${sdReport.emu_gate.executedSteps} steps • cleanroom=${String(sdReport.cleanroom_only)})`,
        },
        { label: t('express.technical.backupLabel'), value: sdReport.target },
      ]
    : [];
  const actionLabel = sdFlashing
    ? t('express.sdMission.flashingBtn')
    : sdTargetMode === 'physical'
      ? t('express.sdMission.flashBtn')
      : t('express.sdMission.createBtn');

  return (
    <Card data-testid="express-mission-sd" className="border-slate-800 bg-bench-900/90 shadow-xl">
      <CardHeader>
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div>
            <CardTitle className="flex items-center gap-2 text-base text-slate-100">
              <Disc3 className="w-5 h-5 text-emerald-400" />
              <span>{t('express.sdMission.title')}</span>
            </CardTitle>
            <CardDescription className="mt-1">{t('express.sdMission.desc')}</CardDescription>
          </div>
          <Badge variant="success" className="font-mono text-[10px]">
            {t('express.sdMission.badgeFat16')}
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="space-y-5">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-3">
            <h3 className="text-xs font-mono font-bold text-cyan-300">
              {t('express.sdMission.step1Title')}
            </h3>
            <div className="space-y-1">
              <label htmlFor="express-sd-model" className="text-[11px] font-mono text-slate-400">
                {t('express.sdMission.modelLabel')}
              </label>
              <DigaModelSelect
                id="express-sd-model"
                testId="express-sd-model-select"
                modelCatalog={modelCatalog}
                value={selectedModel}
                onChange={onSelectModel}
              />
            </div>
            <label className="flex items-start gap-2 text-xs text-slate-300 cursor-pointer pt-1">
              <input
                type="checkbox"
                data-testid="express-sd-include-pana"
                checked={includePanaSd}
                onChange={e => onToggleIncludePanaSd(e.target.checked)}
                className="mt-0.5 rounded border-slate-700 bg-slate-900 accent-emerald-500"
              />
              <span>{t('express.sdMission.includePanaSdLabel')}</span>
            </label>
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-3">
            <div className="flex items-center justify-between gap-2">
              <h3 className="text-xs font-mono font-bold text-cyan-300">
                {t('express.sdMission.step2Title')}
              </h3>
              <Button
                type="button"
                size="sm"
                variant="ghost"
                disabled={sdTargetsLoading}
                onClick={onRefreshTargets}
                className="h-6 px-2 text-[11px] gap-1"
              >
                <RefreshCw className={sdTargetsLoading ? 'w-3 h-3 animate-spin' : 'w-3 h-3'} />
                <span>{t('express.sdMission.refreshTargetsBtn')}</span>
              </Button>
            </div>
            <ExpressModeChoiceGroup
              activeValue={sdTargetMode}
              activeVariant="emerald"
              onSelect={onSelectTargetMode}
              options={[
                { value: 'image', label: t('express.sdMission.targetModeImg'), testId: 'express-sd-mode-image' },
                { value: 'physical', label: t('express.sdMission.targetModePhysical'), testId: 'express-sd-mode-physical' },
              ]}
            />
            {sdTargetMode === 'physical' && (
              <SdPhysicalDrivePicker
                sdTargets={sdTargets}
                selectedDevicePath={selectedDevicePath}
                onSelectDevicePath={onSelectDevicePath}
                selectedSdDrive={selectedSdDrive}
                sdConfirmToken={sdConfirmToken}
                onChangeConfirmToken={onChangeConfirmToken}
              />
            )}
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 flex flex-col justify-between gap-3">
            <div className="space-y-2">
              <h3 className="text-xs font-mono font-bold text-cyan-300">
                {t('express.sdMission.step3Title')}
              </h3>
              {sdProgress && (
                <div className="space-y-1">
                  <div className="flex justify-between text-[11px] font-mono text-cyan-300">
                    <span className="truncate">{sdProgress.detail}</span>
                    <span>{progressPct}%</span>
                  </div>
                  <progress value={progressPct} max={PCT_FULL} className="w-full h-2 rounded-full bg-slate-900 accent-emerald-400" />
                </div>
              )}
              {sdError && (
                <Alert variant="destructive" className="py-2">
                  <AlertDescription className="text-xs font-mono">{sdError}</AlertDescription>
                </Alert>
              )}
            </div>

            <Button
              type="button"
              variant="emerald"
              data-testid="express-sd-flash-btn"
              disabled={sdFlashing || confirmMismatch}
              onClick={onExecuteFlash}
              className="w-full gap-2"
            >
              {sdFlashing ? <Loader2 className="w-4 h-4 animate-spin" /> : <HardDrive className="w-4 h-4" />}
              <span>{actionLabel}</span>
            </Button>
          </div>
        </div>

        {sdReport && (
          <div className="space-y-4">
            <Alert variant="success" data-testid="express-sd-success">
              <CheckCircle2 className="h-4 w-4 text-emerald-400" />
              <AlertTitle>{t('express.sdMission.successTitle')}</AlertTitle>
              <AlertDescription className="space-y-1 text-xs">
                <div className="font-mono break-all">{sdReport.target}</div>
                <ul className="list-disc pl-4 text-slate-200 space-y-0.5 pt-1">
                  {sdReport.next_steps.map(step => (
                    <li key={step}>{step}</li>
                  ))}
                </ul>
              </AlertDescription>
            </Alert>
            <RecorderDisplayGuide />
          </div>
        )}

        <TechnicalDetailsDrawer rows={techRows} onOpenEngineering={onOpenEngineering} testIdPrefix="express-sd-tech" />
      </CardContent>
    </Card>
  );
};
