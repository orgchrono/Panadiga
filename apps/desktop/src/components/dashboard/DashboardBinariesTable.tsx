import React from 'react';
import { FolderCheck, CheckCircle2, AlertTriangle } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import { useTranslation } from '@/i18n/useTranslation';
import { BinaryFileStatus } from '@/models';

interface DashboardBinariesTableProps {
  binaries: BinaryFileStatus[];
}

export const DashboardBinariesTable: React.FC<DashboardBinariesTableProps> = ({ binaries }) => {
  const { t } = useTranslation();
  const activeCount = binaries.filter((b) => b.exists).length;

  return (
    <div className="lg:col-span-2 rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl font-mono">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-xs font-mono font-semibold uppercase tracking-wider text-slate-400 flex items-center gap-2">
          <FolderCheck className="w-4 h-4 text-emerald-400" />
          <span>{t('dashboard.binaries.title')}</span>
        </h3>
        <span className="text-[11px] font-mono text-slate-500">
          {t('dashboard.binaries.activeSummary', { active: activeCount, total: binaries.length })}
        </span>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-left text-xs font-mono">
          <thead>
            <tr className="border-b border-slate-800 text-slate-500 uppercase text-[10px]">
              <th className="py-2 px-3">{t('dashboard.binaries.colBinary')}</th>
              <th className="py-2 px-3">{t('dashboard.binaries.colSize')}</th>
              <th className="py-2 px-3">{t('dashboard.binaries.colRole')}</th>
              <th className="py-2 px-3 text-right">{t('dashboard.binaries.colIntegrity')}</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/60">
            {binaries.length === 0 ? (
              <tr>
                <td colSpan={4} className="py-8 text-center text-xs text-slate-500 font-mono">
                  {t('dashboard.binaries.empty')}
                </td>
              </tr>
            ) : (
              binaries.map((bin) => (
                <tr key={bin.id} className="hover:bg-slate-800/40 transition-colors">
                  <td className="py-2.5 px-3">
                    <div className="font-semibold text-slate-200 flex items-center gap-2">
                      {bin.exists ? (
                        <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                      ) : (
                        <AlertTriangle className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                      )}
                      <span className="truncate max-w-[220px]" title={bin.name}>
                        {bin.name}
                      </span>
                    </div>
                    <span className="text-[10px] text-slate-500 truncate block">
                      {bin.path}
                    </span>
                  </td>
                  <td className="py-2.5 px-3 text-slate-300">
                    {bin.formatted_size ?? '-'}
                  </td>
                  <td className="py-2.5 px-3 text-slate-400 max-w-[240px] truncate text-[11px]">
                    {bin.expected_role}
                  </td>
                  <td className="py-2.5 px-3 text-right">
                    {bin.status === 'ready' && (
                      <StatusBadge variant="success" label={t('dashboard.binaries.statusReady')} />
                    )}
                    {bin.status === 'warning' && (
                      <StatusBadge variant="warning" label={t('dashboard.binaries.statusWarning')} />
                    )}
                    {bin.status === 'missing' && (
                      <StatusBadge variant="error" label={t('dashboard.binaries.statusMissing')} />
                    )}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
};
