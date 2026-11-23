import { useState, useEffect, useCallback, useMemo } from 'react';
import type {
  ManualSearchResult,
  ManualDocumentMetadata,
  ServiceManualCatalogItem,
} from '@/models';
import { listServiceManuals, searchServiceManuals } from '@/services/systemService';
import { useTranslation, type TranslationKey } from '@/i18n/useTranslation';
import { useEngineeringDocsNav } from './useEngineeringDocsNav';

const DEFAULT_MANUAL_TERMS: readonly string[] = Object.freeze([
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

function inferManualCategory(lower: string): ManualDocumentMetadata['category'] {
  if (lower.includes('schematic')) return 'schematic';
  if (lower.includes('operating') || lower.includes('user')) return 'operating_instructions';
  if (lower.includes('quick') || lower.includes('guide') || lower.includes('bulletin')) {
    return 'bulletin';
  }
  return 'service_manual';
}

function inferManualKeyIcs(lower: string): string[] {
  const ics = [
    ...(lower.includes('eh55') ? ['AM34', 'S29JL064H', 'P7501'] : []),
    ...(lower.includes('mn103') || lower.includes('am3') ? ['MN103S', 'AM34'] : []),
    ...(lower.includes('schematic') ? ['VEP79138', 'IC5001', 'Q7514'] : []),
  ];
  return ics.length > 0 ? ics : ['PDF'];
}

function inferManualMetadata(item: ServiceManualCatalogItem, index: number): ManualDocumentMetadata {
  const lower = item.file_name.toLowerCase();
  const cleanTitle = item.file_name
    .replace(/\.pdf$/i, '')
    .replace(/_/g, ' ')
    .replace(/\b\w/g, c => c.toUpperCase());

  return {
    id: String(index + 1),
    file_name: item.file_name,
    file_path: item.relative_path,
    title: cleanTitle,
    description: `${item.relative_path} (${item.formatted_size})`,
    category: inferManualCategory(lower),
    size_bytes: item.size_bytes,
    page_count: 0,
    key_ics: inferManualKeyIcs(lower),
  };
}

export function useManualsViewModel() {
  const { t } = useTranslation();
  const [documents, setDocuments] = useState<ManualDocumentMetadata[]>([]);
  const [searchResults, setSearchResults] = useState<ManualSearchResult[]>([]);
  const [searchQuery, setSearchQuery] = useState<string>('U99');
  const [errorKey, setErrorKey] = useState<TranslationKey | null>(null);
  const [rawError, setRawError] = useState<string | null>(null);
  const [activeTerms] = useState<string[]>([...DEFAULT_MANUAL_TERMS]);
  const [selectedDoc, setSelectedDoc] = useState<ManualDocumentMetadata | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [categoryFilter, setCategoryFilter] = useState<string>('all');

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve()
      .then(() => listServiceManuals())
      .then(items => {
        if (controller.signal.aborted) return;
        const mapped = items.map(inferManualMetadata);
        setDocuments(mapped);
        setSelectedDoc(prev => prev ?? mapped[0] ?? null);
      })
      .catch(err => {
        if (!controller.signal.aborted) {
          setRawError(err instanceof Error ? err.message : String(err));
        }
      });
    return () => controller.abort();
  }, []);

  const performSearch = useCallback(async (q: string, signal?: AbortSignal) => {
    if (signal?.aborted) return;
    setLoading(true);
    try {
      const results = await searchServiceManuals(q);
      if (!signal?.aborted) {
        setSearchResults(results);
      }
    } catch (err) {
      if (!signal?.aborted) {
        setRawError(err instanceof Error ? err.message : String(err));
      }
    } finally {
      if (!signal?.aborted) {
        setLoading(false);
      }
    }
  }, []);

  const executeSearch = useCallback(
    async (queryParam?: string) => {
      const q = (queryParam !== undefined ? queryParam : searchQuery).trim();
      if (!q) {
        setErrorKey('manuals.search.emptyQuery');
        setRawError(null);
        setSearchResults([]);
        return;
      }
      setErrorKey(null);
      setRawError(null);
      await performSearch(q);
    },
    [performSearch, searchQuery],
  );

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        void performSearch('U99', controller.signal);
      }
    });
    return () => controller.abort();
  }, [performSearch]);

  const docsNav = useEngineeringDocsNav(setRawError);

  const filteredDocs = useMemo(() => {
    if (categoryFilter === 'all') return documents;
    return documents.filter(d => d.category === categoryFilter);
  }, [documents, categoryFilter]);

  const selectTerm = useCallback(
    (term: string) => {
      setSearchQuery(term);
      void executeSearch(term);
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
    ...docsNav,
    executeSearch,
    selectTerm,
    errorMessage,
  };
}
