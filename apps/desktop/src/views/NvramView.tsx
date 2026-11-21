import React, { useState } from 'react';
import { useNvramViewModel } from '@/viewmodels/useNvramViewModel';
import {
  NvramTopBanner,
  NvramActionsCard,
  NvramPatchCard,
  NvramMetricsGrid,
  NvramSectorMap,
  NvramMicrosectorsTable,
  NvramAuditDialog,
} from '@/components/nvram';
import { useTranslation } from '@/i18n/useTranslation';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import type { ViewKey } from '@/models';
import { AlertCircle, Radio, RefreshCw, Wrench } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface NvramViewProps {
  onNavigate?: (view: ViewKey) => void;
}

/**
 * Declarative MVVM View: Spansion NOR Flash S29JL064H & NVRAM U99 Sanitizer.
 * Fully decomposed following Single Responsibility, Functional Programming, and Radix UI.
 */
export const NvramView: React.FC<NvramViewProps> = ({ onNavigate }) => {
  const { t } = useTranslation();
  const {
    report,
    filteredSectors,
    selectedSector,
    setSelectedSector,
    loading,
    capturingLive,
    liveCapture,
    cleaning,
    cleanSuccess,
    patching,
    patchSuccess,
    patchError,
    filterType,
    setFilterType,
    error,
    executeSurgicalClean,
    executeGuardedPatch,
    captureFromLivePico,
    clearLiveCapture,
    refresh,
  } = useNvramViewModel();

  const [showBackupModal, setShowBackupModal] = useState<boolean>(false);
  const { isAdvanced, setMode } = useWorkbenchMode();

  return (
    <div className="space-y-6 max-w-7xl mx-auto pb-12 font-mono" aria-label={t('nvram.title')}>
      {loading && (
        <div className="flex items-center gap-2 text-xs text-cyan-400">
          <RefreshCw className="w-3.5 h-3.5 animate-spin" />
          <span>{t('nvram.syncingMap')}</span>
        </div>
      )}
      {/* Surgical Clean Notification */}
      {cleanSuccess && (
        <NvramTopBanner onOpenAudit={() => setShowBackupModal(true)} />
      )}

      {/* Error Banner */}
      {error && (
        <div className="p-4 rounded-xl bg-rose-500/10 border border-rose-500/40 text-rose-300 text-xs flex items-center justify-between shadow-lg">
          <div className="flex items-center gap-2">
            <AlertCircle className="w-4 h-4 text-rose-400 shrink-0" />
            <span>{t('nvram.loadError', { error })}</span>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {onNavigate && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => onNavigate('picobench')}
                className="border-rose-500/40 text-rose-200 hover:bg-rose-900/40 gap-1.5"
              >
                <Radio className="w-3.5 h-3.5" />
                <span>{t('nvram.openPicoBench')}</span>
              </Button>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={refresh}
              className="text-rose-300 hover:text-white hover:bg-rose-900/40 gap-1.5"
            >
              <RefreshCw className="w-3.5 h-3.5" />
              <span>{t('common.retry')}</span>
            </Button>
          </div>
        </div>
      )}

      {/* Top Bar: Action Panel & Flash Model Header */}
      <NvramActionsCard
        report={report}
        cleaning={cleaning}
        cleanSuccess={cleanSuccess}
        liveCapture={liveCapture}
        capturingLive={capturingLive}
        onExecuteClean={executeSurgicalClean}
        onCaptureLive={() => {
          void captureFromLivePico();
        }}
        onClearLiveCapture={() => {
          void clearLiveCapture();
        }}
        onRefresh={refresh}
      />

      {isAdvanced ? (
        <NvramPatchCard
          defaultInputPath={report?.input_file}
          patching={patching}
          patchSuccess={patchSuccess}
          error={patchError}
          lastOutputFile={report?.output_file}
          onExecutePatch={executeGuardedPatch}
        />
      ) : (
        <div className="rounded-xl bg-bench-900/70 border border-slate-800 px-4 py-3 flex flex-wrap items-center justify-between gap-3 text-xs">
          <span className="text-slate-400 font-sans">
            {t('nvram.basicModeBanner')}
          </span>
          <Button type="button" variant="outline" size="sm" onClick={() => setMode('engineering')} className="gap-1.5 text-xs">
            <Wrench className="w-3.5 h-3.5 text-cyan-400" />
            <span>{t('nvram.revealAdvancedBtn')}</span>
          </Button>
        </div>
      )}

      {/* 4 High-Contrast Telemetry Metric Cards */}
      <NvramMetricsGrid
        report={report}
        selectedSector={selectedSector}
        cleanSuccess={cleanSuccess}
      />

      {/* Interactive Sector Physical Grid & Selected Sector Inspector */}
      <NvramSectorMap
        sectorMap={filteredSectors}
        selectedSector={selectedSector}
        filterType={filterType}
        cleanSuccess={cleanSuccess}
        onSelectSector={setSelectedSector}
        onSelectFilter={setFilterType}
      />

      {/* Microsectors Detailed Audit Table */}
      {report && (
        <NvramMicrosectorsTable report={report} cleanSuccess={cleanSuccess} />
      )}

      {/* Accessible Radix UI Audit & Hash Verification Modal */}
      <NvramAuditDialog
        isOpen={showBackupModal}
        onClose={() => setShowBackupModal(false)}
        report={report}
      />
    </div>
  );
};
