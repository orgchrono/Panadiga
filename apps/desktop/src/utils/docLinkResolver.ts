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

const HEADER_PREVIEW_LINES = 12;
const REPO_PREFIX_LEN = 'repo:'.length;
const SRC_PREFIX_LEN = 'src:'.length;
const DOCS_PREFIX_LEN = 'docs/'.length;

const ROOT_TRANSLATION_PAIRS: Readonly<Record<string, string>> = Object.freeze({
  'repo:AUDITORIA_COMPLETA.md': 'repo:COMPLETE_AUDIT.md',
  'repo:COMPLETE_AUDIT.md': 'repo:AUDITORIA_COMPLETA.md',
  'repo:README.md': 'repo:README.pt-BR.md',
  'repo:README.pt-BR.md': 'repo:README.md',
});

export function slugifyHeading(text: string): string {
  return text
    .toLowerCase()
    .replace(/[`*_~()[\]{}.,:;!?'"/\\<>@#$%^&+=|]/g, '')
    .trim()
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-');
}

export function extractHeadings(markdown: string): DocHeading[] {
  const initialState: { readonly inFence: boolean; readonly headings: readonly DocHeading[] } = {
    inFence: false,
    headings: [],
  };

  const finalState = markdown.split(/\r?\n/).reduce((acc, line) => {
    const trimmed = line.trim();
    if (trimmed.startsWith('```')) {
      return { ...acc, inFence: !acc.inFence };
    }
    if (acc.inFence) {
      return acc;
    }
    const match = /^(#{1,4})\s+(.+)$/.exec(trimmed);
    if (!match) {
      return acc;
    }
    const text = match[2].replace(/\[([^\]]+)\]\([^)]*\)/g, '$1').trim();
    return {
      ...acc,
      headings: [...acc.headings, { level: match[1].length, text, slug: slugifyHeading(text) }],
    };
  }, initialState);

  return [...finalState.headings];
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

function normalizePosixSegments(baseDirSegments: readonly string[], rel: string): string {
  return rel
    .replace(/\\/g, '/')
    .split('/')
    .reduce<readonly string[]>((stack, part) => {
      if (!part || part === '.') return stack;
      if (part === '..') return stack.length > 0 ? stack.slice(0, -1) : stack;
      return [...stack, part];
    }, baseDirSegments)
    .join('/');
}

function toRepoVirtualPath(currentRelPath: string | null): readonly string[] {
  if (!currentRelPath) return ['docs', 'INDEX.md'];
  if (currentRelPath.startsWith('repo:')) return [currentRelPath.slice(REPO_PREFIX_LEN)];
  if (currentRelPath.startsWith('src:')) return currentRelPath.slice(SRC_PREFIX_LEN).split('/');
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

  const repoRel =
    SRC_PREFIXES.some(p => pathPart.startsWith(p)) || pathPart.startsWith('docs/')
      ? pathPart.replace(/^\.\//, '')
      : normalizePosixSegments(toRepoVirtualPath(currentRelPath).slice(0, -1), pathPart);

  if (ROOT_DOC_NAMES.has(repoRel)) {
    return { targetPath: `repo:${repoRel}`, anchor, isExternal: false };
  }
  if (repoRel.startsWith('docs/')) {
    const insideDocs = repoRel.slice(DOCS_PREFIX_LEN);
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
  docs: readonly EngineeringDocEntry[],
  current: EngineeringDocEntry | null,
): EngineeringDocEntry | null {
  if (!current) return null;
  const headerLines = content.split(/\r?\n/).slice(0, HEADER_PREVIEW_LINES).join('\n');
  const linkRe = /\[[^\]]*\]\(([^)#\s]+(?:#[^)\s]*)?)\)/g;
  const targetLocale =
    current.locale === 'pt-BR' ? 'en-US' : current.locale === 'en-US' ? 'pt-BR' : null;

  const bannerHit = Array.from(headerLines.matchAll(linkRe), match =>
    resolveDocReference(match[1], current.relative_path),
  )
    .filter(resolved => !resolved.isExternal)
    .map(resolved => docs.find(d => d.relative_path === resolved.targetPath))
    .find(
      hit =>
        Boolean(hit) &&
        hit?.relative_path !== current.relative_path &&
        (!targetLocale || hit?.locale === targetLocale || current.locale === 'root'),
    );

  if (bannerHit) {
    return bannerHit;
  }

  const paired = ROOT_TRANSLATION_PAIRS[current.relative_path];
  return paired ? docs.find(d => d.relative_path === paired) ?? null : null;
}
