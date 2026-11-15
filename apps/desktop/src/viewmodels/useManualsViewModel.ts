import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import {
  Am34BootLogReport,
  DocLocaleFilter,
  EngineeringDocEntry,
  ManualSearchResult,
  ManualDocumentMetadata,
  ServiceManualCatalogItem,
} from '@/models';
import { invokeCommand } from '@/services/ipc';
import { useTranslation, TranslationKey } from '@/i18n/useTranslation';
import { resolveDocReference } from '@/utils/docLinkResolver';

interface NavHistoryItem {
  entry: EngineeringDocEntry;
  anchor?: string;
}

function inferManualMetadata(item: ServiceManualCatalogItem, index: number): ManualDocumentMetadata {
  const lower = item.file_name.toLowerCase();
  const category: ManualDocumentMetadata['category'] = lower.includes('schematic')
    ? 'schematic'
    : lower.includes('operating') || lower.includes('user')
    ? 'operating_instructions'
    : lower.includes('quick') || lower.includes('guide') || lower.includes('bulletin')
    ? 'bulletin'
    : 'service_manual';

  const cleanTitle = item.file_name
    .replace(/\.pdf$/i, '')
    .replace(/_/g, ' ')
    .replace(/\b\w/g, c => c.toUpperCase());

  const keyIcs: string[] = [];
  if (lower.includes('eh55')) keyIcs.push('AM34', 'S29JL064H', 'P7501');
  if (lower.includes('mn103') || lower.includes('am3')) keyIcs.push('MN103S', 'AM34');
  if (lower.includes('schematic')) keyIcs.push('VEP79138', 'IC5001', 'Q7514');
  if (keyIcs.length === 0) keyIcs.push('PDF');

  return {
    id: String(index + 1),
    file_name: item.file_name,
    file_path: item.relative_path,
    title: cleanTitle,
    description: `${item.relative_path} (${item.formatted_size})`,
    category,
    size_bytes: item.size_bytes,
    page_count: 0,
    key_ics: keyIcs,
  };
}

export function useManualsViewModel() {
  const { t } = useTranslation();
  const [documents, setDocuments] = useState<ManualDocumentMetadata[]>([]);
  const [searchResults, setSearchResults] = useState<ManualSearchResult[]>([]);
  const [searchQuery, setSearchQuery] = useState<string>('U99');
  const [errorKey, setErrorKey] = useState<TranslationKey | null>(null);
  const [rawError, setRawError] = useState<string | null>(null);
  const [activeTerms] = useState<string[]>([
    'U99',
    'S29JL064',
    'MN103S',
    'P7501',
    'IC5001',
    'NSW5V',
    'HDD FORMAT',
    'SELF-CHECK',
    'RAM DISK',
    'VEP79138',
  ]);
  const [selectedDoc, setSelectedDoc] = useState<ManualDocumentMetadata | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [categoryFilter, setCategoryFilter] = useState<string>('all');

  const [engineeringDocs, setEngineeringDocs] = useState<EngineeringDocEntry[]>([]);
  const [docLocaleFilter, setDocLocaleFilter] = useState<DocLocaleFilter>('all');
  const [docSearchQuery, setDocSearchQuery] = useState<string>('');
  const [selectedEngDoc, setSelectedEngDoc] = useState<EngineeringDocEntry | null>(null);
  const [engDocContent, setEngDocContent] = useState<string>('');
  const [engDocLoading, setEngDocLoading] = useState<boolean>(false);
  const [activeAnchor, setActiveAnchor] = useState<string | null>(null);
  const [decodedLogReport, setDecodedLogReport] = useState<Am34BootLogReport | null>(null);
  const [navHistory, setNavHistory] = useState<NavHistoryItem[]>([]);
  const [navIndex, setNavIndex] = useState<number>(-1);

  const isMountedRef = useRef(true);
  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    let mounted = true;
    invokeCommand<ServiceManualCatalogItem[]>('list_service_manuals')
      .then(items => {
        if (!mounted) return;
        const mapped = items.map(inferManualMetadata);
        setDocuments(mapped);
        setSelectedDoc(prev => prev ?? mapped[0] ?? null);
      })
      .catch(err => {
        if (!mounted) return;
        setRawError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      mounted = false;
    };
  }, []);

  const executeSearch = useCallback(
    async (queryParam?: string) => {
      const q = (queryParam !== undefined ? queryParam : searchQuery).trim();
      if (!q) {
        if (!isMountedRef.current) return;
        setErrorKey('manuals.search.emptyQuery');
        setRawError(null);
        setSearchResults([]);
        return;
      }

      if (!isMountedRef.current) return;
      setErrorKey(null);
      setRawError(null);
      setLoading(true);
      try {
        const results = await invokeCommand<ManualSearchResult[]>('search_manuals', { query: q });
        if (isMountedRef.current) {
          setSearchResults(results);
        }
      } catch (err) {
        if (isMountedRef.current) {
          setRawError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (isMountedRef.current) {
          setLoading(false);
        }
      }
    },
    [searchQuery],
  );

  useEffect(() => {
    executeSearch('U99');
  }, [executeSearch]);

  const loadDocEntry = useCallback(async (entry: EngineeringDocEntry, anchor?: string) => {
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
        setRawError(err instanceof Error ? err.message : String(err));
      }
    } finally {
      if (isMountedRef.current) {
        setEngDocLoading(false);
      }
    }
  }, []);

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
        setRawError(err instanceof Error ? err.message : String(err));
      });
    return () => {
      mounted = false;
    };
  }, [loadDocEntry]);

  const filteredDocs = useMemo(() => {
    if (categoryFilter === 'all') return documents;
    return documents.filter(d => d.category === categoryFilter);
  }, [documents, categoryFilter]);

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

  const selectTerm = useCallback(
    (term: string) => {
      setSearchQuery(term);
      executeSearch(term);
    },
    [executeSearch],
  );

  const errorMessage = errorKey ? t(errorKey) : rawError;

  return {
    query: searchQuery,
    setQuery: setSearchQuery,
    documents: filteredDocs,
    allDocuments: documents,
    searchResults,
    searchQuery,
    setSearchQuery,
    activeTerms,
    selectedDoc,
    setSelectedDoc,
    loading,
    categoryFilter,
    setCategoryFilter,
    engineeringDocs: filteredEngineeringDocs,
    allEngineeringDocs: engineeringDocs,
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
    executeSearch,
    selectTerm,
    errorMessage,
  };
}
