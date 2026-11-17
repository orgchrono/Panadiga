import React from 'react';
import { Settings } from 'lucide-react';
import { RuntimeDefaultsCard } from '@/components/settings/RuntimeDefaultsCard';
import { ThemeAccessibilityCard } from '@/components/settings/ThemeAccessibilityCard';
import { WorkflowRecoveryCards } from '@/components/settings/WorkflowRecoveryCards';
import { useToast } from '@/context/ToastContext';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { useTranslation } from '@/i18n/useTranslation';
import { useRuntimeSettingsViewModel } from '@/viewmodels/useRuntimeSettingsViewModel';

export const SettingsView: React.FC = () => {
  const { t } = useTranslation();
  const { showToast: toast } = useToast();
  const {
    accessibility,
    setTheme,
    setColorblindSafe,
    setReducedMotion,
    setFontScale,
    mode,
    setMode,
    hudOpen,
    setHudOpen,
    pipeline,
    resetPipeline,
    resetWorkbenchPreferences,
  } = useWorkbenchMode();
  const runtimeVm = useRuntimeSettingsViewModel();

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

      <RuntimeDefaultsCard
        draft={runtimeVm.draft}
        keywordsText={runtimeVm.keywordsText}
        onChangeKeywordsText={runtimeVm.setKeywordsText}
        isLoading={runtimeVm.isLoading}
        saving={runtimeVm.saving}
        error={runtimeVm.error}
        successMessage={runtimeVm.successMessage}
        isDirty={runtimeVm.isDirty}
        onUpdateSerial={runtimeVm.updateSerialField}
        onUpdateStorage={runtimeVm.updateStorageField}
        onUpdateIr={runtimeVm.updateIrField}
        onUpdateWorkbench={runtimeVm.updateWorkbenchField}
        onPersistDefaults={() => void runtimeVm.saveRuntimeConfig()}
        onResetDefaults={() => void runtimeVm.resetRuntimeDefaults()}
      />

      <ThemeAccessibilityCard
        accessibility={accessibility}
        onSetTheme={setTheme}
        onSetColorblindSafe={setColorblindSafe}
        onSetReducedMotion={setReducedMotion}
        onSetFontScale={setFontScale}
        onNotify={toast}
      />
      <WorkflowRecoveryCards
        mode={mode}
        onSetMode={setMode}
        hudOpen={hudOpen}
        onSetHudOpen={setHudOpen}
        pipeline={pipeline}
        onResetPipeline={resetPipeline}
        onResetWorkbenchPreferences={resetWorkbenchPreferences}
        onNotify={toast}
        installingShortcut={runtimeVm.installingShortcut}
        onInstallShortcut={() => void runtimeVm.installSystemLauncher()}
      />
    </div>
  );
};
