import React, { useEffect, useState } from 'react';
import { useHddStorageViewModel } from '@/viewmodels/useHddStorageViewModel';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { useToast } from '@/context/ToastContext';
import { useManagedTimeouts } from '@/hooks/useManagedTimeouts';
import { CLIPBOARD_FEEDBACK_MS } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import type { ViewKey } from '@/models';
import {
  HddTopHeader,
  HddMetricsGrid,
  HddNavigationBar,
  HddPartitionTable,
  HddMbrComparison,
  HddMeihdfsTable,
  HddJournalTable,
  HddDensityMap,
  HddExtractManifestCard,
  HddWriterCard,
  HddCapacityPatcherCard,
} from '@/components/hdd';
import { HardDrive, Unlock } from 'lucide-react';

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

  const { showToast } = useToast();
  const { isAdvanced, setMode } = useWorkbenchMode();
  const [copiedSha, setCopiedSha] = useState(false);
  const [inputPath, setInputPath] = useState(imagePath || '');
  const [showPathInput, setShowPathInput] = useState(false);
  const { scheduleTimeout, clearAllTimeouts } = useManagedTimeouts();

  useEffect(() => {
    if (!isAdvanced && (activeTab === 'meihdfs' || activeTab === 'journal' || activeTab === 'density')) {
      setActiveTab('overview');
    }
  }, [activeTab, isAdvanced, setActiveTab]);

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    clearAllTimeouts();
    setCopiedSha(true);
    showToast({
      type: 'info',
      title: t('hdd.shaCopiedToastTitle'),
      message: t('hdd.shaCopiedToastMsg'),
    });
    scheduleTimeout(() => setCopiedSha(false), CLIPBOARD_FEEDBACK_MS);
  };

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
        onSelectImage={() => setShowPathInput(prev => !prev)}
      />

      <HddMetricsGrid report={hddReport} />

      <HddNavigationBar
        showPathInput={showPathInput}
        inputPath={inputPath}
        onInputPathChange={setInputPath}
        onOpenPath={handleOpenPath}
        onSelectImagePath={handleSelectImagePath}
        activeTab={activeTab}
        onSelectTab={setActiveTab}
        isAdvanced={isAdvanced}
        onEnableAdvanced={() => setMode('advanced')}
        hasManifest={Boolean(manifest)}
      />

      {!hddReport && activeTab !== 'writer' && activeTab !== 'capacity' && (
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
              onChange={e => setInputPath(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleOpenPath()}
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

      {activeTab === 'overview' && hddReport && <HddPartitionTable hddReport={hddReport} />}
      {activeTab === 'mbr' && hddReport && <HddMbrComparison hddReport={hddReport} />}
      {activeTab === 'meihdfs' && (
        <HddMeihdfsTable
          filteredRecords={filteredRecords}
          searchQuery={searchQuery}
          tagFilter={tagFilter}
          onSearchChange={setSearchQuery}
          onTagFilterChange={setTagFilter}
        />
      )}
      {activeTab === 'journal' && meihdfsReport && <HddJournalTable meihdfsReport={meihdfsReport} />}
      {activeTab === 'density' && hddReport && <HddDensityMap hddReport={hddReport} />}
      {activeTab === 'extract' && manifest && (
        <HddExtractManifestCard
          manifest={manifest}
          copiedSha={copiedSha}
          onCopySha={copyToClipboard}
        />
      )}
      {activeTab === 'capacity' && <HddCapacityPatcherCard />}
      {activeTab === 'writer' && <HddWriterCard />}
    </div>
  );
};
