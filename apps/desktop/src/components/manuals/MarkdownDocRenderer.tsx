import React, { useEffect, useRef } from 'react';
import { EmptyState } from '@/components/ui';
import { INLINE_KEY_PREVIEW_LEN } from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import { parseLineRangeAnchor } from '@/utils/docLinkResolver';
import { withUniqueKeys } from '@/utils/formatters';
import {
  blockBaseKey,
  getHeadingClass,
  parseMarkdownBlocks,
  renderInlineTokens,
  type MarkdownBlock,
} from './markdownDocHelpers';

interface MarkdownDocRendererProps {
  content: string;
  isMarkdown: boolean;
  activeAnchor: string | null;
  onNavigateLink: (rawTarget: string) => void;
}

function renderMarkdownBlock(
  block: MarkdownBlock,
  key: string,
  onNavigateLink: (rawTarget: string) => void,
): React.ReactNode {
  switch (block.kind) {
    case 'code':
      return (
        <div key={key} className="my-2 rounded-lg border border-slate-800 bg-slate-900/90 overflow-x-auto">
          {block.lang && (
            <div className="border-b border-slate-800 px-3 py-1 text-[10px] uppercase tracking-wider text-slate-400">
              {block.lang}
            </div>
          )}
          <pre className="p-3 text-xs font-mono text-slate-200 whitespace-pre">{block.lines.join('\n')}</pre>
        </div>
      );
    case 'hr':
      return <hr key={key} className="my-3 border-slate-800" />;
    case 'heading':
      return (
        <div key={key} id={block.slug} className={getHeadingClass(block.level)}>
          {renderInlineTokens(block.rawText, onNavigateLink, key)}
        </div>
      );
    case 'blockquote':
      return (
        <blockquote
          key={key}
          className="my-2 border-l-2 border-cyan-500/60 bg-cyan-950/20 px-3 py-2 rounded-r text-xs text-slate-200 space-y-1"
        >
          {withUniqueKeys(block.lines, (ql) => `${key}-${ql.slice(0, INLINE_KEY_PREVIEW_LEN)}`).map(
            ({ item: ql, key: qk }) => (
              <div key={qk}>{renderInlineTokens(ql, onNavigateLink, qk)}</div>
            ),
          )}
        </blockquote>
      );
    case 'table':
      return (
        <div key={key} className="my-2 overflow-x-auto rounded-lg border border-slate-800">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-slate-900/90 text-cyan-300">
              <tr>
                {withUniqueKeys(block.headers, (h) => `${key}-th-${h}`).map(({ item: h, key: thk }) => (
                  <th key={thk} className="px-2.5 py-1.5 border-b border-slate-800 font-semibold">
                    {renderInlineTokens(h, onNavigateLink, thk)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/70">
              {withUniqueKeys(block.rows, (r) => `${key}-tr-${r.join('|').slice(0, INLINE_KEY_PREVIEW_LEN)}`).map(
                ({ item: row, key: trk }) => (
                  <tr key={trk} className="hover:bg-slate-900/40">
                    {withUniqueKeys(row, (c) => `${trk}-td-${c.slice(0, INLINE_KEY_PREVIEW_LEN)}`).map(
                      ({ item: cell, key: tdk }) => (
                        <td key={tdk} className="px-2.5 py-1.5 text-slate-300 align-top">
                          {renderInlineTokens(cell, onNavigateLink, tdk)}
                        </td>
                      ),
                    )}
                  </tr>
                ),
              )}
            </tbody>
          </table>
        </div>
      );
    case 'list':
      return (
        <ul key={key} className="my-1.5 list-disc pl-5 space-y-1 text-xs text-slate-300">
          {withUniqueKeys(block.items, (item) => `${key}-${item.slice(0, INLINE_KEY_PREVIEW_LEN)}`).map(
            ({ item, key: lik }) => (
              <li key={lik}>{renderInlineTokens(item, onNavigateLink, lik)}</li>
            ),
          )}
        </ul>
      );
    case 'paragraph':
      return (
        <p key={key} className="my-1 text-xs text-slate-300 leading-relaxed">
          {renderInlineTokens(block.text, onNavigateLink, key)}
        </p>
      );
  }
}

export const MarkdownDocRenderer: React.FC<MarkdownDocRendererProps> = ({
  content,
  isMarkdown,
  activeAnchor,
  onNavigateLink,
}) => {
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!activeAnchor || !containerRef.current) return;
    const range = parseLineRangeAnchor(activeAnchor);
    const idTarget = range ? `line-${range.start}` : activeAnchor.toLowerCase();
    const el = containerRef.current.querySelector<HTMLElement>(`[id="${CSS.escape(idTarget)}"]`);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }, [activeAnchor, content]);

  if (!isMarkdown) {
    const range = parseLineRangeAnchor(activeAnchor ?? undefined);
    const lines = content.split(/\r?\n/);
    const isBlank = lines.length === 0 || (lines.length === 1 && lines[0].length === 0);
    if (isBlank) {
      return <EmptyState compact title={t('manuals.engDocs.emptyContent')} />;
    }
    const numberedLines = lines.map((text, zeroIndex) => ({ lineNo: zeroIndex + 1, text }));
    return (
      <div ref={containerRef} className="font-mono text-xs leading-relaxed">
        {numberedLines.map(({ lineNo, text }) => {
          const highlighted = range !== null && lineNo >= range.start && lineNo <= range.end;
          return (
            <div
              key={lineNo}
              id={`line-${lineNo}`}
              className={`flex gap-3 px-2 rounded ${highlighted ? 'bg-amber-500/20 text-amber-100' : 'text-slate-200 hover:bg-slate-900/60'}`}
            >
              <span className="w-10 shrink-0 select-none text-right text-slate-500">{lineNo}</span>
              <span className="whitespace-pre-wrap break-all">{text || ' '}</span>
            </div>
          );
        })}
      </div>
    );
  }

  const blocks = withUniqueKeys(parseMarkdownBlocks(content.split(/\r?\n/)), blockBaseKey);
  return (
    <div ref={containerRef} className="space-y-0.5">
      {blocks.map(({ item, key }) => renderMarkdownBlock(item, key, onNavigateLink))}
    </div>
  );
};
