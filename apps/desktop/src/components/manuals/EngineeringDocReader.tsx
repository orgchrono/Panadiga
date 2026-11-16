/**
 * @file components/manuals/EngineeringDocReader.tsx
 * @brief Document toolbar, TOC jump selector, decoded .log telemetry summary, and Markdown viewer pane.
 */

import React from 'react';
import {
  ArrowLeft,
  ArrowRight,
  Code2,
  Languages,
  Loader2,
  Terminal,
} from 'lucide-react';
import type { Am34BootLogReport, EngineeringDocEntry } from '@/models';
import { Badge, Button, ScrollArea } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { DocHeading } from '@/utils/docLinkResolver';
import { MarkdownDocRenderer } from './MarkdownDocRenderer';

export interface EngineeringDocNavigationProps {
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

interface EngineeringDocReaderProps extends EngineeringDocNavigationProps {
  selectedDoc: EngineeringDocEntry | null;
  content: string;
  loading: boolean;
  isMarkdown: boolean;
  headings: DocHeading[];
  translationCounterpart: EngineeringDocEntry | null;
}

export const EngineeringDocReader: React.FC<EngineeringDocReaderProps> = ({
  selectedDoc,
  content,
  loading,
  isMarkdown,
  headings,
  translationCounterpart,
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

  return (
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
                {headings.length === 0 ? (
                  <option value="" disabled>
                    -
                  </option>
                ) : (
                  headings.map((h, idx) => (
                    <option key={`${h.slug}-${idx}`} value={h.slug}>
                      {'  '.repeat(Math.max(0, h.level - 1))}{h.text}
                    </option>
                  ))
                )}
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
          {decodedLog.diagnosis && (
            <div className="text-[11px] font-semibold text-emerald-300">
              {decodedLog.diagnosis}
            </div>
          )}
          {decodedLog.boot_stages && decodedLog.boot_stages.length > 0 && (
            <div className="flex flex-wrap gap-1">
              {decodedLog.boot_stages.map(stage => (
                <Badge key={stage} variant="secondary" className="text-[10px]">
                  {stage}
                </Badge>
              ))}
            </div>
          )}
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
  );
};
