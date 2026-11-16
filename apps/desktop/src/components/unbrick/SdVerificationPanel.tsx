import React, { useState } from 'react';
import { HardDrive, Loader2, ShieldCheck } from 'lucide-react';
import { Alert, AlertDescription, Badge, Button, EmptyState, Input } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import {
  resolveSdOverallBadgeVariant,
  resolveSeverityBadgeVariant,
  validateRequiredText,
  type SdDiskVerificationProps,
} from './unbrickDomain';

interface SdVerificationPanelProps extends SdDiskVerificationProps {
  verifyDir: string;
  onChangeVerifyDir: (val: string) => void;
}

export const SdVerificationPanel: React.FC<SdVerificationPanelProps> = ({
  verifyDir,
  onChangeVerifyDir,
  verifyLoading,
  verifyResult,
  verifyError,
  onVerifyMedia,
  diskLoading = false,
  diskResult = null,
  diskError = null,
  onInspectSdDisk,
}) => {
  const { t } = useTranslation();
  const [verifyDirError, setVerifyDirError] = useState<string | null>(null);
  const [diskTarget, setDiskTarget] = useState('E:');
  const [diskTargetError, setDiskTargetError] = useState<string | null>(null);
  const checks = verifyResult?.checks ?? [];

  const handleVerifyClick = async () => {
    const res = validateRequiredText(verifyDir, 'unbrick.errors.verifyDirRequired');
    if (res.errorKey) {
      setVerifyDirError(t(res.errorKey));
      return;
    }
    setVerifyDirError(null);
    await onVerifyMedia(res.trimmed);
  };

  const handleInspectDiskClick = async () => {
    if (!onInspectSdDisk) return;
    const res = validateRequiredText(diskTarget, 'unbrick.sdKit.inspectDiskRequired');
    if (res.errorKey) {
      setDiskTargetError(t(res.errorKey));
      return;
    }
    setDiskTargetError(null);
    await onInspectSdDisk(res.trimmed);
  };

  return (
    <div className="border-t border-slate-800/80 pt-3 space-y-2.5">
      <div className="flex flex-col sm:flex-row gap-2">
        <div className="flex-1 space-y-1">
          <Input
            value={verifyDir}
            aria-label={t('unbrick.sdKit.verifyInputLabel')}
            aria-invalid={Boolean(verifyDirError)}
            aria-describedby={verifyDirError ? 'sd-verify-dir-err' : undefined}
            onChange={e => {
              onChangeVerifyDir(e.target.value);
              if (verifyDirError) setVerifyDirError(null);
            }}
            placeholder="sd_card_eh55"
          />
          {verifyDirError && (
            <p id="sd-verify-dir-err" className="text-[11px] text-rose-400 font-mono">
              {verifyDirError}
            </p>
          )}
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={verifyLoading}
          onClick={() => void handleVerifyClick()}
        >
          {verifyLoading ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
          ) : (
            <ShieldCheck className="w-3.5 h-3.5 mr-1.5 text-cyan-400" />
          )}
          <span>{t('unbrick.sdKit.verifyBtn')}</span>
        </Button>
      </div>

      {verifyError && (
        <Alert variant="destructive">
          <AlertDescription>{verifyError}</AlertDescription>
        </Alert>
      )}

      {checks.length === 0 ? (
        <EmptyState compact title={t('unbrick.sdKit.checksEmpty')} />
      ) : (
        <div className="space-y-1.5 max-h-52 overflow-y-auto pr-1">
          {checks.map(item => (
            <div
              key={item.id}
              className="rounded-xl border border-slate-800 bg-slate-950/70 p-2.5 text-xs space-y-1"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono font-semibold text-slate-200">{item.title}</span>
                <Badge variant={resolveSeverityBadgeVariant(item.severity)}>
                  {item.severity.toUpperCase()}
                </Badge>
              </div>
              <p className="text-[11px] text-slate-400 leading-relaxed">{item.detail}</p>
            </div>
          ))}
        </div>
      )}

      {onInspectSdDisk && (
        <div className="border-t border-slate-800/80 pt-3 space-y-2.5">
          <div className="flex flex-col sm:flex-row gap-2">
            <div className="flex-1 space-y-1">
              <Input
                value={diskTarget}
                aria-label={t('unbrick.sdKit.inspectDiskLabel')}
                aria-invalid={Boolean(diskTargetError)}
                aria-describedby={diskTargetError ? 'sd-disk-target-err' : undefined}
                onChange={e => {
                  setDiskTarget(e.target.value);
                  if (diskTargetError) setDiskTargetError(null);
                }}
                placeholder="E: ou firmware_and_images/sd.img"
              />
              {diskTargetError && (
                <p id="sd-disk-target-err" className="text-[11px] text-rose-400 font-mono">
                  {diskTargetError}
                </p>
              )}
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={diskLoading}
              onClick={() => void handleInspectDiskClick()}
            >
              {diskLoading ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
              ) : (
                <HardDrive className="w-3.5 h-3.5 mr-1.5 text-emerald-400" />
              )}
              <span>{t('unbrick.sdKit.inspectDiskBtn')}</span>
            </Button>
          </div>

          {diskError && (
            <Alert variant="destructive">
              <AlertDescription>{diskError}</AlertDescription>
            </Alert>
          )}

          {diskResult && (
            <div className="rounded-xl border border-slate-800 bg-slate-950/80 p-3 space-y-2 text-[11px] font-mono">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="font-semibold text-cyan-300">{diskResult.target}</span>
                <div className="flex flex-wrap gap-1.5">
                  <Badge variant="outline">{diskResult.partition_scheme}</Badge>
                  {diskResult.bpb && (
                    <Badge variant="secondary">
                      {diskResult.bpb.computed_fat_kind} · {Math.round(diskResult.bpb.cluster_size_bytes / 1024)} KiB/cluster
                    </Badge>
                  )}
                  <Badge variant={resolveSdOverallBadgeVariant(diskResult.overall_status)}>
                    {diskResult.overall_status}
                  </Badge>
                </div>
              </div>
              {diskResult.digadiag_entry && (
                <div className="text-slate-300">
                  {diskResult.digadiag_entry.name_8_3}: {diskResult.digadiag_entry.file_size.toLocaleString()} B
                  {' • '}
                  {diskResult.digadiag_entry.chain_contiguous ? 'contíguo' : 'fragmentado'}
                  {' • '}
                  {diskResult.digadiag_entry.chain_complete ? 'cadeia OK' : 'cadeia incompleta'}
                </div>
              )}
              {diskResult.pana_sd_entry && (
                <div className="text-slate-300">
                  {diskResult.pana_sd_entry.name_8_3}: {diskResult.pana_sd_entry.file_size.toLocaleString()} B
                  {' • '}
                  {diskResult.pana_sd_entry.chain_contiguous ? 'contíguo' : 'fragmentado'}
                </div>
              )}
              {diskResult.checks.slice(0, 4).map(chk => (
                <div key={chk.id} className="text-slate-400">
                  [{chk.severity.toUpperCase()}] {chk.title}: {chk.detail}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
