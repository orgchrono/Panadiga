import React from 'react';
import { Layers } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import { HDD_ENTROPY_PRECISION } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import { HddAnalysisReport } from '@/models';

export interface HddPartitionTableProps {
  hddReport: HddAnalysisReport;
}

export const HddPartitionTable: React.FC<HddPartitionTableProps> = ({ hddReport }) => {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {hddReport.partitions.length === 0 ? (
          <div className="col-span-full p-8 text-center text-xs text-slate-500 font-mono rounded-xl bg-bench-900 border border-slate-800">
            {t('hdd.partitions.empty')}
          </div>
        ) : (
          hddReport.partitions.map((part) => (
            <div
              key={part.partition_id}
              className="rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl flex flex-col justify-between"
            >
              <div>
                <div className="flex items-center justify-between mb-3">
                  <span className="text-xs font-bold text-cyan-400">
                    {t('hdd.partitions.partHeader', { id: part.partition_id })}
                  </span>
                  <StatusBadge variant="success" label={t('hdd.partitions.badge')} />
                </div>
                <h3 className="text-sm font-bold text-slate-100">{part.name}</h3>
                <p className="text-xs text-slate-400 mt-1">
                  {t('hdd.partitions.magicStart', {
                    magic: part.signature_hex,
                    lba: part.lba_start,
                    offset: part.offset_hex,
                  })}
                </p>

                <div className="mt-4 p-3 rounded-lg bg-slate-950 border border-slate-800/80 space-y-2 text-xs">
                  <div className="flex justify-between text-slate-400">
                    <span>{t('hdd.partitions.payloadSize')}</span>
                    <span className="text-slate-200 font-bold">
                      {part.exact_payload_size.toLocaleString()} {t('hdd.metrics.bytesUnit')}
                    </span>
                  </div>
                  <div className="flex justify-between text-slate-400">
                    <span>{t('hdd.partitions.sectorAlign')}</span>
                    <span className="text-slate-200">
                      {t('hdd.partitions.sectorAlignVal', {
                        sectors: part.sectors_count,
                        bytes: part.sector_aligned_size.toLocaleString(),
                      })}
                    </span>
                  </div>
                  <div className="flex justify-between text-slate-400">
                    <span>{t('hdd.partitions.shannonEntropy')}</span>
                    <span className="text-amber-400 font-bold">
                      {t('hdd.partitions.entropyVal', {
                        entropy: part.entropy.toFixed(HDD_ENTROPY_PRECISION),
                      })}
                    </span>
                  </div>
                  <div className="flex justify-between text-slate-400">
                    <span>{t('hdd.partitions.shaLabel')}</span>
                    <span className="text-emerald-400 truncate max-w-xs" title={part.exact_sha256}>
                      {part.exact_sha256}
                    </span>
                  </div>
                </div>
              </div>

              <div className="mt-4 pt-3 border-t border-slate-800 text-[11px] text-slate-500 truncate">
                {t('hdd.partitions.headerFooter', { hex: part.header_hex_64 })}
              </div>
            </div>
          ))
        )}
      </div>

      <div className="rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-400 mb-3 flex items-center gap-2">
          <Layers className="w-4 h-4 text-cyan-400" />
          <span>{t('hdd.partitions.summaryTitle')}</span>
        </h3>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 text-xs font-mono">
          <div className="p-3 rounded-lg bg-slate-950 border border-slate-800">
            <span className="text-slate-500 text-[10px] block">{t('hdd.partitions.descTableLabel')}</span>
            <span className="text-slate-200 font-bold">{t('hdd.partitions.descTableVal')}</span>
            <span className="text-slate-400 text-[11px] block mt-1">{t('hdd.partitions.descTableSub')}</span>
          </div>
          <div className="p-3 rounded-lg bg-slate-950 border border-slate-800">
            <span className="text-slate-500 text-[10px] block">{t('hdd.partitions.journalLabel')}</span>
            <span className="text-slate-200 font-bold">{t('hdd.partitions.journalVal')}</span>
            <span className="text-slate-400 text-[11px] block mt-1">{t('hdd.partitions.journalSub')}</span>
          </div>
          <div className="p-3 rounded-lg bg-slate-950 border border-slate-800">
            <span className="text-slate-500 text-[10px] block">{t('hdd.partitions.transitionLabel')}</span>
            <span className="text-slate-200 font-bold">{t('hdd.partitions.transitionVal')}</span>
            <span className="text-slate-400 text-[11px] block mt-1">{t('hdd.partitions.transitionSub')}</span>
          </div>
        </div>
      </div>
    </div>
  );
};
