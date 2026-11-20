import React from 'react';
import { ArrowRight } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import { Button, EmptyState } from '@/components/ui';
import { KIB_BYTES } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { AtaIdentifyInfo } from '@/models';

const MIB_BYTES = KIB_BYTES * KIB_BYTES;

export interface DavisIdentifySectionProps {
  readonly info: AtaIdentifyInfo | null;
  readonly eh55Compatible: boolean;
  readonly stepLogs: readonly string[];
  readonly onOpenHddPipeline: () => void;
}

export const DavisIdentifySection: React.FC<DavisIdentifySectionProps> = ({
  info,
  eh55Compatible,
  stepLogs,
  onOpenHddPipeline,
}) => {
  const { t } = useTranslation();

  return (
    <>
      <div className="rounded-xl bg-slate-950 border border-slate-800 p-4 space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="font-bold text-slate-100">{t('pipelines.davisProbe.identify.title')}</span>
          {info && (
            <div className="flex items-center gap-2">
              <StatusBadge
                variant={eh55Compatible ? 'success' : 'warning'}
                label={
                  eh55Compatible
                    ? t('pipelines.davisProbe.identify.verdictCompatible')
                    : t('pipelines.davisProbe.identify.verdictIncompatible')
                }
              />
              <Button
                type="button"
                size="sm"
                variant="outline"
                data-testid="davis-open-hdd-btn"
                onClick={onOpenHddPipeline}
                className="gap-1 text-[11px]"
              >
                <span>{t('pipelines.davisProbe.identify.openHddPipelineBtn')}</span>
                <ArrowRight className="w-3 h-3" />
              </Button>
            </div>
          )}
        </div>

        {!info ? (
          <EmptyState compact title={t('pipelines.davisProbe.identify.empty')} />
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-2 text-[11px]">
            <div className="p-2 rounded bg-bench-900 border border-slate-800">
              <span className="text-slate-400 block">{t('pipelines.davisProbe.identify.model')}</span>
              <strong className="text-slate-100">{info.model}</strong>
            </div>
            <div className="p-2 rounded bg-bench-900 border border-slate-800">
              <span className="text-slate-400 block">{t('pipelines.davisProbe.identify.serial')}</span>
              <strong className="text-slate-200">{info.serial_number}</strong>
            </div>
            <div className="p-2 rounded bg-bench-900 border border-slate-800">
              <span className="text-slate-400 block">{t('pipelines.davisProbe.identify.firmware')}</span>
              <strong className="text-slate-200">{info.firmware_revision}</strong>
            </div>
            <div className="p-2 rounded bg-bench-900 border border-slate-800">
              <span className="text-slate-400 block">{t('pipelines.davisProbe.identify.capacity')}</span>
              <strong className="text-cyan-300">
                {Math.round(info.capacity_bytes / MIB_BYTES)} MiB ({info.effective_sectors} LBA)
              </strong>
            </div>
            <div className="p-2 rounded bg-bench-900 border border-slate-800">
              <span className="text-slate-400 block">{t('pipelines.davisProbe.identify.mwdma2')}</span>
              <strong className={info.mwdma2_supported ? 'text-emerald-300' : 'text-rose-400'}>
                {info.mwdma2_supported
                  ? t('pipelines.davisProbe.identify.mwdma2Ok')
                  : t('pipelines.davisProbe.identify.mwdma2Missing')}
              </strong>
            </div>
            <div className="p-2 rounded bg-bench-900 border border-slate-800">
              <span className="text-slate-400 block">{t('pipelines.davisProbe.identify.mirrorMainLba')}</span>
              <strong className="text-amber-300">{info.mirror_main_lba ?? '-'}</strong>
            </div>
          </div>
        )}
      </div>

      <div className="rounded-xl bg-slate-950 border border-slate-800 p-3 space-y-1.5">
        <div className="font-bold text-slate-300">{t('pipelines.davisProbe.logsTitle')}</div>
        {stepLogs.length === 0 ? (
          <EmptyState compact title={t('pipelines.davisProbe.logsEmpty')} />
        ) : (
          <ul className="space-y-0.5 text-[11px] text-slate-300 max-h-36 overflow-y-auto">
            {stepLogs.map(line => (
              <li key={line}>{line}</li>
            ))}
          </ul>
        )}
      </div>
    </>
  );
};
