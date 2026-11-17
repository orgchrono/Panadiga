import React from 'react';
import { Wrench } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import { HEX32_PAD_WIDTH } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import { NvramCleanReport, MicroSectorAudit } from '@/models';

interface NvramMicrosectorsTableProps {
  report: NvramCleanReport;
  cleanSuccess: boolean;
}

function renderSectorDiagnosis(
  sectorName: string,
  cleanSuccess: boolean,
  t: ReturnType<typeof useTranslation>['t'],
): React.ReactNode {
  if (cleanSuccess) {
    return <StatusBadge variant="success" label={t('nvram.table.badgeVirgin')} />;
  }
  if (sectorName === 'SA137') {
    return <StatusBadge variant="error" label={t('nvram.table.badgeU99Loop')} />;
  }
  return <StatusBadge variant="warning" label={t('nvram.table.badgeEemulActive')} />;
}

export const NvramMicrosectorsTable: React.FC<NvramMicrosectorsTableProps> = ({
  report,
  cleanSuccess,
}) => {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl font-mono">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-300 flex items-center gap-2">
            <Wrench className="w-4 h-4 text-amber-400" />
            <span>{t('nvram.table.title')}</span>
          </h3>
          <p className="text-[11px] text-slate-500 font-sans mt-0.5">
            {t('nvram.table.subtitle')}
          </p>
        </div>
        <span className="text-xs text-slate-400">{t('nvram.table.totalBytes')}</span>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-left text-xs font-mono">
          <thead>
            <tr className="border-b border-slate-800 text-slate-500 uppercase text-[10px]">
              <th className="py-2.5 px-3">{t('nvram.table.colSector')}</th>
              <th className="py-2.5 px-3">{t('nvram.table.colOffset')}</th>
              <th className="py-2.5 px-3">{t('nvram.table.colActive')}</th>
              <th className="py-2.5 px-3">{t('nvram.table.colHeader')}</th>
              <th className="py-2.5 px-3 text-right">{t('nvram.table.colDiagnosis')}</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/60">
            {report.microsectors.length === 0 ? (
              <tr>
                <td colSpan={5} className="py-8 text-center text-xs text-slate-500 font-mono">
                  {t('nvram.table.empty')}
                </td>
              </tr>
            ) : (
              report.microsectors.map((sec: MicroSectorAudit) => {
                const pct = ((sec.active_bytes / sec.total_bytes) * 100).toFixed(1);
                const barStyle: React.CSSProperties & Record<'--sector-pct', string> = {
                  '--sector-pct': `${pct}%`,
                };
                return (
                  <tr key={sec.sector_name} className="hover:bg-slate-800/40 transition-colors">
                    <td className="py-2.5 px-3 font-bold text-slate-200">
                      {sec.sector_name}
                    </td>
                    <td className="py-2.5 px-3 text-cyan-400">
                      0x{sec.offset.toString(16).padStart(HEX32_PAD_WIDTH, '0').toUpperCase()}
                    </td>
                    <td
                      className="py-2.5 px-3"
                      aria-label={t('nvram.table.activeProgressAria', {
                        sector: sec.sector_name,
                        active: sec.active_bytes,
                        total: sec.total_bytes,
                        pct,
                      })}
                    >
                      {sec.active_bytes} / {sec.total_bytes} ({pct}%)
                      <div className="mt-1 w-16 h-1.5 rounded-full bg-slate-800 overflow-hidden">
                        <div
                          className={`h-full w-[var(--sector-pct)] ${cleanSuccess ? 'bg-emerald-400' : 'bg-amber-400'}`}
                          style={barStyle}
                        />
                      </div>
                    </td>
                    <td className="py-2.5 px-3 text-slate-300 tracking-wider">
                      {sec.header_hex}
                    </td>
                    <td className="py-2.5 px-3 text-right">
                      {renderSectorDiagnosis(sec.sector_name, cleanSuccess, t)}
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
};
