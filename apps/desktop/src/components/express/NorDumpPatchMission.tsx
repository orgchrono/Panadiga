import React from 'react';
import { CheckCircle2, Cpu, Loader2, ShieldCheck, Wrench } from 'lucide-react';
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
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  Eh55BgoAuditReport,
  NorDumpProgressEvent,
  NorRepairExpressOutcome,
  NvramCleanReport,
  PicoEh55NorDumpResponse,
} from '@/models';
import { ExpressModeChoiceGroup } from './ExpressModeChoiceGroup';
import { TechnicalDetailsDrawer, type TechnicalDetailRow } from './TechnicalDetailsDrawer';

const PCT_FULL = 100;

export interface NorDumpPatchMissionProps {
  readonly norSourceMode: 'local' | 'pico';
  readonly onSelectSourceMode: (mode: 'local' | 'pico') => void;
  readonly norCapturing: boolean;
  readonly norProgress: NorDumpProgressEvent | null;
  readonly norDumpReport: PicoEh55NorDumpResponse | null;
  readonly norAuditing: boolean;
  readonly norAuditNvram: NvramCleanReport | null;
  readonly norAuditBgo: Eh55BgoAuditReport | null;
  readonly norPatching: boolean;
  readonly norRepairResult: NorRepairExpressOutcome | null;
  readonly norError: string | null;
  readonly onExecuteInspect: () => void;
  readonly onExecuteSafeRepair: (liveUpperNor?: boolean) => void;
  readonly onOpenEngineering: () => void;
}

const NorThreeZoneHealthCard: React.FC<{
  readonly hasAudit: boolean;
  readonly norAuditNvram: NvramCleanReport | null;
  readonly norAuditBgo: Eh55BgoAuditReport | null;
}> = ({ hasAudit, norAuditNvram, norAuditBgo }) => {
  const { t } = useTranslation();
  const nvramDirty = (norAuditNvram?.nvram_non_ff_bytes ?? 0) > 0;
  const bgoHealthy = norAuditBgo?.healthy ?? true;

  if (!hasAudit) {
    return <EmptyState compact title={t('express.norMission.emptyHealth')} />;
  }

  return (
    <div className="space-y-2 text-xs">
      <div className="rounded-lg border border-emerald-500/30 bg-emerald-950/20 p-2.5">
        <div className="font-mono font-bold text-emerald-300">
          {t('express.norMission.zoneIplTitle')}
        </div>
        <div className="text-slate-300">{t('express.norMission.zoneIplOk')}</div>
      </div>
      <div
        className={
          bgoHealthy
            ? 'rounded-lg border border-emerald-500/30 bg-emerald-950/20 p-2.5'
            : 'rounded-lg border border-amber-500/40 bg-amber-950/20 p-2.5'
        }
      >
        <div className="font-mono font-bold text-slate-100">
          {t('express.norMission.zoneBgoTitle')}
        </div>
        <div className="text-slate-300">
          {bgoHealthy
            ? t('express.norMission.zoneBgoOk', {
                p2: norAuditBgo?.p2_present_ids.length ?? 0,
                corrupt: norAuditBgo?.corrupt_sas.length ?? 0,
              })
            : t('express.norMission.zoneBgoWarn', {
                corrupt: norAuditBgo?.corrupt_sas.length ?? 0,
                missing: norAuditBgo?.p2_missing_ids.length ?? 0,
              })}
        </div>
      </div>
      <div
        className={
          nvramDirty
            ? 'rounded-lg border border-rose-500/40 bg-rose-950/20 p-2.5'
            : 'rounded-lg border border-emerald-500/30 bg-emerald-950/20 p-2.5'
        }
      >
        <div className="font-mono font-bold text-slate-100">
          {t('express.norMission.zoneNvramTitle')}
        </div>
        <div className="text-slate-300">
          {nvramDirty
            ? t('express.norMission.zoneNvramWarn', {
                bytes: norAuditNvram?.nvram_non_ff_bytes ?? 0,
              })
            : t('express.norMission.zoneNvramOk')}
        </div>
      </div>
    </div>
  );
};

function buildNorTechRows(
  t: ReturnType<typeof useTranslation>['t'],
  norDumpReport: PicoEh55NorDumpResponse | null,
  norAuditNvram: NvramCleanReport | null,
  norRepairResult: NorRepairExpressOutcome | null,
): readonly TechnicalDetailRow[] {
  const dumpRows = norDumpReport
    ? [
        { label: t('express.technical.backupLabel'), value: norDumpReport.outputPath },
        { label: t('express.technical.sha256Label'), value: norDumpReport.sha256 },
      ]
    : [];
  const nvramRows = norAuditNvram
    ? [
        {
          label: t('express.technical.geometryLabel'),
          value: `0x${norAuditNvram.nvram_offset_start.toString(16).toUpperCase()}..0x${norAuditNvram.nvram_offset_end.toString(16).toUpperCase()}`,
        },
      ]
    : [];
  const repairRows = norRepairResult
    ? [
        { label: 'CoW Output', value: norRepairResult.summaryPath },
        { label: `${t('express.technical.sha256Label')} (Post)`, value: norRepairResult.sha256 },
      ]
    : [];
  return [...dumpRows, ...nvramRows, ...repairRows];
}

