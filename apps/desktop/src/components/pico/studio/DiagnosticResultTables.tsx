import React from 'react';
import { EmptyState } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { PicoDiagnosticResult } from '@/models';

export interface DiagnosticResultTablesProps {
  result: PicoDiagnosticResult;
}

export const DiagnosticResultTables: React.FC<DiagnosticResultTablesProps> = ({ result }) => {
  const { t } = useTranslation();
  const hasAnyTable =
    result.pincheck_rows.length > 0 ||
    result.transition_rows.length > 0 ||
    result.dump_rows.length > 0;

  if (!hasAnyTable) {
    return <EmptyState compact title={t('pico.studioTables.empty')} />;
  }

  return (
    <>
      {result.pincheck_rows.length > 0 && (
        <div className="overflow-x-auto rounded-lg border border-slate-800">
          <table className="w-full text-[11px]">
            <thead className="bg-slate-900 text-slate-400">
              <tr>
                <th className="text-left p-2">{t('pico.studioTables.colPin')}</th>
                <th className="text-left p-2">{t('pico.studioTables.colHiz')}</th>
                <th className="text-left p-2">{t('pico.studioTables.colPullDown')}</th>
                <th className="text-left p-2">{t('pico.studioTables.colPullUp')}</th>
                <th className="text-left p-2">{t('pico.studioTables.colElectricalState')}</th>
              </tr>
            </thead>
            <tbody>
              {result.pincheck_rows.map((row) => (
                <tr key={row.pin_label} className="border-t border-slate-800/80 text-slate-200">
                  <td className="p-2 font-bold text-cyan-300">{row.pin_label}</td>
                  <td className="p-2">{row.hiz}</td>
                  <td className="p-2">{row.pull_down}</td>
                  <td className="p-2">{row.pull_up}</td>
                  <td className="p-2 font-sans text-slate-300">{row.state}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {result.transition_rows.length > 0 && (
        <div className="overflow-x-auto rounded-lg border border-slate-800">
          <table className="w-full text-[11px]">
            <thead className="bg-slate-900 text-slate-400">
              <tr>
                <th className="text-left p-2">{t('pico.studioTables.colSignal')}</th>
                <th className="text-left p-2">{t('pico.studioTables.colGpio')}</th>
                <th className="text-left p-2">{t('pico.studioTables.colFinalLevel')}</th>
                <th className="text-left p-2">{t('pico.studioTables.colTransitions')}</th>
              </tr>
            </thead>
            <tbody>
              {result.transition_rows.map((row) => (
                <tr key={`${row.signal}-${row.gpio}`} className="border-t border-slate-800/80">
                  <td className="p-2 font-bold text-cyan-300">{row.signal}</td>
                  <td className="p-2 text-slate-400">{row.gpio}</td>
                  <td className="p-2 text-slate-200">{row.final_level ?? '-'}</td>
                  <td className="p-2 text-emerald-400 font-bold">{row.transitions}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {result.dump_rows.length > 0 && (
        <div className="overflow-x-auto rounded-lg border border-slate-800">
          <table className="w-full text-[11px]">
            <thead className="bg-slate-900 text-slate-400">
              <tr>
                <th className="text-left p-2">{t('pico.studioTables.colAddress')}</th>
                <th className="text-left p-2">{t('pico.studioTables.colWords32')}</th>
                <th className="text-left p-2">{t('pico.studioTables.colAsciiLe')}</th>
              </tr>
            </thead>
            <tbody>
              {result.dump_rows.map((row) => (
                <tr key={row.address} className="border-t border-slate-800/80">
                  <td className="p-2 font-bold text-cyan-400">{row.address}</td>
                  <td className="p-2 text-slate-200">{row.words.join(' ')}</td>
                  <td className="p-2 text-amber-300">{row.ascii}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  );
};
