import React from 'react';
import {
  CheckCircle2,
  Cpu,
  Disc3,
  HardDrive,
  Loader2,
  Lock,
  RefreshCw,
  ShieldCheck,
} from 'lucide-react';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
  EmptyState,
  Input,
  Select,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  ComposedHddFlashReport,
  DvdBurnProgressEvent,
  DvdCreatorReport,
  OpticalBurnerDriveInfo,
  PreWriteGateReport,
  SafeTargetDiskInfo,
} from '@/models';
import { ExpressModeChoiceGroup } from './ExpressModeChoiceGroup';

const PCT_FULL = 100;
const SHA_PREVIEW_LEN = 16;

export interface VaultMediaAndGatePanelProps {
  readonly opticalBurners: readonly OpticalBurnerDriveInfo[];
  readonly dvdTargetMode: 'iso' | 'burner';
  readonly onSelectDvdTargetMode: (mode: 'iso' | 'burner') => void;
  readonly selectedBurnerId: string;
  readonly onSelectBurnerId: (id: string) => void;
  readonly dvdConfirmToken: string;
  readonly onChangeDvdConfirmToken: (tok: string) => void;
  readonly dvdBusy: boolean;
  readonly dvdProgress: DvdBurnProgressEvent | null;
  readonly dvdReport: DvdCreatorReport | null;
  readonly dvdError: string | null;
  readonly onRefreshBurners: () => void;
  readonly onExecuteDvdCreate: () => void;
  readonly emuGateBusy: boolean;
  readonly emuGateReport: PreWriteGateReport | null;
  readonly emuGateError: string | null;
  readonly onExecuteMandatoryEmuGate: () => void;
  readonly hddTargets: readonly SafeTargetDiskInfo[];
  readonly hddDevicePath: string;
  readonly onChangeHddDevicePath: (path: string) => void;
  readonly hddConfirmToken: string;
  readonly onChangeHddConfirmToken: (tok: string) => void;
  readonly hddBusy: boolean;
  readonly hddReport: ComposedHddFlashReport | null;
  readonly hddError: string | null;
  readonly onExecuteComposedHddFlash: () => void;
}

const DvdCreatorSection: React.FC<
  Pick<
    VaultMediaAndGatePanelProps,
    | 'opticalBurners'
    | 'dvdTargetMode'
    | 'onSelectDvdTargetMode'
    | 'selectedBurnerId'
    | 'onSelectBurnerId'
    | 'dvdConfirmToken'
    | 'onChangeDvdConfirmToken'
    | 'dvdBusy'
    | 'dvdProgress'
    | 'dvdReport'
    | 'dvdError'
    | 'onRefreshBurners'
    | 'onExecuteDvdCreate'
  >
