import React, { useEffect, useState } from 'react';
import { HardDrive, Unlock } from 'lucide-react';
import {
  HddActiveTabContent,
  HddMetricsGrid,
  HddNavigationBar,
  HddTopHeader,
  StorageDeviceSelector,
} from '@/components/hdd';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { useTranslation } from '@/i18n/useTranslation';
import type { ViewKey } from '@/models';
import { useHddSafeFlashViewModel } from '@/viewmodels/pipelines';
import { useHddCapacityViewModel } from '@/viewmodels/useHddCapacityViewModel';
import { useHddStorageViewModel } from '@/viewmodels/useHddStorageViewModel';
import { useHddWriterViewModel } from '@/viewmodels/useHddWriterViewModel';
import { useStorageDrives } from '@/viewmodels/useStorageDrives';

/**
 * Declarative MVVM View: Panasonic MEIHDFS Storage & HDD Forensics Workbench.
 * Decomposed into dedicated domain components under components/hdd/.
 */
export const HddStorageView: React.FC<{ onNavigate?: (view: ViewKey) => void }> = ({
  onNavigate,
}) => {
  const { t } = useTranslation();
  const {
    hddReport,
    meihdfsReport,
    filteredRecords,
    activeTab,
    setActiveTab,
    loading,
    extracting,
    copiedSha,
    copyManifestSha,
    manifest,
    tagFilter,
    setTagFilter,
    searchQuery,
    setSearchQuery,
    imagePath,
    error,
    extractPartitions,
    refresh,
  } = useHddStorageViewModel();

  const capacityVm = useHddCapacityViewModel();
  const writerVm = useHddWriterViewModel();
  const safeFlashVm = useHddSafeFlashViewModel();
  const hddInspectorDrives = useStorageDrives('hard_disk', true);
  const { isAdvanced, setMode } = useWorkbenchMode();
  const [inputPath, setInputPath] = useState(imagePath || '');
  const [showPathInput, setShowPathInput] = useState(false);

  useEffect(() => {
    if (!isAdvanced && (activeTab === 'meihdfs' || activeTab === 'journal' || activeTab === 'density')) {
      setActiveTab('overview');
    }
  }, [activeTab, isAdvanced, setActiveTab]);

  const handleOpenPath = () => {
    if (inputPath.trim()) {
      refresh(inputPath.trim());
      setShowPathInput(false);
    }
  };

  const handleSelectImagePath = (resolved: string) => {
    setInputPath(resolved);
    refresh(resolved);
    setShowPathInput(false);
  };

  return (
    <div className="space-y-6 max-w-7xl mx-auto pb-12 font-mono" aria-label={t('hdd.title')}>
      <HddTopHeader
        report={hddReport}
        extracting={extracting}
        onExtractPartitions={extractPartitions}
        onSelectImage={() => setShowPathInput((prev) => !prev)}
      />

      <HddMetricsGrid report={hddReport} />

      <HddNavigationBar
        showPathInput={showPathInput}
        inputPath={inputPath}
        onInputPathChange={setInputPath}
        onOpenPath={handleOpenPath}
        onSelectImagePath={handleSelectImagePath}
        drives={hddInspectorDrives.filteredDrives}
        roleFilter={hddInspectorDrives.roleFilter}
        onChangeRoleFilter={hddInspectorDrives.setRoleFilter}
        loadingDrives={hddInspectorDrives.loading}
        onRefreshDrives={() => void hddInspectorDrives.refresh()}
        activeTab={activeTab}
        onSelectTab={setActiveTab}
        isAdvanced={isAdvanced}
        onEnableAdvanced={() => setMode('engineering')}
        hasManifest={Boolean(manifest)}
      />

      {!hddReport &&
        activeTab !== 'writer' &&
        activeTab !== 'capacity' &&
        activeTab !== 'safe-flash' && (
        <div className="p-8 rounded-2xl bg-bench-900 border border-slate-800 text-center space-y-4 max-w-2xl mx-auto shadow-xl">
          <div className="w-12 h-12 rounded-full bg-cyan-500/10 text-cyan-400 border border-cyan-500/30 flex items-center justify-center mx-auto">
            <HardDrive className="w-6 h-6" />
          </div>
          <div>
            <h3 className="text-sm font-bold text-slate-100">
              {error ? t('hdd.fallback.notFoundTitle') : t('hdd.fallback.emptyTitle')}
            </h3>
            <p className="text-xs text-slate-400 mt-1 max-w-md mx-auto">
              {error || t('hdd.fallback.defaultError')}
            </p>
          </div>

          <div className="flex items-center gap-2 max-w-md mx-auto pt-2">
            <input
              type="text"
              placeholder={t('hdd.fallback.placeholder')}
              value={inputPath}
              onChange={(e) => setInputPath(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleOpenPath()}
              className="flex-1 px-3 py-2 rounded-xl bg-slate-950 border border-slate-800 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 font-mono"
            />
            <button
              type="button"
              onClick={handleOpenPath}
              disabled={loading || !inputPath.trim()}
              className="px-4 py-2 rounded-xl bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-bold text-xs transition-colors disabled:opacity-50"
            >
              {loading ? t('hdd.fallback.loadingBtn') : t('hdd.fallback.analyzeBtn')}
            </button>
          </div>

          <div className="pt-3 border-t border-slate-800 text-left space-y-2">
            <span className="text-[11px] text-slate-400 block">
              {t('hdd.devices.inspectDeviceLabel')}
            </span>
            <StorageDeviceSelector
              compact
              drives={hddInspectorDrives.filteredDrives}
              selectedDevicePath={inputPath}
              roleFilter={hddInspectorDrives.roleFilter}
              onChangeRoleFilter={hddInspectorDrives.setRoleFilter}
              loading={hddInspectorDrives.loading}
              onRefresh={() => void hddInspectorDrives.refresh()}
              onSelectDrive={(drive) => handleSelectImagePath(drive.device_path)}
            />
          </div>

          <div className="pt-4 border-t border-slate-800 flex flex-wrap items-center justify-center gap-3 text-xs">
            <button
              type="button"
              onClick={() => setActiveTab('writer')}
              className="px-4 py-2 rounded-xl bg-purple-500/10 hover:bg-purple-500/20 text-purple-300 border border-purple-500/30 font-bold flex items-center gap-2 transition-colors"
            >
              <HardDrive className="w-4 h-4" />
              <span>{t('hdd.fallback.viewDisksBtn')}</span>
            </button>
            <button
              type="button"
              onClick={() => setActiveTab('capacity')}
              className="px-4 py-2 rounded-xl bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 font-bold flex items-center gap-2 transition-colors"
            >
              <Unlock className="w-4 h-4" />
              <span>{t('hdd.fallback.openCapacityBtn')}</span>
            </button>
            {onNavigate && (
              <button
                type="button"
                onClick={() => onNavigate('settings')}
                className="px-4 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 font-bold transition-colors"
              >
                {t('hdd.fallback.settingsBtn')}
              </button>
            )}
          </div>
        </div>
      )}

      <HddActiveTabContent
        activeTab={activeTab}
        hddReport={hddReport}
        meihdfsReport={meihdfsReport}
        filteredRecords={filteredRecords}
        searchQuery={searchQuery}
        setSearchQuery={setSearchQuery}
        tagFilter={tagFilter}
        setTagFilter={setTagFilter}
        manifest={manifest}
        copiedSha={copiedSha}
        onCopySha={copyManifestSha}
        capacityProps={capacityVm}
        writerProps={writerVm}
        safeFlashVm={safeFlashVm}
      />
    </div>
  );
};