export const NorDumpPatchMission: React.FC<NorDumpPatchMissionProps> = ({
  norSourceMode,
  onSelectSourceMode,
  norCapturing,
  norProgress,
  norDumpReport,
  norAuditing,
  norAuditNvram,
  norAuditBgo,
  norPatching,
  norRepairResult,
  norError,
  onExecuteInspect,
  onExecuteSafeRepair,
  onOpenEngineering,
}) => {
  const { t } = useTranslation();
  const isInspecting = norCapturing || norAuditing;
  const hasAudit = Boolean(norAuditNvram || norAuditBgo);
  const progressPct =
    norProgress && norProgress.total > 0
      ? Math.min(PCT_FULL, Math.round((norProgress.current / norProgress.total) * PCT_FULL))
      : 0;
  const techRows = buildNorTechRows(t, norDumpReport, norAuditNvram, norRepairResult);
  const inspectLabel = isInspecting
    ? t('express.norMission.inspectingBtn')
    : norSourceMode === 'pico'
      ? t('express.norMission.captureInspectBtn')
      : t('express.norMission.inspectBtn');

  return (
    <Card data-testid="express-mission-nor" className="border-slate-800 bg-bench-900/90 shadow-xl">
      <CardHeader>
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div>
            <CardTitle className="flex items-center gap-2 text-base text-slate-100">
              <Cpu className="w-5 h-5 text-cyan-400" />
              <span>{t('express.norMission.title')}</span>
            </CardTitle>
            <CardDescription className="mt-1">{t('express.norMission.desc')}</CardDescription>
          </div>
          <Badge variant="outline" className="font-mono text-[10px] border-amber-500/40 text-amber-300">
            {t('express.norMission.badgeCow')}
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="space-y-5">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-3">
            <h3 className="text-xs font-mono font-bold text-cyan-300">
              {t('express.norMission.step1Title')}
            </h3>
            <ExpressModeChoiceGroup
              activeValue={norSourceMode}
              onSelect={onSelectSourceMode}
              options={[
                { value: 'local', label: t('express.norMission.sourceLocalBtn'), testId: 'express-nor-source-local' },
                { value: 'pico', label: t('express.norMission.sourcePicoBtn'), testId: 'express-nor-source-pico' },
              ]}
            />
            {norProgress && (
              <div className="space-y-1">
                <div className="flex justify-between text-[11px] font-mono text-cyan-300">
                  <span className="truncate">{norProgress.stage}</span>
                  <span>{progressPct}%</span>
                </div>
                <progress value={progressPct} max={PCT_FULL} className="w-full h-2 rounded-full bg-slate-900 accent-cyan-400" />
              </div>
            )}
            <Button
              type="button"
              variant="outline"
              data-testid="express-nor-inspect-btn"
              disabled={isInspecting}
              onClick={onExecuteInspect}
              className="w-full gap-2 border-cyan-500/40 text-cyan-200"
            >
              {isInspecting ? <Loader2 className="w-4 h-4 animate-spin" /> : <ShieldCheck className="w-4 h-4" />}
              <span>{inspectLabel}</span>
            </Button>
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-2.5">
            <h3 className="text-xs font-mono font-bold text-cyan-300">
              {t('express.norMission.step2Title')}
            </h3>
            <NorThreeZoneHealthCard hasAudit={hasAudit} norAuditNvram={norAuditNvram} norAuditBgo={norAuditBgo} />
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 flex flex-col justify-between gap-3">
            <div className="space-y-2">
              <h3 className="text-xs font-mono font-bold text-cyan-300">
                {t('express.norMission.step3Title')}
              </h3>
              {norError && (
                <Alert variant="destructive" className="py-2">
                  <AlertDescription className="text-xs font-mono">{norError}</AlertDescription>
                </Alert>
              )}
              {norRepairResult && (
                <Alert variant="success" data-testid="express-nor-success" className="py-2">
                  <CheckCircle2 className="h-4 w-4 text-emerald-400" />
                  <AlertTitle>{t('express.norMission.successTitle')}</AlertTitle>
                  <AlertDescription className="text-[11px] font-mono break-all">
                    {norRepairResult.summaryPath}
                  </AlertDescription>
                </Alert>
              )}
            </div>

            <Button
              type="button"
              variant="emerald"
              data-testid="express-nor-repair-btn"
              disabled={norPatching || !hasAudit}
              onClick={() => onExecuteSafeRepair(false)}
              className="w-full gap-2"
            >
              {norPatching ? <Loader2 className="w-4 h-4 animate-spin" /> : <Wrench className="w-4 h-4" />}
              <span>{norPatching ? t('express.norMission.repairingBtn') : t('express.norMission.repairBtn')}</span>
            </Button>
          </div>
        </div>

        <TechnicalDetailsDrawer rows={techRows} onOpenEngineering={onOpenEngineering} testIdPrefix="express-nor-tech" />
      </CardContent>
    </Card>
  );
};
