import React from 'react';
import { HardDrive, ShieldAlert, Wrench } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import { SafetyPreflightCard, Sha256ArtifactCard } from '@/components/pipelines';
import { Alert, AlertDescription, Badge, Button, Input } from '@/components/ui';
import { HEX32_PAD_WIDTH } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { HardenSdUnbrickKitResponse, PreflightCheckItem, SdDiskInspectionReport } from '@/models';

export interface SdHardenSectionProps {
  readonly sdOutputDir: string;
  readonly setSdOutputDir: (v: string) => void;
  readonly panaDvdPath: string;
  readonly setPanaDvdPath: (v: string) => void;
  readonly prog061Path: string;
  readonly setProg061Path: (v: string) => void;
  readonly allowFactoryProg061: boolean;
  readonly setAllowFactoryProg061: (v: boolean) => void;
  readonly sdDiskTarget: string;
  readonly setSdDiskTarget: (v: string) => void;
  readonly sdDiskReport: SdDiskInspectionReport | null;
  readonly sdDiskBusy: boolean;
  readonly auditSdDisk: () => Promise<SdDiskInspectionReport | null>;
  readonly sdGate: { readonly blocked: boolean; readonly reason: string | null };
  readonly checkedPreflightIds: readonly string[];
  readonly togglePreflight: (id: string) => void;
  readonly preflightChecks: readonly PreflightCheckItem[];
  readonly preflightReady: boolean;
  readonly hardenResult: HardenSdUnbrickKitResponse | null;
  readonly hardenBusy: boolean;
  readonly hardenError: string | null;
  readonly runHardenSdKit: () => Promise<HardenSdUnbrickKitResponse | null>;
  readonly hideSharedInputs?: boolean;
}

export const SdHardenSection: React.FC<SdHardenSectionProps> = (props) => {
  const { t } = useTranslation();
  const outInvalid = !props.sdOutputDir.trim();
  const parity = props.hardenResult?.dualFamilyParity;
  const verReport = props.hardenResult?.hardenedVerification;
  const slotProg = parity?.m6930lb_prog061;
  const fvuHex = slotProg
    ? slotProg.fvu_checksum_u32.toString(16).toUpperCase().padStart(HEX32_PAD_WIDTH, '0')
    : null;
  const mediaVerified = Boolean(
    verReport?.digadiag_present &&
      verReport.digadiag_exact_size_ok &&
      verReport.pana_sd_list_sums_ok,
  );

  return (
    <div className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-4 font-mono">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 pb-2.5">
        <div>
          <h4 className="text-xs font-bold text-slate-100">{t('pipelines.tbcSd.sdHardenTitle')}</h4>
          <p className="text-[11px] text-slate-400 font-sans mt-0.5">{t('pipelines.tbcSd.sdHardenDesc')}</p>
        </div>
        {props.hardenResult?.factoryProg061Blocked && (
          <Badge variant="destructive">{t('pipelines.tbcSd.factoryBlockedBadge')}</Badge>
        )}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
        {!props.hideSharedInputs && (
          <>
            <div className="space-y-1">
              <label htmlFor="sd-harden-out" className="text-slate-400 block">{t('pipelines.tbcSd.sdOutputDir')}</label>
              <Input id="sd-harden-out" type="text" value={props.sdOutputDir} aria-invalid={outInvalid} onChange={e => props.setSdOutputDir(e.target.value)} />
            </div>
            <div className="space-y-1">
              <label htmlFor="sd-harden-dvd" className="text-slate-400 block">{t('pipelines.tbcSd.panaDvdPath')}</label>
              <Input id="sd-harden-dvd" type="text" value={props.panaDvdPath} onChange={e => props.setPanaDvdPath(e.target.value)} />
            </div>
          </>
        )}
        <div className="space-y-1">
          <label htmlFor="sd-harden-prog061" className="text-slate-400 block">{t('pipelines.tbcSd.prog061Path')}</label>
          <Input id="sd-harden-prog061" data-testid="sd-prog061-input" type="text" value={props.prog061Path} onChange={e => props.setProg061Path(e.target.value)} />
        </div>
        <div className="space-y-1">
          <label htmlFor="sd-gate-target" className="text-slate-400 block">{t('pipelines.tbcSd.sdDiskTarget')}</label>
          <div className="flex gap-2">
            <Input id="sd-gate-target" data-testid="sd-gate-target" type="text" value={props.sdDiskTarget} onChange={e => props.setSdDiskTarget(e.target.value)} />
            <Button type="button" size="sm" variant="outline" data-testid="sd-audit-disk-btn" disabled={props.sdDiskBusy || !props.sdDiskTarget.trim()} onClick={() => void props.auditSdDisk()} className="shrink-0 gap-1">
              <HardDrive className="w-3.5 h-3.5" />
              <span>{t('pipelines.tbcSd.auditSdBtn')}</span>
            </Button>
          </div>
        </div>
      </div>

      <label className="flex items-center gap-2 text-xs text-amber-300 cursor-pointer">
        <input type="checkbox" checked={props.allowFactoryProg061} onChange={e => props.setAllowFactoryProg061(e.target.checked)} />
        <span>{t('pipelines.tbcSd.allowFactoryProg061')}</span>
      </label>

      {props.sdGate.reason && (
        <Alert variant={props.sdGate.blocked ? 'destructive' : 'success'}>
          <ShieldAlert className="h-4 w-4" />
          <AlertDescription>{props.sdGate.reason}</AlertDescription>
        </Alert>
      )}

      <SafetyPreflightCard checks={props.preflightChecks} checkedIds={props.checkedPreflightIds} onToggleCheck={props.togglePreflight} severity="critical" />

      {props.hardenError && (
        <Alert variant="destructive"><AlertDescription>{props.hardenError}</AlertDescription></Alert>
      )}

      <Button type="button" variant="emerald" size="sm" data-testid="sd-harden-build-btn" disabled={props.hardenBusy || outInvalid || !props.preflightReady} onClick={() => void props.runHardenSdKit()} className="gap-1.5">
        <Wrench className="w-3.5 h-3.5" />
        <span>{t('pipelines.tbcSd.hardenBtn')}</span>
      </Button>

      {props.hardenResult && (
        <div className="space-y-3">
          {parity && (
            <div className="p-3 rounded-lg bg-bench-900 border border-slate-800 flex flex-wrap items-center justify-between gap-2 text-xs">
              <span className="font-bold text-slate-200">{t('pipelines.tbcSd.parityTitle')}</span>
              <div className="flex items-center gap-2">
                {fvuHex && <Badge variant="secondary">{t('pipelines.tbcSd.fvuChecksum', { hex: fvuHex })}</Badge>}
                <StatusBadge
                  variant={parity.dual_parity_ok ? 'success' : 'error'}
                  label={parity.dual_parity_ok ? t('pipelines.tbcSd.parityOk') : t('pipelines.tbcSd.parityFail')}
                />
              </div>
            </div>
          )}
          <Sha256ArtifactCard
            outputPath={props.hardenResult.kitReport.digadiag_path}
            outputSha256={props.hardenResult.kitReport.digadiag_sha256}
            verified={mediaVerified}
          />
        </div>
      )}
    </div>
  );
};
