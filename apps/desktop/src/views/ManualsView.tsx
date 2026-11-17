import React from 'react';
import { useTranslation } from '@/i18n/useTranslation';
import {
  MANUALS_SPLIT_INITIAL_PCT,
  MANUALS_SPLIT_MAX_PCT,
  MANUALS_SPLIT_MIN_PCT,
} from '@/constants';
import { useManualsViewModel } from '@/viewmodels/useManualsViewModel';
import { useRagViewModel } from '@/viewmodels/useRagViewModel';
import { StatusBadge } from '@/components/StatusBadge';
import { ResizableSplitPane } from '@/components/ui';
import { RagAssistantPanel } from '@/components/rag';
import {
  EngineeringDocsPanel,
  ManualsSearchCard,
  ManualsResultsList,
  ManualsDocCatalog,
} from '@/components/manuals';
import { BookOpen } from 'lucide-react';

export const ManualsView: React.FC = () => {
  const { t } = useTranslation();
  const {
    documents,
    searchResults,
    searchQuery,
    setSearchQuery,
    activeTerms,
    selectedDoc,
    setSelectedDoc,
    loading,
    engineeringDocs,
    allEngineeringDocs,
    docLocaleFilter,
    setDocLocaleFilter,
    docSearchQuery,
    setDocSearchQuery,
    selectedEngDoc,
    engDocContent,
    engDocLoading,
    activeAnchor,
    setActiveAnchor,
    decodedLogReport,
    canGoBackDoc,
    canGoForwardDoc,
    goBackDoc,
    goForwardDoc,
    selectEngineeringDoc,
    navigateToReference,
    executeSearch,
    selectTerm,
  } = useManualsViewModel();

  const ragVm = useRagViewModel();

  return (
    <div className="space-y-6 max-w-7xl mx-auto pb-12 font-mono">
      {/* Top Header */}
      <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 p-5 rounded-2xl bg-bench-900 border border-slate-800 shadow-xl">
        <div className="flex items-center gap-3.5">
          <div className="p-3 rounded-xl bg-blue-500/10 text-blue-400 border border-blue-500/30">
            <BookOpen className="w-6 h-6" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-base font-bold text-slate-100">
                {t('manuals.header.title')}
              </h2>
              <StatusBadge variant="info" label={t('manuals.header.badge')} />
            </div>
            <p className="text-xs text-slate-400 mt-0.5">
              {t('manuals.header.subtitle')}
            </p>
          </div>
        </div>

        <div className="text-right text-xs text-slate-400">
          <span>{t('manuals.header.size')}</span>
        </div>
      </div>

      <RagAssistantPanel viewModel={ragVm} />

      <ManualsSearchCard
        searchQuery={searchQuery}
        setSearchQuery={setSearchQuery}
        activeTerms={activeTerms}
        executeSearch={executeSearch}
        selectTerm={selectTerm}
      />

      <ResizableSplitPane
        initialLeftPercent={MANUALS_SPLIT_INITIAL_PCT}
        minLeftPercent={MANUALS_SPLIT_MIN_PCT}
        maxLeftPercent={MANUALS_SPLIT_MAX_PCT}
        storageKey="panadiga.split.manuals"
        ariaLabel={t('manuals.meta.splitAria')}
        left={
          <ManualsResultsList
            searchResults={searchResults}
            loading={loading}
          />
        }
        right={
          <ManualsDocCatalog
            documents={documents}
            selectedDoc={selectedDoc}
            setSelectedDoc={setSelectedDoc}
          />
        }
      />

      <EngineeringDocsPanel
        docs={engineeringDocs}
        allDocs={allEngineeringDocs}
        localeFilter={docLocaleFilter}
        onLocaleFilterChange={setDocLocaleFilter}
        searchQuery={docSearchQuery}
        onSearchQueryChange={setDocSearchQuery}
        selectedDoc={selectedEngDoc}
        content={engDocContent}
        loading={engDocLoading}
        activeAnchor={activeAnchor}
        onSelectAnchor={setActiveAnchor}
        decodedLog={decodedLogReport}
        canGoBack={canGoBackDoc}
        canGoForward={canGoForwardDoc}
        onGoBack={goBackDoc}
        onGoForward={goForwardDoc}
        onSelectDoc={selectEngineeringDoc}
        onNavigateReference={navigateToReference}
      />
    </div>
  );
};
