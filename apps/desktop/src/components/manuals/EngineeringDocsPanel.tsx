import React, { useMemo } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  BookText,
  Code2,
  FileText,
  Languages,
  Loader2,
  Search,
  Terminal,
} from 'lucide-react';
import type { Am34BootLogReport, DocLocaleFilter, EngineeringDocEntry } from '@/models';
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  ScrollArea,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import { extractHeadings, findTranslationCounterpart } from '@/utils/docLinkResolver';
import { MarkdownDocRenderer } from './MarkdownDocRenderer';

interface EngineeringDocsPanelProps {
  docs: EngineeringDocEntry[];
  allDocs: EngineeringDocEntry[];
  localeFilter: DocLocaleFilter;
  onLocaleFilterChange: (locale: DocLocaleFilter) => void;
  searchQuery: string;
  onSearchQueryChange: (q: string) => void;
  selectedDoc: EngineeringDocEntry | null;
  content: string;
  loading: boolean;
  activeAnchor: string | null;
  onSelectAnchor: (slug: string | null) => void;
  decodedLog: Am34BootLogReport | null;
  canGoBack: boolean;
  canGoForward: boolean;
  onGoBack: () => void;
  onGoForward: () => void;
  onSelectDoc: (doc: EngineeringDocEntry) => void;
  onNavigateReference: (rawTarget: string) => void;
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
            {LOCALE_FILTERS.map(loc => (
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
            ))}
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

        <div className="lg:col-span-8 flex flex-col gap-2 rounded-xl border border-slate-800 bg-slate-950/80 p-3">
          {selectedDoc && (
            <div className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 pb-2">
              <div className="flex items-center gap-1.5 min-w-0">
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  disabled={!canGoBack}
                  onClick={onGoBack}
                  aria-label={t('manuals.engDocs.back')}
                >
                  <ArrowLeft className="w-3.5 h-3.5" />
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  disabled={!canGoForward}
                  onClick={onGoForward}
                  aria-label={t('manuals.engDocs.forward')}
                >
                  <ArrowRight className="w-3.5 h-3.5" />
                </Button>
                {selectedDoc.relative_path.startsWith('src:') && (
                  <Badge variant="outline" className="text-[10px] border-amber-500/50 text-amber-300">
                    <Code2 className="w-3 h-3 mr-1" />
                    {t('manuals.engDocs.sourcePreviewBadge')}
                  </Badge>
                )}
                <span className="text-[11px] font-mono text-slate-400 truncate">
                  {selectedDoc.relative_path}
                </span>
              </div>

              <div className="flex flex-wrap items-center gap-1.5">
                {headings.length > 0 && (
                  <select
                    aria-label={t('manuals.engDocs.tocPlaceholder')}
                    value={activeAnchor ?? ''}
                    onChange={e => onSelectAnchor(e.target.value || null)}
                    className="max-w-[200px] rounded-md border border-slate-800 bg-slate-900 px-2 py-1 text-[11px] font-mono text-slate-200 focus:border-cyan-500 focus:outline-none"
                  >
                    <option value="">{t('manuals.engDocs.tocPlaceholder')}</option>
                    {headings.map((h, idx) => (
                      <option key={`${h.slug}-${idx}`} value={h.slug}>
                        {'  '.repeat(Math.max(0, h.level - 1))}{h.text}
                      </option>
                    ))}
                  </select>
                )}
                {translationCounterpart && (
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => onSelectDoc(translationCounterpart)}
                    data-testid="eng-docs-switch-lang"
                    className="gap-1.5 text-[11px] border-cyan-500/40 text-cyan-200"
                  >
                    <Languages className="w-3.5 h-3.5" />
                    <span>{translationCounterpart.locale}</span>
                  </Button>
                )}
              </div>
            </div>
          )}

          {decodedLog && (
            <div className="rounded-lg border border-cyan-800/50 bg-cyan-950/20 p-2.5 text-[11px] font-mono space-y-1.5">
              <div className="flex items-center gap-1.5 font-semibold text-cyan-300">
                <Terminal className="w-3.5 h-3.5" />
                <span>{t('manuals.engDocs.decodeSummaryTitle')}</span>
              </div>
              <div className="flex flex-wrap gap-2 text-slate-200">
                {decodedLog.loader_version && (
                  <Badge variant="outline">{t('manuals.engDocs.loader')}: {decodedLog.loader_version}</Badge>
                )}
                {decodedLog.boot_mode && (
                  <Badge variant="outline">{t('manuals.engDocs.bootMode')}: {decodedLog.boot_mode}</Badge>
                )}
                {decodedLog.sd_capacity_lba !== null && (
                  <Badge variant="outline">{t('manuals.engDocs.sdLba')}: 0x{decodedLog.sd_capacity_lba.toString(16).toUpperCase()}</Badge>
                )}
                {decodedLog.vdd_volts !== null && (
                  <Badge variant="outline">{t('manuals.engDocs.vdd')}: {decodedLog.vdd_volts.toFixed(2)}V</Badge>
                )}
                {(decodedLog.sdcd_errors.length > 0 || decodedLog.drive_error_codes.length > 0) && (
                  <Badge variant="outline" className="border-amber-500/50 text-amber-200">
                    {t('manuals.engDocs.errors')}: {[...decodedLog.sdcd_errors.map(String), ...decodedLog.drive_error_codes].join(', ')}
                  </Badge>
                )}
              </div>
              {decodedLog.notes.length > 0 && (
                <div className="text-[10px] text-slate-300 space-y-0.5">
                  {decodedLog.notes.slice(0, 3).map((note, ni) => (
                    <div key={ni}>• {note}</div>
                  ))}
                </div>
              )}
            </div>
          )}

          {loading ? (
            <div className="flex items-center gap-2 text-xs font-mono text-cyan-300 p-4">
              <Loader2 className="w-4 h-4 animate-spin" />
              <span>{t('manuals.engDocs.loading')}</span>
            </div>
          ) : selectedDoc ? (
            <ScrollArea className="h-96 pr-2">
              <MarkdownDocRenderer
                content={content}
                isMarkdown={isMarkdown}
                activeAnchor={activeAnchor}
                onNavigateLink={onNavigateReference}
              />
            </ScrollArea>
          ) : (
            <p className="text-xs font-mono text-slate-500 p-4">{t('manuals.engDocs.selectPrompt')}</p>
          )}
        </div>
      </CardContent>
    </Card>
  );
};
