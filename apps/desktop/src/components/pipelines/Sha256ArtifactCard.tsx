import React from 'react';
import { CheckCircle2, Copy, ShieldAlert } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';

export interface Sha256ArtifactCardProps {
  readonly title?: string;
  readonly sourcePath?: string | null;
  readonly outputPath: string;
  readonly inputSha256?: string | null;
  readonly outputSha256: string;
  readonly expectedSha256?: string | null;
  readonly bytesProcessed?: number | null;
  readonly verified?: boolean;
  readonly copied?: boolean;
  readonly onCopyHash?: (digest: string) => void;
}

export const Sha256ArtifactCard: React.FC<Sha256ArtifactCardProps> = ({
  title,
  sourcePath,
  outputPath,
  inputSha256,
  outputSha256,
  expectedSha256,
  bytesProcessed,
  verified,
  copied = false,
  onCopyHash,
}) => {
  const { t } = useTranslation();
  const matchesExpected =
    verified !== undefined
      ? verified
      : expectedSha256
        ? expectedSha256.toLowerCase() === outputSha256.toLowerCase()
        : Boolean(outputSha256);

  return (
    <div className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-3 text-xs font-mono">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 pb-2">
        <div className="flex items-center gap-2 font-bold text-slate-100">
          {matchesExpected ? (
            <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
          ) : (
            <ShieldAlert className="w-4 h-4 text-rose-400 shrink-0" />
          )}
          <span>{title ?? t('pipelines.primitives.sha256Card.title')}</span>
        </div>
        <div className="flex items-center gap-2">
          <StatusBadge
            variant={matchesExpected ? 'success' : 'error'}
            label={
              matchesExpected
                ? t('pipelines.primitives.sha256Card.verifiedBadge')
                : t('pipelines.primitives.sha256Card.mismatchBadge')
            }
          />
          {onCopyHash && outputSha256 && (
            <Button
              type="button"
              size="sm"
              variant="outline"
              aria-label={t('pipelines.primitives.sha256Card.copyAria')}
              onClick={() => onCopyHash(outputSha256)}
              className="h-7 px-2 text-[11px] gap-1"
            >
              <Copy className="w-3 h-3" />
              <span>
                {copied
                  ? t('pipelines.primitives.sha256Card.copiedBtn')
                  : t('pipelines.primitives.sha256Card.copyBtn')}
              </span>
            </Button>
          )}
        </div>
      </div>

      <div className="space-y-1.5 text-[11px]">
        {sourcePath && (
          <div className="flex flex-col sm:flex-row sm:justify-between gap-1">
            <span className="text-slate-400">{t('pipelines.primitives.sha256Card.sourceLabel')}</span>
            <span className="text-slate-200 break-all">{sourcePath}</span>
          </div>
        )}
        <div className="flex flex-col sm:flex-row sm:justify-between gap-1">
          <span className="text-slate-400">{t('pipelines.primitives.sha256Card.outputLabel')}</span>
          <span className="text-cyan-300 break-all">{outputPath}</span>
        </div>
        {inputSha256 && (
          <div className="flex flex-col sm:flex-row sm:justify-between gap-1">
            <span className="text-slate-400">{t('pipelines.primitives.sha256Card.inputHashLabel')}</span>
            <code className="text-slate-300 break-all">{inputSha256}</code>
          </div>
        )}
        <div className="flex flex-col sm:flex-row sm:justify-between gap-1">
          <span className="text-slate-400">{t('pipelines.primitives.sha256Card.outputHashLabel')}</span>
          <code className="text-emerald-300 break-all">{outputSha256}</code>
        </div>
        {bytesProcessed !== null && bytesProcessed !== undefined && (
          <div className="flex flex-col sm:flex-row sm:justify-between gap-1">
            <span className="text-slate-400">{t('pipelines.primitives.sha256Card.bytesLabel')}</span>
            <span className="text-slate-200">{bytesProcessed.toLocaleString()} B</span>
          </div>
        )}
      </div>
    </div>
  );
};
