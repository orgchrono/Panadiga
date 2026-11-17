import React from 'react';
import {
  AlertCircle,
  BarChart2,
  FileCheck2,
  KeyRound,
  Layers,
  PackagePlus,
  RefreshCw,
} from 'lucide-react';
import {
  FrmCodebookTab,
  FrmCorrelationTab,
  FrmEntropyTab,
  FrmMetricsGrid,
  FrmPackagerTab,
  FrmTocTab,
  FrmTopHeader,
} from '@/components/crypto';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  CodebookEntry,
  CryptanalysisReport,
  FrmTocDumpReport,
} from '@/models';
import { useFrmCryptoViewModel } from '@/viewmodels/useFrmCryptoViewModel';
import { useFrmPackager } from '@/viewmodels/useFrmPackager';

type FrmTabId = 'entropy' | 'toc' | 'packager' | 'codebook' | 'correlation';

const FRM_TABS: ReadonlyArray<{
  readonly id: FrmTabId;
  readonly labelKey: TranslationKey;
  readonly Icon: React.ComponentType<{ className?: string }>;
}> = [
  { id: 'entropy', labelKey: 'crypto.tabs.entropy', Icon: BarChart2 },
  { id: 'toc', labelKey: 'crypto.tabs.toc', Icon: Layers },
  { id: 'packager', labelKey: 'crypto.tabs.packager', Icon: PackagePlus },
  { id: 'codebook', labelKey: 'crypto.tabs.codebook', Icon: KeyRound },
  { id: 'correlation', labelKey: 'crypto.tabs.correlation', Icon: FileCheck2 },
];

interface FrmTabBodyProps {
  readonly activeTab: FrmTabId;
  readonly cryptReport: CryptanalysisReport | null;
  readonly tocReport: FrmTocDumpReport | null;
  readonly filteredCodebook: CodebookEntry[];
  readonly codebookFilter: string;
  readonly setCodebookFilter: (val: string) => void;
  readonly packager: ReturnType<typeof useFrmPackager>;
  readonly onInspectPacked: (outPath: string) => void;
}

const FrmTabBody: React.FC<FrmTabBodyProps> = ({
  activeTab,
  cryptReport,
  tocReport,
  filteredCodebook,
  codebookFilter,
  setCodebookFilter,
  packager,
  onInspectPacked,
}) => {
  const { t } = useTranslation();

  switch (activeTab) {
    case 'entropy':
      return cryptReport ? <FrmEntropyTab topBlocks={cryptReport.top_blocks} /> : null;
    case 'toc':
      return tocReport ? (
        <FrmTocTab tocReport={tocReport} />
      ) : (
        <div className="p-8 text-center text-slate-500 font-mono text-xs rounded-xl bg-bench-900 border border-slate-800">
          {t('crypto.tabs.toc')} - {t('shell.ui.loadingToc')}
        </div>
      );
    case 'packager':
      return (
        <FrmPackagerTab
          packLoading={packager.packLoading}
          packError={packager.packError}
          packResult={packager.packResult}
          onPack={packager.packFrm}
          onInspectOutput={onInspectPacked}
          unpackLoading={packager.unpackLoading}
          unpackError={packager.unpackError}
          unpackResult={packager.unpackResult}
          onUnpack={packager.unpackExtrhead}
        />
      );
    case 'codebook':
      return (
        <FrmCodebookTab
          filteredCodebook={filteredCodebook}
          codebookFilter={codebookFilter}
          setCodebookFilter={setCodebookFilter}
        />
      );
    case 'correlation':
      return cryptReport ? (
        <FrmCorrelationTab correlations={cryptReport.hdd_correlations || []} />
      ) : null;
    default:
      return null;
  }
};

export const FrmCryptoView: React.FC = () => {
  const { t } = useTranslation();
  const {
    frmPath,
    setFrmPath,
    cryptReport,
    tocReport,
    filteredCodebook,
    activeTab,
    setActiveTab,
    loading,
    error,
    codebookFilter,
    setCodebookFilter,
    refresh,
  } = useFrmCryptoViewModel();
  const packager = useFrmPackager();

  const handleInspectPacked = (outPath: string) => {
    setFrmPath(outPath);
    setActiveTab('toc');
  };

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center h-96 gap-3 text-slate-500 font-mono text-xs">
        <RefreshCw className="w-6 h-6 animate-spin text-purple-400" />
        <span>{t('crypto.loading')}</span>
      </div>
    );
  }

  if (!cryptReport && activeTab !== 'packager') {
    return (
      <div className="space-y-6 max-w-7xl mx-auto pb-12 font-mono">
        <FrmTopHeader
          containers={packager.containers}
          selectedPath={frmPath}
          onSelectPath={setFrmPath}
          onRefresh={refresh}
          loading={loading}
        />
        <div className="max-w-2xl mx-auto my-8 p-6 rounded-2xl bg-bench-900 border border-slate-800 text-center space-y-4 font-mono">
          <div className="w-12 h-12 rounded-full bg-rose-500/10 text-rose-400 border border-rose-500/30 flex items-center justify-center mx-auto">
            <AlertCircle className="w-6 h-6" />
          </div>
          <div>
            <h3 className="text-sm font-bold text-slate-100">
              {t('crypto.notFound.title', { path: frmPath })}
            </h3>
            <p className="text-xs text-slate-400 mt-1">
              {error || t('crypto.notFound.defaultError')}
            </p>
          </div>
          <div className="flex items-center justify-center gap-3 flex-wrap">
            <button
              type="button"
              onClick={refresh}
              className="px-4 py-2 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-xs inline-flex items-center gap-2 transition-colors"
            >
              <RefreshCw className="w-4 h-4" />
              <span>{t('crypto.notFound.reloadBtn')}</span>
            </button>
            <button
              type="button"
              onClick={() => setActiveTab('packager')}
              className="px-4 py-2 rounded-xl bg-purple-500/20 hover:bg-purple-500/30 text-purple-300 border border-purple-500/40 font-bold text-xs inline-flex items-center gap-2 transition-colors"
            >
              <PackagePlus className="w-4 h-4" />
              <span>{t('crypto.tabs.packager')}</span>
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-7xl mx-auto pb-12 font-mono">
      <FrmTopHeader
        frmFile={cryptReport?.frm_file}
        containers={packager.containers}
        selectedPath={frmPath}
        onSelectPath={setFrmPath}
        onRefresh={refresh}
        loading={loading}
      />

      {cryptReport && <FrmMetricsGrid cryptReport={cryptReport} tocReport={tocReport} />}

      <div className="flex items-center gap-2 border-b border-slate-800 pb-2 text-xs overflow-x-auto">
        {FRM_TABS.length === 0
          ? null
          : FRM_TABS.map(({ id, labelKey, Icon }) => {
              const active = activeTab === id;
              return (
                <button
                  key={id}
                  type="button"
                  onClick={() => setActiveTab(id)}
                  aria-current={active ? 'page' : undefined}
                  className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-purple-500/50 ${
                    active
                      ? 'bg-purple-500/10 text-purple-300 border border-purple-500/30'
                      : 'text-slate-400 hover:text-slate-200'
                  }`}
                >
                  <Icon className="w-3.5 h-3.5" />
                  <span>{t(labelKey)}</span>
                </button>
              );
            })}
      </div>

      <FrmTabBody
        activeTab={activeTab}
        cryptReport={cryptReport}
        tocReport={tocReport}
        filteredCodebook={filteredCodebook}
        codebookFilter={codebookFilter}
        setCodebookFilter={setCodebookFilter}
        packager={packager}
        onInspectPacked={handleInspectPacked}
      />
    </div>
  );
};