> = props => {
  const { t } = useTranslation();
  const pct = props.dvdProgress ? Math.min(PCT_FULL, props.dvdProgress.percent) : 0;

  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-3">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <Disc3 className="w-4 h-4 text-cyan-400" />
          <h3 className="text-xs font-mono font-bold text-cyan-300">{t('express.dvd.title')}</h3>
        </div>
        <Button type="button" size="sm" variant="ghost" onClick={props.onRefreshBurners} className="h-6 px-2 text-[11px] gap-1">
          <RefreshCw className="w-3 h-3" />
          <span>{t('express.dvd.scanBurnersBtn')}</span>
        </Button>
      </div>
      <p className="text-[11px] text-slate-400">{t('express.dvd.desc')}</p>

      <ExpressModeChoiceGroup
        activeValue={props.dvdTargetMode}
        activeVariant="emerald"
        onSelect={props.onSelectDvdTargetMode}
        options={[
          { value: 'iso', label: t('express.dvd.modeIso'), testId: 'express-dvd-mode-iso' },
          { value: 'burner', label: t('express.dvd.modeBurner'), testId: 'express-dvd-mode-burner' },
        ]}
      />

      {props.dvdTargetMode === 'burner' && (
        <div className="space-y-2">
          {props.opticalBurners.length === 0 ? (
            <EmptyState compact title={t('express.dvd.noBurnersFound')} />
          ) : (
            <Select
              aria-label={t('express.dvd.modeBurner')}
              data-testid="express-dvd-burner-select"
              value={props.selectedBurnerId}
              onChange={e => props.onSelectBurnerId(e.target.value)}
            >
              {props.opticalBurners.map(b => (
                <option key={b.deviceId} value={b.deviceId}>
                  {b.displayName} • {b.minWriteSpeedLabel}
                </option>
              ))}
            </Select>
          )}
          <Input
            aria-label={t('express.dvd.confirmLabel')}
            data-testid="express-dvd-confirm-input"
            value={props.dvdConfirmToken}
            onChange={e => props.onChangeDvdConfirmToken(e.target.value)}
            placeholder="CONFIRM-sr0"
          />
        </div>
      )}

      {props.dvdProgress && (
        <div className="space-y-1">
          <div className="flex justify-between text-[11px] font-mono text-cyan-300">
            <span className="truncate">{props.dvdProgress.detail}</span>
            <span>{pct}%</span>
          </div>
          <progress value={pct} max={PCT_FULL} className="w-full h-2 rounded-full bg-slate-900 accent-cyan-400" />
        </div>
      )}
      {props.dvdError && (
        <Alert variant="destructive" className="py-2">
          <AlertDescription className="text-xs font-mono">{props.dvdError}</AlertDescription>
        </Alert>
      )}

      <Button
        type="button"
        variant="secondary"
        data-testid="express-dvd-create-btn"
        disabled={props.dvdBusy}
        onClick={props.onExecuteDvdCreate}
        className="w-full gap-2"
      >
        {props.dvdBusy ? <Loader2 className="w-4 h-4 animate-spin" /> : <Disc3 className="w-4 h-4" />}
        <span>{props.dvdTargetMode === 'burner' ? t('express.dvd.burnBtn') : t('express.dvd.createIsoBtn')}</span>
      </Button>

      {props.dvdReport && (
        <Alert variant="success" data-testid="express-dvd-success" className="py-2">
          <CheckCircle2 className="h-4 w-4 text-emerald-400" />
          <AlertTitle className="text-xs">{t('express.dvd.successTitle')}</AlertTitle>
          <AlertDescription className="text-[11px] font-mono break-all">
            {props.dvdReport.volumeLabel} • {props.dvdReport.writeSpeedPolicy} • {props.dvdReport.inspection.overall_status}
          </AlertDescription>
        </Alert>
      )}
    </div>
  );
};

const PreWriteGateHddSection: React.FC<
  Pick<
    VaultMediaAndGatePanelProps,
    | 'emuGateBusy'
    | 'emuGateReport'
    | 'emuGateError'
    | 'onExecuteMandatoryEmuGate'
    | 'hddTargets'
    | 'hddDevicePath'
    | 'onChangeHddDevicePath'
    | 'hddConfirmToken'
    | 'onChangeHddConfirmToken'
    | 'hddBusy'
    | 'hddReport'
    | 'hddError'
    | 'onExecuteComposedHddFlash'
  >
