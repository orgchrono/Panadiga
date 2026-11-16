import React from 'react';
import { Settings } from 'lucide-react';
import { useTranslation } from '@/i18n/useTranslation';
import { ThemeAccessibilityCard } from '@/components/settings/ThemeAccessibilityCard';
import { WorkflowRecoveryCards } from '@/components/settings/WorkflowRecoveryCards';

export const SettingsView: React.FC = () => {
  const { t } = useTranslation();

  return (
    <div className="space-y-6 max-w-5xl mx-auto pb-12 font-mono">
      <div className="rounded-2xl bg-bench-900 border border-slate-800 p-5 shadow-xl flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div className="flex items-center gap-3.5">
          <div className="p-3 rounded-xl bg-cyan-500/10 text-cyan-400 border border-cyan-500/30">
            <Settings className="w-6 h-6" />
          </div>
          <div>
            <h2 className="text-base font-bold text-slate-100">{t('settings.title')}</h2>
            <p className="text-xs text-slate-400 font-sans mt-0.5">{t('settings.subtitle')}</p>
          </div>
        </div>
      </div>

      <ThemeAccessibilityCard />
      <WorkflowRecoveryCards />
    </div>
  );
};
