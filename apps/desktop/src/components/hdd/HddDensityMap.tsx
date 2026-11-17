import React from 'react';
import { BarChart3 } from 'lucide-react';
import { DENSITY_HIGH_THRESHOLD_PCT, DENSITY_MEDIUM_THRESHOLD_PCT } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import { HddAnalysisReport } from '@/models';

export interface HddDensityMapProps {
  hddReport: HddAnalysisReport;
}

function getDensityTileClass(percentage: number): string {
  if (percentage > DENSITY_HIGH_THRESHOLD_PCT) return 'bg-cyan-500 text-slate-950 font-bold';
  if (percentage > DENSITY_MEDIUM_THRESHOLD_PCT) return 'bg-cyan-700 text-slate-100';
  if (percentage > 0) return 'bg-cyan-950 text-cyan-400 border border-cyan-800';
  return 'bg-slate-900 text-slate-600';
}

export const HddDensityMap: React.FC<HddDensityMapProps> = ({ hddReport }) => {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
            <BarChart3 className="w-4 h-4 text-cyan-400" />
            <span>{t('hdd.densitySection.title')}</span>
          </h3>
          <p className="text-xs text-slate-400 mt-0.5">
            {t('hdd.densitySection.subtitle')}
          </p>
        </div>
      </div>

      <section
        className="grid grid-cols-10 sm:grid-cols-20 gap-1.5 p-3 rounded-xl bg-slate-950 border border-slate-800"
        aria-label={t('hdd.densitySection.ariaLabel')}
      >
        {hddReport.density_map.length === 0 ? (
          <div className="col-span-full py-8 text-center text-xs text-slate-500 font-mono">
            {t('hdd.densitySection.empty')}
          </div>
        ) : (
          hddReport.density_map.map((chunk) => {
            const tileBg = getDensityTileClass(chunk.percentage);
            const hexOffset = chunk.offset.toString(16).toUpperCase();
            return (
              <div
                key={chunk.mb_index}
                title={t('hdd.densitySection.tileTitle', {
                  index: chunk.mb_index,
                  hex: hexOffset,
                  pct: chunk.percentage,
                })}
                className={`h-9 rounded flex flex-col items-center justify-center text-[9px] cursor-pointer hover:ring-2 hover:ring-cyan-300 transition-all ${tileBg}`}
              >
                <span>{t('hdd.densitySection.blockLabel', { index: chunk.mb_index })}</span>
                <span className="text-[8px] opacity-80">
                  {t('hdd.densitySection.pctLabel', { pct: chunk.percentage > 0 ? chunk.percentage : 0 })}
                </span>
              </div>
            );
          })
        )}
      </section>
    </div>
  );
};
