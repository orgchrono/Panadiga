import React, { useMemo } from 'react';
import { BookText, FileText, Search } from 'lucide-react';
import type { DocLocaleFilter, EngineeringDocEntry } from '@/models';
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { extractHeadings, findTranslationCounterpart } from '@/utils/docLinkResolver';
import {
  EngineeringDocReader,
  type EngineeringDocNavigationProps,
} from './EngineeringDocReader';

interface EngineeringDocsPanelProps extends EngineeringDocNavigationProps {
  docs: EngineeringDocEntry[];
  allDocs: EngineeringDocEntry[];
  localeFilter: DocLocaleFilter;
  onLocaleFilterChange: (locale: DocLocaleFilter) => void;
  searchQuery: string;
  onSearchQueryChange: (q: string) => void;
  selectedDoc: EngineeringDocEntry | null;
  content: string;
  loading: boolean;
}

const LOCALE_FILTERS: DocLocaleFilter[] = [
  'all',
  'pt-BR',
  'en-US',
  'audits',
  'pseudocode',
  'raw_txt',
  'debug_logs',
  'root',
];

export const EngineeringDocsPanel: React.FC<EngineeringDocsPanelProps> = ({
  docs,
  allDocs,
  localeFilter,
  onLocaleFilterChange,
  searchQuery,
  onSearchQueryChange,
  selectedDoc,
  content,
  loading,
  activeAnchor,
  onSelectAnchor,
  decodedLog,
  canGoBack,
  canGoForward,
  onGoBack,
  onGoForward,
  onSelectDoc,
  onNavigateReference,
}) => {
  const { t } = useTranslation();

  const isMarkdown = useMemo(() => {
    const p = selectedDoc?.relative_path.toLowerCase() ?? '';
    return p.endsWith('.md') || (!p.startsWith('src:') && !p.endsWith('.log') && !p.endsWith('.txt'));
  }, [selectedDoc]);

  const headings = useMemo(() => (isMarkdown ? extractHeadings(content) : []), [content, isMarkdown]);

  const translationCounterpart = useMemo(
    () => findTranslationCounterpart(content, allDocs, selectedDoc),
    [content, allDocs, selectedDoc],
  );

  return (
    <Card data-testid="engineering-docs-panel">
      <CardHeader>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <BookText className="w-4 h-4 text-cyan-400" />
              <CardTitle>{t('manuals.engDocs.title')}</CardTitle>
            </div>
            <CardDescription>{t('manuals.engDocs.subtitle')}</CardDescription>
          </div>

          <div className="flex flex-wrap gap-1.5" role="group" aria-label={t('manuals.engDocs.title')}>
            {LOCALE_FILTERS.length === 0 ? (
              <span className="text-xs font-mono text-slate-500">-</span>
            ) : (
              LOCALE_FILTERS.map(loc => (
                <Button
                  key={loc}
                  type="button"
                  size="sm"
                  variant={localeFilter === loc ? 'default' : 'outline'}
                  onClick={() => onLocaleFilterChange(loc)}
                  data-testid={`eng-docs-filter-${loc}`}
                >
                  {loc === 'all' ? t('manuals.engDocs.filterAll') : loc}
                </Button>
              ))
            )}
          </div>
        </div>
      </CardHeader>

      <CardContent className="grid grid-cols-1 lg:grid-cols-12 gap-4">
        <div className="lg:col-span-4 space-y-2">
          <div className="relative">
            <Search className="w-3.5 h-3.5 text-slate-500 absolute left-2.5 top-2.5 pointer-events-none" />
            <input
              type="text"
              value={searchQuery}
              onChange={e => onSearchQueryChange(e.target.value)}
              placeholder={t('manuals.engDocs.searchPlaceholder')}
              aria-label={t('manuals.engDocs.searchPlaceholder')}
              data-testid="eng-docs-search-input"
              className="w-full rounded-lg border border-slate-800 bg-slate-950/90 pl-8 pr-3 py-1.5 text-xs font-mono text-slate-200 placeholder:text-slate-500 focus:border-cyan-500 focus:outline-none"
            />
          </div>

          <div className="space-y-1.5 max-h-96 overflow-y-auto pr-1">
            {docs.length === 0 ? (
              <p className="text-xs font-mono text-slate-500 p-3">{t('manuals.engDocs.empty')}</p>
            ) : (
              docs.map(doc => {
                const active = selectedDoc?.relative_path === doc.relative_path;
                return (
                  <button
                    key={doc.relative_path}
                    type="button"
                    onClick={() => onSelectDoc(doc)}
                    className={`w-full text-left rounded-xl border p-2.5 transition-colors font-mono ${
                      active
                        ? 'border-cyan-500/60 bg-cyan-500/10 text-cyan-200'
                        : 'border-slate-800 bg-slate-950/60 text-slate-300 hover:border-slate-700'
                    }`}
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="inline-flex items-center gap-1.5 text-xs font-semibold truncate">
                        <FileText className="w-3.5 h-3.5 shrink-0 text-cyan-400" />
                        {doc.title}
                      </span>
                      <Badge variant="outline" className="text-[10px] shrink-0">
                        {doc.locale}
                      </Badge>
                    </div>
                    <div className="text-[10px] text-slate-500 truncate mt-0.5">
                      {doc.relative_path}
                    </div>
                  </button>
                );
              })
            )}
          </div>
        </div>

        <EngineeringDocReader
          selectedDoc={selectedDoc}
          content={content}
          loading={loading}
          isMarkdown={isMarkdown}
          headings={headings}
          translationCounterpart={translationCounterpart}
          activeAnchor={activeAnchor}
          onSelectAnchor={onSelectAnchor}
          decodedLog={decodedLog}
          canGoBack={canGoBack}
          canGoForward={canGoForward}
          onGoBack={onGoBack}
          onGoForward={onGoForward}
          onSelectDoc={onSelectDoc}
          onNavigateReference={onNavigateReference}
        />
      </CardContent>
    </Card>
  );
};
