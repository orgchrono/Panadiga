import React from 'react';
import { CheckCircle2, Loader2, Radio, RefreshCw, ShieldCheck } from 'lucide-react';
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
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { NvramCleanProgressEvent, PicoPanamon1CleanNvramResponse } from '@/models';
import { TechnicalDetailsDrawer, type TechnicalDetailRow } from './TechnicalDetailsDrawer';

const PCT_FULL = 100;

export interface U99QuickRescueMissionProps {
  readonly picoPorts: readonly string[];
  readonly picoPortsLoading: boolean;
  readonly picoPortInput: string;
  readonly onChangePortInput: (port: string) => void;
  readonly onRefreshPorts: () => void;
  readonly u99Cleaning: boolean;
  readonly u99Progress: NvramCleanProgressEvent | null;
  readonly u99Error: string | null;
  readonly u99Report: PicoPanamon1CleanNvramResponse | null;
  readonly onExecuteRescue: () => void;
  readonly onOpenEngineering: () => void;
}

export const U99QuickRescueMission: React.FC<U99QuickRescueMissionProps> = ({
  picoPorts,
  picoPortsLoading,
  picoPortInput,
  onChangePortInput,
  onRefreshPorts,
  u99Cleaning,
  u99Progress,
  u99Error,
  u99Report,
  onExecuteRescue,
  onOpenEngineering,
}) => {
  const { t } = useTranslation();

  const progressPct =
    u99Progress && u99Progress.total > 0
      ? Math.min(PCT_FULL, Math.round((u99Progress.current / u99Progress.total) * PCT_FULL))
      : 0;

  const techRows: readonly TechnicalDetailRow[] = u99Report
    ? [
        { label: t('express.technical.backupLabel'), value: u99Report.backupPath },
        {
          label: `${t('express.technical.sha256Label')} (Before)`,
          value: u99Report.sha256Before,
        },
        {
          label: `${t('express.technical.sha256Label')} (0xFF After)`,
          value: u99Report.sha256After,
        },
        {
          label: t('express.technical.geometryLabel'),
          value: `SA134..SA141 @ 0x847F0000 (+${u99Report.bytesErased} B)`,
        },
      ]
    : [];

  return (
    <Card data-testid="express-mission-u99" className="border-slate-800 bg-bench-900/90 shadow-xl">
      <CardHeader>
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div>
            <CardTitle className="flex items-center gap-2 text-base text-slate-100">
              <Radio className="w-5 h-5 text-cyan-400" />
              <span>{t('express.u99Mission.title')}</span>
            </CardTitle>
            <CardDescription className="mt-1">{t('express.u99Mission.desc')}</CardDescription>
          </div>
          <Badge variant="secondary" className="font-mono text-[10px]">
            {t('express.u99Mission.badgeSafe')}
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="space-y-5">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          {/* Step 1: COM Port */}
          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-3">
            <div className="flex items-center justify-between gap-2">
              <h3 className="text-xs font-mono font-bold text-cyan-300">
                {t('express.u99Mission.step1Title')}
              </h3>
              <Button
                type="button"
                size="sm"
                variant="ghost"
                disabled={picoPortsLoading}
                onClick={onRefreshPorts}
                className="h-6 px-2 text-[11px] gap-1"
              >
                <RefreshCw className={picoPortsLoading ? 'w-3 h-3 animate-spin' : 'w-3 h-3'} />
                <span>{t('express.u99Mission.refreshPortsBtn')}</span>
              </Button>
            </div>

            <div className="space-y-1.5">
              <label htmlFor="express-u99-port" className="text-[11px] font-mono text-slate-400">
                {t('express.u99Mission.portLabel')}
              </label>
              <Input
                id="express-u99-port"
                data-testid="express-u99-port-input"
                value={picoPortInput}
                onChange={e => onChangePortInput(e.target.value)}
              />
            </div>

            {picoPorts.length === 0 ? (
              <EmptyState compact title={t('express.u99Mission.emptyPortsHint')} />
            ) : (
              <div className="flex flex-wrap gap-1.5">
                {picoPorts.map(port => (
                  <Button
                    key={port}
                    type="button"
                    size="sm"
                    variant={picoPortInput === port ? 'default' : 'outline'}
                    onClick={() => onChangePortInput(port)}
                    className="h-7 px-2 text-xs font-mono"
                  >
                    {port}
                  </Button>
                ))}
              </div>
            )}
          </div>

          {/* Step 2: Safety & Auto Backup */}
          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-2.5">
            <h3 className="text-xs font-mono font-bold text-cyan-300">
              {t('express.u99Mission.step2Title')}
            </h3>
            <p className="text-xs text-slate-300 leading-relaxed font-sans">
              {t('express.u99Mission.backupSummary')}
            </p>
            <p className="text-[11px] text-amber-300/90 font-sans">
              {t('express.header.safetyGateNote')}
            </p>
          </div>

          {/* Step 3: Run */}
          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 flex flex-col justify-between gap-3">
            <div className="space-y-2">
              <h3 className="text-xs font-mono font-bold text-cyan-300">
                {t('express.u99Mission.step3Title')}
              </h3>
              {u99Progress && (
                <div className="space-y-1">
                  <div className="flex justify-between text-[11px] font-mono text-cyan-300">
                    <span className="truncate">{u99Progress.detail}</span>
                    <span>{progressPct}%</span>
                  </div>
                  <progress value={progressPct} max={PCT_FULL} className="w-full h-2 rounded-full bg-slate-900 accent-cyan-400" />
                </div>
              )}
              {u99Error && (
                <Alert variant="destructive" className="py-2">
                  <AlertDescription className="text-xs font-mono">{u99Error}</AlertDescription>
                </Alert>
              )}
            </div>

            <Button
              type="button"
              variant="emerald"
              data-testid="express-u99-run-btn"
              disabled={u99Cleaning || !picoPortInput.trim()}
              onClick={onExecuteRescue}
              className="w-full gap-2"
            >
              {u99Cleaning ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <ShieldCheck className="w-4 h-4" />
              )}
              <span>
                {u99Cleaning ? t('express.u99Mission.runningBtn') : t('express.u99Mission.runBtn')}
              </span>
            </Button>
          </div>
        </div>

        {u99Report && (
          <Alert variant="success" data-testid="express-u99-success">
            <CheckCircle2 className="h-4 w-4 text-emerald-400" />
            <AlertTitle>{t('express.u99Mission.successTitle')}</AlertTitle>
            <AlertDescription className="space-y-1 text-xs">
              <div>
                {t('express.u99Mission.erasedSectorsLabel', {
                  count: u99Report.erasedSectors,
                  bytes: u99Report.bytesErased,
                })}{' '}
                •{' '}
                {t('express.u99Mission.verifiedFfLabel', {
                  status: u99Report.verifiedAllFf ? 'PASS' : 'FAIL',
                })}
              </div>
              <div className="text-slate-200">{u99Report.nextStep}</div>
            </AlertDescription>
          </Alert>
        )}

        <TechnicalDetailsDrawer
          rows={techRows}
          onOpenEngineering={onOpenEngineering}
          testIdPrefix="express-u99-tech"
        />
      </CardContent>
    </Card>
  );
};
