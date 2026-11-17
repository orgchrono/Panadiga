/**
 * @file viewmodels/useEngineeringDocsNav.ts
 * @brief Hook managing engineering documentation browsing, history stack, anchor resolution, and .log decoding.
 */

import { useCallback, useEffect, useMemo, useState } from 'react';
import type { Am34BootLogReport, DocLocaleFilter, EngineeringDocEntry } from '@/models';
import { openExternalUrl } from '@/services/browserActions';
import {
  decodeEngineeringLog,
  listEngineeringDocs,
  readEngineeringDoc,
} from '@/services/systemService';
import { resolveDocReference } from '@/utils/docLinkResolver';

interface NavHistoryItem {
  readonly entry: EngineeringDocEntry;
  readonly anchor?: string;
}

interface NavHistoryState {
  readonly items: readonly NavHistoryItem[];
  readonly index: number;
}

export function useEngineeringDocsNav(onError: (msg: string) => void) {
  const [engineeringDocs, setEngineeringDocs] = useState<EngineeringDocEntry[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [docLocaleFilter, setDocLocaleFilter] = useState<DocLocaleFilter>('all');
  const [docSearchQuery, setDocSearchQuery] = useState<string>('');
  const [selectedEngDoc, setSelectedEngDoc] = useState<EngineeringDocEntry | null>(null);
  const [engDocContent, setEngDocContent] = useState<string>('');
  const [engDocLoading, setEngDocLoading] = useState<boolean>(false);
  const [activeAnchor, setActiveAnchor] = useState<string | null>(null);
  const [decodedLogReport, setDecodedLogReport] = useState<Am34BootLogReport | null>(null);
  const [navState, setNavState] = useState<NavHistoryState>({ items: [], index: -1 });

  const loadDocEntry = useCallback(
    async (entry: EngineeringDocEntry, anchor?: string, signal?: AbortSignal) => {
      setSelectedEngDoc(entry);
      setActiveAnchor(anchor ?? null);
      setEngDocLoading(true);
      setDecodedLogReport(null);
      try {
        const text = await readEngineeringDoc(entry.relative_path);
        if (signal?.aborted) return;
        setEngDocContent(text);
        if (entry.relative_path.toLowerCase().endsWith('.log')) {
          try {
            const decoded = await decodeEngineeringLog(entry.relative_path);
            if (!signal?.aborted) setDecodedLogReport(decoded);
          } catch {
            // Plain raw log display remains available even if structured decoding is unavailable
          }
        }
      } catch (err) {
        if (!signal?.aborted) {
          onError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (!signal?.aborted) {
          setEngDocLoading(false);
        }
      }
    },
    [onError],
  );

  const openWithHistory = useCallback(
    (entry: EngineeringDocEntry, anchor?: string) => {
      setNavState(prev => {
        const truncated = prev.items.slice(0, prev.index + 1);
        const items = [...truncated, { entry, anchor }];
        return { items, index: items.length - 1 };
      });
      void loadDocEntry(entry, anchor);
    },
    [loadDocEntry],
  );

  const selectEngineeringDoc = useCallback(
    async (entry: EngineeringDocEntry) => {
      openWithHistory(entry);
    },
    [openWithHistory],
  );

  const navigateToReference = useCallback(
    (rawTarget: string) => {
      const resolved = resolveDocReference(rawTarget, selectedEngDoc?.relative_path ?? null);
      if (resolved.isExternal) {
        openExternalUrl(resolved.targetPath);
        return;
      }
      if (selectedEngDoc && resolved.targetPath === selectedEngDoc.relative_path && resolved.anchor) {
        setActiveAnchor(resolved.anchor);
        return;
      }
      const catalogMatch = engineeringDocs.find(d => d.relative_path === resolved.targetPath);
      const targetEntry: EngineeringDocEntry = catalogMatch ?? {
        locale: resolved.targetPath.startsWith('src:') ? 'src' : 'root',
        file_name: resolved.targetPath.split('/').at(-1) ?? resolved.targetPath,
        relative_path: resolved.targetPath,
        title: resolved.targetPath.replace(/^src:/, '').replace(/^repo:/, ''),
        size_bytes: 0,
      };
      openWithHistory(targetEntry, resolved.anchor);
    },
    [engineeringDocs, openWithHistory, selectedEngDoc],
  );

  const goBackDoc = useCallback(() => {
    if (navState.index <= 0) return;
    const nextIndex = navState.index - 1;
    const target = navState.items[nextIndex];
    setNavState(prev => ({ ...prev, index: nextIndex }));
    void loadDocEntry(target.entry, target.anchor);
  }, [loadDocEntry, navState]);

  const goForwardDoc = useCallback(() => {
    if (navState.index < 0 || navState.index >= navState.items.length - 1) return;
    const nextIndex = navState.index + 1;
    const target = navState.items[nextIndex];
    setNavState(prev => ({ ...prev, index: nextIndex }));
    void loadDocEntry(target.entry, target.anchor);
  }, [loadDocEntry, navState]);

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve()
      .then(() => listEngineeringDocs())
      .then(items => {
        if (controller.signal.aborted) return;
        setEngineeringDocs(items);
        const preferred = items.find(d => d.relative_path === 'INDEX.md') ?? items[0];
        if (preferred) {
          setNavState({ items: [{ entry: preferred }], index: 0 });
          void loadDocEntry(preferred, undefined, controller.signal);
        }
      })
      .catch(err => {
        if (!controller.signal.aborted) {
          onError(err instanceof Error ? err.message : String(err));
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) setIsLoading(false);
      });
    return () => controller.abort();
  }, [loadDocEntry, onError]);

  const filteredEngineeringDocs = useMemo(() => {
    const q = docSearchQuery.trim().toLowerCase();
    return engineeringDocs.filter(d => {
      if (docLocaleFilter !== 'all' && d.locale !== docLocaleFilter) return false;
      if (!q) return true;
      return (
        d.title.toLowerCase().includes(q) ||
        d.file_name.toLowerCase().includes(q) ||
        d.relative_path.toLowerCase().includes(q)
      );
    });
  }, [engineeringDocs, docLocaleFilter, docSearchQuery]);

  return {
    engineeringDocs: filteredEngineeringDocs,
    allEngineeringDocs: engineeringDocs,
    isDocsCatalogLoading: isLoading,
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
    canGoBackDoc: navState.index > 0,
    canGoForwardDoc: navState.index >= 0 && navState.index < navState.items.length - 1,
    goBackDoc,
    goForwardDoc,
    selectEngineeringDoc,
    navigateToReference,
  };
}
