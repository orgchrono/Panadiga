/**
 * @file viewmodels/useEngineeringDocsNav.ts
 * @brief Hook managing engineering documentation browsing, history stack, anchor resolution, and .log decoding.
 */

import { useCallback, useEffect, useMemo, useState, type MutableRefObject } from 'react';
import type { Am34BootLogReport, DocLocaleFilter, EngineeringDocEntry } from '@/models';
import { invokeCommand } from '@/services/ipc';
import { resolveDocReference } from '@/utils/docLinkResolver';

interface NavHistoryItem {
  entry: EngineeringDocEntry;
  anchor?: string;
}

export function useEngineeringDocsNav(
  isMountedRef: MutableRefObject<boolean>,
  onError: (msg: string) => void,
) {
  const [engineeringDocs, setEngineeringDocs] = useState<EngineeringDocEntry[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [docLocaleFilter, setDocLocaleFilter] = useState<DocLocaleFilter>('all');
  const [docSearchQuery, setDocSearchQuery] = useState<string>('');
  const [selectedEngDoc, setSelectedEngDoc] = useState<EngineeringDocEntry | null>(null);
  const [engDocContent, setEngDocContent] = useState<string>('');
  const [engDocLoading, setEngDocLoading] = useState<boolean>(false);
  const [activeAnchor, setActiveAnchor] = useState<string | null>(null);
  const [decodedLogReport, setDecodedLogReport] = useState<Am34BootLogReport | null>(null);
  const [navHistory, setNavHistory] = useState<NavHistoryItem[]>([]);
  const [navIndex, setNavIndex] = useState<number>(-1);

  const loadDocEntry = useCallback(
    async (entry: EngineeringDocEntry, anchor?: string) => {
      setSelectedEngDoc(entry);
      setActiveAnchor(anchor ?? null);
      setEngDocLoading(true);
      setDecodedLogReport(null);
      try {
        const text = await invokeCommand<string>('read_engineering_doc', {
          relativePath: entry.relative_path,
        });
        if (!isMountedRef.current) return;
        setEngDocContent(text);
        if (entry.relative_path.toLowerCase().endsWith('.log')) {
          try {
            const decoded = await invokeCommand<Am34BootLogReport>('decode_engineering_log', {
              relativePath: entry.relative_path,
            });
            if (isMountedRef.current) setDecodedLogReport(decoded);
          } catch {
            // Plain raw log display remains available even if structured decoding is mocked out
          }
        }
      } catch (err) {
        if (isMountedRef.current) {
          onError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (isMountedRef.current) {
          setEngDocLoading(false);
        }
      }
    },
    [isMountedRef, onError],
  );

  const openWithHistory = useCallback(
    (entry: EngineeringDocEntry, anchor?: string) => {
      setNavHistory(prev => {
        const truncated = prev.slice(0, navIndex + 1);
        const next = [...truncated, { entry, anchor }];
        setNavIndex(next.length - 1);
        return next;
      });
      void loadDocEntry(entry, anchor);
    },
    [loadDocEntry, navIndex],
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
        window.open(resolved.targetPath, '_blank', 'noopener,noreferrer');
        return;
      }
      if (selectedEngDoc && resolved.targetPath === selectedEngDoc.relative_path && resolved.anchor) {
        setActiveAnchor(resolved.anchor);
        return;
      }
      const catalogMatch = engineeringDocs.find(d => d.relative_path === resolved.targetPath);
      const targetEntry: EngineeringDocEntry = catalogMatch ?? {
        locale: resolved.targetPath.startsWith('src:') ? 'src' : 'root',
        file_name: resolved.targetPath.split('/').pop() ?? resolved.targetPath,
        relative_path: resolved.targetPath,
        title: resolved.targetPath.replace(/^src:/, '').replace(/^repo:/, ''),
        size_bytes: 0,
      };
      openWithHistory(targetEntry, resolved.anchor);
    },
    [engineeringDocs, openWithHistory, selectedEngDoc],
  );

  const goBackDoc = useCallback(() => {
    if (navIndex <= 0) return;
    const target = navHistory[navIndex - 1];
    setNavIndex(navIndex - 1);
    void loadDocEntry(target.entry, target.anchor);
  }, [loadDocEntry, navHistory, navIndex]);

  const goForwardDoc = useCallback(() => {
    if (navIndex < 0 || navIndex >= navHistory.length - 1) return;
    const target = navHistory[navIndex + 1];
    setNavIndex(navIndex + 1);
    void loadDocEntry(target.entry, target.anchor);
  }, [loadDocEntry, navHistory, navIndex]);

  useEffect(() => {
    let mounted = true;
    setIsLoading(true);
    invokeCommand<EngineeringDocEntry[]>('list_engineering_docs')
      .then(items => {
        if (!mounted) return;
        setEngineeringDocs(items);
        const preferred = items.find(d => d.relative_path === 'INDEX.md') ?? items[0];
        if (preferred) {
          setNavHistory([{ entry: preferred }]);
          setNavIndex(0);
          void loadDocEntry(preferred);
        }
      })
      .catch(err => {
        if (!mounted) return;
        onError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        if (mounted) setIsLoading(false);
      });
    return () => {
      mounted = false;
    };
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
    canGoBackDoc: navIndex > 0,
    canGoForwardDoc: navIndex >= 0 && navIndex < navHistory.length - 1,
    goBackDoc,
    goForwardDoc,
    selectEngineeringDoc,
    navigateToReference,
  };
}
