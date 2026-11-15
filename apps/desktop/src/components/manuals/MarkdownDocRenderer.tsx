import React, { useEffect, useRef } from 'react';
import { Code2, ExternalLink } from 'lucide-react';
import { EmptyState } from '@/components/ui';
import { isNavigableCodePath, parseLineRangeAnchor, slugifyHeading } from '@/utils/docLinkResolver';

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

function renderInlineTokens(
  text: string,
  onNavigateLink: (rawTarget: string) => void,
  keyPrefix: string,
): React.ReactNode[] {
  const parts = text.split(INLINE_TOKEN_RE);
  return parts.map((part, idx) => {
    const key = `${keyPrefix}-${idx}`;
    const linkMatch = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(part);
    if (linkMatch) {
      const [, label, href] = linkMatch;
      const cleanLabel = label.replace(/\*\*/g, '');
      const isWeb = /^https?:\/\//i.test(href);
      if (isWeb) {
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

    if (part.startsWith('`') && part.endsWith('`') && part.length >= 2) {
      const codeText = part.slice(1, -1);
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

    if (part.startsWith('**') && part.endsWith('**') && part.length >= 4) {
      return <strong key={key} className="font-semibold text-slate-100">{part.slice(2, -2)}</strong>;
    }

    return <React.Fragment key={key}>{part}</React.Fragment>;
  });
}

function splitTableRow(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map(c => c.trim());
}

export const MarkdownDocRenderer: React.FC<MarkdownDocRendererProps> = ({
  content,
  isMarkdown,
  activeAnchor,
  onNavigateLink,
}) => {
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
      return <EmptyState compact title="Documento sem conteúdo" />;
    }
    return (
      <div ref={containerRef} className="font-mono text-xs leading-relaxed">
        {lines.map((line, idx) => {
          const lineNo = idx + 1;
          const highlighted = range !== null && lineNo >= range.start && lineNo <= range.end;
          return (
            <div
              key={lineNo}
              id={`line-${lineNo}`}
              className={`flex gap-3 px-2 rounded ${highlighted ? 'bg-amber-500/20 text-amber-100' : 'text-slate-200 hover:bg-slate-900/60'}`}
            >
              <span className="w-10 shrink-0 select-none text-right text-slate-500">{lineNo}</span>
              <span className="whitespace-pre-wrap break-all">{line || ' '}</span>
            </div>
          );
        })}
      </div>
    );
  }

  const lines = content.split(/\r?\n/);
  const nodes: React.ReactNode[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    if (!trimmed) {
      i += 1;
      continue;
    }

    if (trimmed.startsWith('```')) {
      const lang = trimmed.slice(3).trim();
      const codeLines: string[] = [];
      i += 1;
      while (i < lines.length && !lines[i].trim().startsWith('```')) {
        codeLines.push(lines[i]);
        i += 1;
      }
      i += 1;
      nodes.push(
        <div key={`code-${i}`} className="my-2 rounded-lg border border-slate-800 bg-slate-900/90 overflow-x-auto">
          {lang && (
            <div className="border-b border-slate-800 px-3 py-1 text-[10px] uppercase tracking-wider text-slate-400">
              {lang}
            </div>
          )}
          <pre className="p-3 text-xs font-mono text-slate-200 whitespace-pre">{codeLines.join('\n')}</pre>
        </div>,
      );
      continue;
    }

    if (/^---+$/.test(trimmed)) {
      nodes.push(<hr key={`hr-${i}`} className="my-3 border-slate-800" />);
      i += 1;
      continue;
    }

    const headingMatch = /^(#{1,4})\s+(.+)$/.exec(trimmed);
    if (headingMatch) {
      const level = headingMatch[1].length;
      const rawText = headingMatch[2];
      const slug = slugifyHeading(rawText.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1'));
      const cls =
        level === 1
          ? 'text-base font-bold text-cyan-300 mt-2 mb-2 border-b border-slate-800 pb-1.5'
          : level === 2
          ? 'text-sm font-bold text-slate-100 mt-4 mb-1.5'
          : 'text-xs font-bold text-cyan-200 mt-3 mb-1';
      nodes.push(
        <div key={`h-${i}`} id={slug} className={cls}>
          {renderInlineTokens(rawText, onNavigateLink, `h-${i}`)}
        </div>,
      );
      i += 1;
      continue;
    }

    if (trimmed.startsWith('>')) {
      const quoteLines: string[] = [];
      while (i < lines.length && lines[i].trim().startsWith('>')) {
        quoteLines.push(lines[i].trim().replace(/^>\s?/, ''));
        i += 1;
      }
      nodes.push(
        <blockquote key={`q-${i}`} className="my-2 border-l-2 border-cyan-500/60 bg-cyan-950/20 px-3 py-2 rounded-r text-xs text-slate-200 space-y-1">
          {quoteLines.map((ql, qi) => (
            <div key={qi}>{renderInlineTokens(ql, onNavigateLink, `q-${i}-${qi}`)}</div>
          ))}
        </blockquote>,
      );
      continue;
    }

    if (trimmed.startsWith('|') && i + 1 < lines.length && /^\|[\s:|-]+\|/.test(lines[i + 1].trim())) {
      const headers = splitTableRow(trimmed);
      i += 2;
      const rows: string[][] = [];
      while (i < lines.length && lines[i].trim().startsWith('|')) {
        rows.push(splitTableRow(lines[i]));
        i += 1;
      }
      nodes.push(
        <div key={`tbl-${i}`} className="my-2 overflow-x-auto rounded-lg border border-slate-800">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-slate-900/90 text-cyan-300">
              <tr>
                {headers.map((h, hi) => (
                  <th key={hi} className="px-2.5 py-1.5 border-b border-slate-800 font-semibold">
                    {renderInlineTokens(h, onNavigateLink, `th-${i}-${hi}`)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/70">
              {rows.map((row, ri) => (
                <tr key={ri} className="hover:bg-slate-900/40">
                  {row.map((cell, ci) => (
                    <td key={ci} className="px-2.5 py-1.5 text-slate-300 align-top">
                      {renderInlineTokens(cell, onNavigateLink, `td-${i}-${ri}-${ci}`)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>,
      );
      continue;
    }

    if (/^([-*]|\d+\.)\s+/.test(trimmed)) {
      const listItems: string[] = [];
      while (i < lines.length && /^([-*]|\d+\.)\s+/.test(lines[i].trim())) {
        listItems.push(lines[i].trim().replace(/^([-*]|\d+\.)\s+/, ''));
        i += 1;
      }
      nodes.push(
        <ul key={`ul-${i}`} className="my-1.5 list-disc pl-5 space-y-1 text-xs text-slate-300">
          {listItems.map((item, li) => (
            <li key={li}>{renderInlineTokens(item, onNavigateLink, `li-${i}-${li}`)}</li>
          ))}
        </ul>,
      );
      continue;
    }

    nodes.push(
      <p key={`p-${i}`} className="my-1 text-xs text-slate-300 leading-relaxed">
        {renderInlineTokens(trimmed, onNavigateLink, `p-${i}`)}
      </p>,
    );
    i += 1;
  }

  return <div ref={containerRef} className="space-y-0.5">{nodes}</div>;
};