> = props => {
  const { t } = useTranslation();
  const gatePassed = props.emuGateReport?.passed ?? false;
  const writeLocked =
    !gatePassed || props.hddBusy || !props.hddDevicePath.trim() || !props.hddConfirmToken.trim();
  const safeDisks = props.hddTargets.filter(d => d.safe_to_flash);

  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-3">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <Cpu className="w-4 h-4 text-amber-400" />
          <h3 className="text-xs font-mono font-bold text-amber-300">{t('express.gate.title')}</h3>
        </div>
        <Badge data-testid="express-vault-gate-status" variant={gatePassed ? 'success' : 'warning'} className="text-[10px] font-mono">
          {gatePassed ? t('express.gate.statusPass') : t('express.gate.statusLocked')}
        </Badge>
      </div>
      <p className="text-[11px] text-slate-400">{t('express.gate.desc')}</p>

      <Button
        type="button"
        variant="default"
        data-testid="express-vault-run-gate-btn"
        disabled={props.emuGateBusy}
        onClick={props.onExecuteMandatoryEmuGate}
        className="w-full gap-2"
      >
        {props.emuGateBusy ? <Loader2 className="w-4 h-4 animate-spin" /> : <ShieldCheck className="w-4 h-4" />}
        <span>{t('express.gate.runBtn')}</span>
      </Button>

      {props.emuGateReport && (
        <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-2.5 text-[11px] font-mono text-emerald-200 space-y-1">
          <div>
            {t('express.gate.summaryLine1', {
              ipl: String(props.emuGateReport.iplUntouched),
              mmio: String(props.emuGateReport.lethalMmioBlocked),
            })}
          </div>
          <div>
            {t('express.gate.summaryLine2', {
              steps: props.emuGateReport.executedSteps,
              sha: props.emuGateReport.artifactSha256.slice(0, SHA_PREVIEW_LEN),
            })}
          </div>
        </div>
      )}
      {props.emuGateError && (
        <Alert variant="destructive" className="py-2">
          <AlertDescription className="text-xs font-mono">{props.emuGateError}</AlertDescription>
        </Alert>
      )}

      <div className="border-t border-slate-800 pt-3 space-y-2">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {safeDisks.length > 0 ? (
            <Select
              aria-label={t('express.gate.hddTargetLabel')}
              data-testid="express-vault-hdd-select"
              value={props.hddDevicePath}
              onChange={e => props.onChangeHddDevicePath(e.target.value)}
            >
              {safeDisks.map(d => (
                <option key={d.device_path} value={d.device_path}>
                  {d.display_name} ({d.device_path})
                </option>
              ))}
            </Select>
          ) : (
            <Input
              aria-label={t('express.gate.hddTargetLabel')}
              data-testid="express-vault-hdd-input"
              value={props.hddDevicePath}
              onChange={e => props.onChangeHddDevicePath(e.target.value)}
              placeholder="\\\\.\\PhysicalDrive2"
            />
          )}
          <Input
            aria-label={t('express.gate.hddConfirmLabel')}
            data-testid="express-vault-hdd-confirm-input"
            value={props.hddConfirmToken}
            onChange={e => props.onChangeHddConfirmToken(e.target.value)}
            placeholder="CONFIRM-PhysicalDrive2"
          />
        </div>

        {props.hddError && (
          <Alert variant="destructive" className="py-2">
            <AlertDescription className="text-xs font-mono">{props.hddError}</AlertDescription>
          </Alert>
        )}

        <Button
          type="button"
          variant="emerald"
          data-testid="express-vault-write-btn"
          disabled={writeLocked}
          onClick={props.onExecuteComposedHddFlash}
          className="w-full gap-2"
        >
          {!gatePassed ? <Lock className="w-4 h-4" /> : props.hddBusy ? <Loader2 className="w-4 h-4 animate-spin" /> : <HardDrive className="w-4 h-4" />}
          <span>{t('express.gate.writeBtn')}</span>
        </Button>

        {props.hddReport && (
          <Alert variant="success" data-testid="express-vault-write-success" className="py-2">
            <CheckCircle2 className="h-4 w-4 text-emerald-400" />
            <AlertTitle className="text-xs">{t('express.gate.hddWriteDone')}</AlertTitle>
            <AlertDescription className="text-[11px] font-mono">
              {t('express.gate.writeSummary', {
                readback: String(props.hddReport.readbackVerified),
                bytes: props.hddReport.p1MainBytesWritten,
              })}
            </AlertDescription>
          </Alert>
        )}
      </div>
    </div>
  );
};

export const VaultMediaAndGatePanel: React.FC<VaultMediaAndGatePanelProps> = props => (
  <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
    <DvdCreatorSection {...props} />
    <PreWriteGateHddSection {...props} />
  </div>
);
