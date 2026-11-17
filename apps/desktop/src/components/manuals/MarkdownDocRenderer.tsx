import React, { useEffect, useRef } from 'react';
import { Code2, ExternalLink } from 'lucide-react';
import { EmptyState } from '@/components/ui';
import {
  INLINE_KEY_PREVIEW_LEN,
  MD_BOLD_WRAPPER_LEN,
  MD_FENCE_MARKER_LEN,
} from '@/constants';
import { useTranslation } from '@/i18n/useTranslation';
import { isNavigableCodePath, parseLineRangeAnchor, slugifyHeading } from '@/utils/docLinkResolver';
import { withUniqueKeys } from '@/utils/formatters';

interface MarkdownDocRendererProps {
  content: string;
  isMarkdown: boolean;
  activeAnchor: string | null;
  onNavigateLink: (rawTarget: string) => void;
}

const INLINE_TOKEN_RE = new RegExp(
  '(\\[[^\\]]+\\]\\([^)]+\\)|\\x60[^\\x60]+\\x60|\\*\\*[^*]+\\*\\*)',
  'g',
);
const TABLE_SEPARATOR_RE = /^\|[\s:|-]+\|/;
const HEADING_RE = /^(#{1,4})\s+(.+)$/;
const LIST_ITEM_RE = /^([-*]|\d+\.)\s+/;
const HR_RE = /^---+$/;
const MD_BOLD_HALF_LEN = 2;

type MarkdownBlock =
  | { readonly kind: 'code'; readonly lang: string; readonly lines: readonly string[] }
  | { readonly kind: 'hr' }
  | { readonly kind: 'heading'; readonly level: number; readonly rawText: string; readonly slug: string }
  | { readonly kind: 'blockquote'; readonly lines: readonly string[] }
  | { readonly kind: 'table'; readonly headers: readonly string[]; readonly rows: readonly (readonly string[])[] }
  | { readonly kind: 'list'; readonly items: readonly string[] }
  | { readonly kind: 'paragraph'; readonly text: string };

type ActiveBlock =
  | { readonly kind: 'code'; readonly lang: string; readonly lines: readonly string[] }
  | { readonly kind: 'blockquote'; readonly lines: readonly string[] }
  | { readonly kind: 'list'; readonly items: readonly string[] }
  | {
      readonly kind: 'table';
      readonly headers: readonly string[];
      readonly skipSeparator: boolean;
      readonly rows: readonly (readonly string[])[];
    };

interface ParserState {
  readonly blocks: readonly MarkdownBlock[];
  readonly active: ActiveBlock | null;
}

function renderLinkToken(
  label: string,
  href: string,
  key: string,
  onNavigateLink: (rawTarget: string) => void,
): React.ReactNode {
  const cleanLabel = label.replace(/\*\*/g, '');
  if (/^https?:\/\//i.test(href)) {
    return (
      <a
        key={key}
        href={href}
        target="_blank"
        rel="noreferrer"
        className="inline-flex items-center gap-0.5 text-cyan-300 underline decoration-cyan-500/50 hover:text-cyan-200"
      >
        {cleanLabel}
        <ExternalLink className="w-3 h-3 inline" />
      </a>
    );
  }
  return (
    <button
      key={key}
      type="button"
      onClick={() => onNavigateLink(href)}
      className="text-left text-cyan-300 underline decoration-cyan-500/60 hover:text-cyan-100 cursor-pointer"
    >
      {cleanLabel}
    </button>
  );
}

function renderSingleInlineToken(
  part: string,
  key: string,
  onNavigateLink: (rawTarget: string) => void,
): React.ReactNode {
  const linkMatch = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(part);
  if (linkMatch) {
    return renderLinkToken(linkMatch[1], linkMatch[2], key, onNavigateLink);
  }

  if (part.startsWith('`') && part.endsWith('`') && part.length >= MD_BOLD_HALF_LEN) {
    const codeText = part.slice(1, part.length - 1);
    if (isNavigableCodePath(codeText)) {
      return (
        <button
          key={key}
          type="button"
          onClick={() => onNavigateLink(codeText)}
          className="inline-flex items-center gap-1 rounded bg-cyan-950/70 border border-cyan-700/50 px-1.5 py-0.5 text-[11px] font-mono text-cyan-200 hover:bg-cyan-900/70 cursor-pointer"
        >
          <Code2 className="w-3 h-3 shrink-0" />
          {codeText}
        </button>
      );
    }
    return (
      <code key={key} className="rounded bg-slate-900 px-1.5 py-0.5 text-[11px] font-mono text-amber-200">
        {codeText}
      </code>
    );
  }

  if (part.startsWith('**') && part.endsWith('**') && part.length >= MD_BOLD_WRAPPER_LEN) {
    return (
      <strong key={key} className="font-semibold text-slate-100">
        {part.slice(MD_BOLD_HALF_LEN, part.length - MD_BOLD_HALF_LEN)}
      </strong>
    );
  }

  return <React.Fragment key={key}>{part}</React.Fragment>;
}

function renderInlineTokens(
  text: string,
  onNavigateLink: (rawTarget: string) => void,
  keyPrefix: string,
): React.ReactNode[] {
  const parts = text.split(INLINE_TOKEN_RE);
  return withUniqueKeys(parts, (part) => `${keyPrefix}-${part.slice(0, INLINE_KEY_PREVIEW_LEN)}`).map(
    ({ item, key }) => renderSingleInlineToken(item, key, onNavigateLink),
  );
}

function splitTableRow(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map((cell) => cell.trim());
}

function flushActiveBlock(state: ParserState): ParserState {
  if (!state.active) return state;
  const completed: MarkdownBlock =
    state.active.kind === 'table'
      ? { kind: 'table', headers: state.active.headers, rows: state.active.rows }
      : state.active;
  return { blocks: [...state.blocks, completed], active: null };
}

function startBlockFromLine(trimmed: string, nextTrimmed: string): ActiveBlock | MarkdownBlock {
  if (trimmed.startsWith('```')) {
    return { kind: 'code', lang: trimmed.slice(MD_FENCE_MARKER_LEN).trim(), lines: [] };
  }
  if (HR_RE.test(trimmed)) {
    return { kind: 'hr' };
  }
  const headingMatch = HEADING_RE.exec(trimmed);
  if (headingMatch) {
    const rawText = headingMatch[2];
    const slug = slugifyHeading(rawText.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1'));
    return { kind: 'heading', level: headingMatch[1].length, rawText, slug };
  }
  if (trimmed.startsWith('>')) {
    return { kind: 'blockquote', lines: [trimmed.replace(/^>\s?/, '')] };
  }
  if (trimmed.startsWith('|') && TABLE_SEPARATOR_RE.test(nextTrimmed)) {
    return { kind: 'table', headers: splitTableRow(trimmed), skipSeparator: true, rows: [] };
  }
  if (LIST_ITEM_RE.test(trimmed)) {
    return { kind: 'list', items: [trimmed.replace(LIST_ITEM_RE, '')] };
  }
  return { kind: 'paragraph', text: trimmed };
}

function isFinalizedBlock(value: NonNullable<ParserState['active']> | MarkdownBlock): value is MarkdownBlock {
  return (
    value.kind === 'hr' ||
    value.kind === 'heading' ||
    value.kind === 'paragraph' ||
    ('skipSeparator' in value === false && value.kind === 'table')
  );
}

function continueActiveBlock(
  state: ParserState,
  line: string,
  trimmed: string,
): ParserState | null {
  const { active } = state;
  if (!active) return null;

  if (active.kind === 'code') {
    if (trimmed.startsWith('```')) {
      return { blocks: [...state.blocks, active], active: null };
    }
    return { ...state, active: { ...active, lines: [...active.lines, line] } };
  }
  if (active.kind === 'blockquote' && trimmed.startsWith('>')) {
    return {
      ...state,
      active: { ...active, lines: [...active.lines, trimmed.replace(/^>\s?/, '')] },
    };
  }
  if (active.kind === 'list' && LIST_ITEM_RE.test(trimmed)) {
    return {
      ...state,
      active: { ...active, items: [...active.items, trimmed.replace(LIST_ITEM_RE, '')] },
    };
  }
  if (active.kind === 'table') {
    if (active.skipSeparator) {
      return { ...state, active: { ...active, skipSeparator: false } };
    }
    if (trimmed.startsWith('|')) {
      return { ...state, active: { ...active, rows: [...active.rows, splitTableRow(trimmed)] } };
    }
  }
  return null;
}

function parseMarkdownBlocks(lines: readonly string[]): readonly MarkdownBlock[] {
  const folded = lines.reduce<ParserState>(
    (state, line, index) => {
      const trimmed = line.trim();
      const continued = continueActiveBlock(state, line, trimmed);
      if (continued) return continued;

      const flushed = flushActiveBlock(state);
      if (!trimmed) return flushed;

      const nextTrimmed = lines[index + 1]?.trim() ?? '';
      const started = startBlockFromLine(trimmed, nextTrimmed);
      if (isFinalizedBlock(started)) {
        return { blocks: [...flushed.blocks, started], active: null };
      }
      return { blocks: flushed.blocks, active: started };
    },
    { blocks: [], active: null },
  );
  return flushActiveBlock(folded).blocks;
}

function blockBaseKey(block: MarkdownBlock): string {
  switch (block.kind) {
    case 'code':
      return `code-${block.lang}-${block.lines[0]?.slice(0, INLINE_KEY_PREVIEW_LEN) ?? ''}`;
    case 'hr':
      return 'hr';
    case 'heading':
      return `h-${block.level}-${block.slug}`;
    case 'blockquote':
      return `q-${block.lines[0]?.slice(0, INLINE_KEY_PREVIEW_LEN) ?? ''}`;
    case 'table':
      return `tbl-${block.headers.join('|').slice(0, INLINE_KEY_PREVIEW_LEN)}`;
    case 'list':
      return `ul-${block.items[0]?.slice(0, INLINE_KEY_PREVIEW_LEN) ?? ''}`;
    case 'paragraph':
      return `p-${block.text.slice(0, INLINE_KEY_PREVIEW_LEN)}`;
  }
}

function getHeadingClass(level: number): string {
  if (level === 1) {
    return 'text-base font-bold text-cyan-300 mt-2 mb-2 border-b border-slate-800 pb-1.5';
  }
  if (level === MD_BOLD_HALF_LEN) {
    return 'text-sm font-bold text-slate-100 mt-4 mb-1.5';
  }
  return 'text-xs font-bold text-cyan-200 mt-3 mb-1';
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
