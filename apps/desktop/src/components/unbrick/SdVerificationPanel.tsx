import React, { useState } from 'react';
import { Loader2, ShieldCheck } from 'lucide-react';
import { Alert, AlertDescription, Badge, Button, EmptyState, Input } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { SdDiskInspectSection } from './SdDiskInspectSection';
import {
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
  autoSdMountDir = null,
  ...diskProps
}) => {
  const { t } = useTranslation();
  const [verifyDirError, setVerifyDirError] = useState<string | null>(null);
  const effectiveVerifyDir =
    verifyDir === 'sd_card_eh55' && autoSdMountDir ? autoSdMountDir : verifyDir;
  const checks = verifyResult?.checks ?? [];

  const handleVerifyClick = async () => {
    const res = validateRequiredText(effectiveVerifyDir, 'unbrick.errors.verifyDirRequired');
    if (res.errorKey) {
      setVerifyDirError(t(res.errorKey));
      return;
    }
    setVerifyDirError(null);
    await onVerifyMedia(res.trimmed);
  };

  return (
    <div className="border-t border-slate-800/80 pt-3 space-y-2.5">
      <div className="flex flex-col sm:flex-row gap-2">
        <div className="flex-1 space-y-1">
          <Input
            value={effectiveVerifyDir}
            aria-label={t('unbrick.sdKit.verifyInputLabel')}
            aria-invalid={Boolean(verifyDirError)}
            aria-describedby={verifyDirError ? 'sd-verify-dir-err' : undefined}
            onChange={(e) => {
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
          {checks.map((item) => (
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

      <SdDiskInspectSection {...diskProps} onChangeVerifyDir={onChangeVerifyDir} />
    </div>
  );
};
