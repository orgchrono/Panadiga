import React from 'react';
import { Cpu } from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
import { SystemOverview } from '@/models';

interface DashboardArchitectureCardProps {
  overview: SystemOverview;
}

export const DashboardArchitectureCard: React.FC<DashboardArchitectureCardProps> = ({ overview }) => {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl bg-bench-900 border border-slate-800 p-5 shadow-xl font-mono flex flex-col justify-between">
      <div>
        <h3 className="text-xs font-mono font-semibold uppercase tracking-wider text-slate-400 mb-4 flex items-center gap-2">
          <Cpu className="w-4 h-4 text-cyan-400" />
          <span>
            {t('dashboard.arch.title', {
              model: overview.product_model || t('dashboard.arch.defaultModel'),
            })}
          </span>
        </h3>

        <div className="space-y-3 text-xs font-mono">
          <div className="flex items-center justify-between py-1.5 border-b border-slate-800/60">
            <span className="text-slate-500">{t('dashboard.arch.soc')}</span>
            <span className="text-slate-200 font-semibold">{overview.soc_processor}</span>
          </div>
          <div className="flex items-center justify-between py-1.5 border-b border-slate-800/60">
            <span className="text-slate-500">{t('dashboard.arch.core')}</span>
            <span className="text-slate-200">{overview.core_architecture}</span>
          </div>
          <div className="flex items-center justify-between py-1.5 border-b border-slate-800/60">
            <span className="text-slate-500">{t('dashboard.arch.flashRom')}</span>
            <span className="text-slate-200">{overview.flash_rom}</span>
          </div>
          <div className="flex items-center justify-between py-1.5 border-b border-slate-800/60">
            <span className="text-slate-500">{t('dashboard.arch.hdd')}</span>
            <span className="text-slate-200">{overview.hdd_specs}</span>
          </div>
          <div className="flex items-center justify-between py-1.5 border-b border-slate-800/60">
            <span className="text-slate-500">{t('dashboard.arch.optical')}</span>
            <span className="text-slate-200">{overview.optical_drive}</span>
          </div>
          <div className="flex items-center justify-between py-1.5 border-b border-slate-800/60">
            <span className="text-slate-500">{t('dashboard.arch.ocdPort')}</span>
            <span className="text-cyan-400 font-semibold">{overview.debug_port}</span>
          </div>
          <div className="flex items-center justify-between py-1.5">
            <span className="text-slate-500">{t('dashboard.arch.firmware')}</span>
            <span className="text-emerald-400 font-bold">{overview.firmware_version}</span>
          </div>
        </div>
      </div>
    </div>
  );
};
