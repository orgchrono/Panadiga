import type { EngineeringDocEntry } from '@/models';

export interface DocHeading {
  level: number;
  text: string;
  slug: string;
}

export interface ResolvedDocRef {
  targetPath: string;
  anchor?: string;
  isExternal: boolean;
}

const ROOT_DOC_NAMES = new Set([
  'README.md',
  'README.pt-BR.md',
  'AUDITORIA_COMPLETA.md',
  'COMPLETE_AUDIT.md',
  'AGENTS.md',
]);

const SRC_PREFIXES = ['crates/', 'firmware/', 'apps/', 'scripts/', 'tools/'];

const SRC_EXT_RE = /\.(rs|c|h|pio|py|ts|tsx|css|json|toml|yml|yaml|md|txt|log|cmd|sh)$/i;

export function slugifyHeading(text: string): string {
  return text
    .toLowerCase()
    .replace(/[`*_~()[\]{}.,:;!?'"/\\<>@#$%^&+=|]/g, '')
    .trim()
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-');
}

export function extractHeadings(markdown: string): DocHeading[] {
  const headings: DocHeading[] = [];
  let inFence = false;
  for (const line of markdown.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed.startsWith('```')) {
      inFence = !inFence;
      continue;
    }
    if (inFence) continue;
    const m = /^(#{1,4})\s+(.+)$/.exec(trimmed);
    if (m) {
      const text = m[2].replace(/\[([^\]]+)\]\([^)]*\)/g, '$1').trim();
      headings.push({ level: m[1].length, text, slug: slugifyHeading(text) });
    }
  }
  return headings;
}

export function parseLineRangeAnchor(anchor?: string): { start: number; end: number } | null {
  if (!anchor) return null;
  const m = /^L(\d+)(?:-L?(\d+))?$/i.exec(anchor.trim());
  if (!m) return null;
  const start = Number.parseInt(m[1], 10);
  const end = m[2] ? Number.parseInt(m[2], 10) : start;
  if (!Number.isFinite(start) || start <= 0) return null;
  return { start, end: Math.max(start, end) };
}

function normalizePosixSegments(baseDirSegments: string[], rel: string): string {
  const stack = [...baseDirSegments];
  for (const part of rel.replace(/\\/g, '/').split('/')) {
    if (!part || part === '.') continue;
    if (part === '..') {
      if (stack.length > 0) stack.pop();
    } else {
      stack.push(part);
    }
  }
  return stack.join('/');
}

function toRepoVirtualPath(currentRelPath: string | null): string[] {
  if (!currentRelPath) return ['docs', 'INDEX.md'];
  if (currentRelPath.startsWith('repo:')) return [currentRelPath.slice(5)];
  if (currentRelPath.startsWith('src:')) return currentRelPath.slice(4).split('/');
  return ['docs', ...currentRelPath.split('/')];
}

export function isNavigableCodePath(text: string): boolean {
  const clean = text.trim().split('#')[0];
  return (
    (SRC_PREFIXES.some(p => clean.startsWith(p)) || clean.startsWith('docs/')) &&
    SRC_EXT_RE.test(clean)
  );
}

export function resolveDocReference(rawHref: string, currentRelPath: string | null): ResolvedDocRef {
  const clean = rawHref.trim();
  if (/^https?:\/\//i.test(clean) || /^mailto:/i.test(clean)) {
    return { targetPath: clean, isExternal: true };
  }
  const hashIdx = clean.indexOf('#');
  const pathPart = hashIdx >= 0 ? clean.slice(0, hashIdx) : clean;
  const anchor = hashIdx >= 0 ? clean.slice(hashIdx + 1) : undefined;

  if (!pathPart) {
    return { targetPath: currentRelPath ?? 'INDEX.md', anchor, isExternal: false };
  }
  if (pathPart.startsWith('repo:') || pathPart.startsWith('src:')) {
    return { targetPath: pathPart, anchor, isExternal: false };
  }

  let repoRel: string;
  if (SRC_PREFIXES.some(p => pathPart.startsWith(p)) || pathPart.startsWith('docs/')) {
    repoRel = pathPart.replace(/^\.\//, '');
  } else {
    const baseSegments = toRepoVirtualPath(currentRelPath);
    baseSegments.pop();
    repoRel = normalizePosixSegments(baseSegments, pathPart);
  }

  if (ROOT_DOC_NAMES.has(repoRel)) {
    return { targetPath: `repo:${repoRel}`, anchor, isExternal: false };
  }
  if (repoRel.startsWith('docs/')) {
    const insideDocs = repoRel.slice(5);
    return {
      targetPath: insideDocs.endsWith('/') ? `${insideDocs}README.md` : insideDocs,
      anchor,
      isExternal: false,
    };
  }
  if (SRC_PREFIXES.some(p => repoRel.startsWith(p)) && SRC_EXT_RE.test(repoRel)) {
    return { targetPath: `src:${repoRel}`, anchor, isExternal: false };
  }
  return { targetPath: repoRel, anchor, isExternal: false };
}

export function findTranslationCounterpart(
  content: string,
  docs: EngineeringDocEntry[],
  current: EngineeringDocEntry | null,
): EngineeringDocEntry | null {
  if (!current) return null;
  const headerLines = content.split(/\r?\n/).slice(0, 12).join('\n');
  const linkRe = /\[[^\]]*\]\(([^)#\s]+(?:#[^)\s]*)?)\)/g;
  const targetLocale = current.locale === 'pt-BR' ? 'en-US' : current.locale === 'en-US' ? 'pt-BR' : null;

  let match: RegExpExecArray | null;
  while ((match = linkRe.exec(headerLines)) !== null) {
    const resolved = resolveDocReference(match[1], current.relative_path);
    if (resolved.isExternal) continue;
    const hit = docs.find(d => d.relative_path === resolved.targetPath);
    if (hit && hit.relative_path !== current.relative_path) {
      if (!targetLocale || hit.locale === targetLocale || current.locale === 'root') {
        return hit;
      }
    }
  }

  const rootPairs: Record<string, string> = {
    'repo:AUDITORIA_COMPLETA.md': 'repo:COMPLETE_AUDIT.md',
    'repo:COMPLETE_AUDIT.md': 'repo:AUDITORIA_COMPLETA.md',
    'repo:README.md': 'repo:README.pt-BR.md',
    'repo:README.pt-BR.md': 'repo:README.md',
  };
  const paired = rootPairs[current.relative_path];
  return paired ? docs.find(d => d.relative_path === paired) ?? null : null;
}
