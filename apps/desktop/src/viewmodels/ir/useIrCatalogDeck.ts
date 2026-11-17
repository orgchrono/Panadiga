import { useCallback, useEffect, useMemo, useRef, useState, type Dispatch } from 'react';
import { irCommanderActions } from '@/store/slices/irCommanderSlice';
import {
  irAssembleCodeset,
  irCatalogStats,
  irExportIrplus,
  irGetVerifiedMacros,
  irImportProfile,
  irSearchCatalog,
} from '@/services/irIpc';
import type {
  DynamicKeymapProfile,
  DynamicRemoteKey,
  PanasonicCatalogSummary,
  PanasonicCategory,
  RemoteMode,
  VerifiedIrMacro,
} from '@/models/ir';
import {
  COMBO_DECK_CODESET_ID,
  DECK_CODESET_MAP,
  getErrorMessage,
  replaceProfileKey,
} from './irStorage';

type IrCommanderAction = ReturnType<
  (typeof irCommanderActions)[keyof typeof irCommanderActions]
>;

interface UseIrCatalogDeckParams {
  dispatch: Dispatch<IrCommanderAction>;
  t: (key: string, opts?: Record<string, unknown>) => string;
  showToast: (toast: {
    type: 'info' | 'success' | 'warning' | 'error';
    title: string;
    message: string;
  }) => void;
}

export function useIrCatalogDeck({ dispatch, t, showToast }: UseIrCatalogDeckParams) {
  const [totalCodesets, setTotalCodesets] = useState(0);
  const [totalAliases, setTotalAliases] = useState(0);
  const [searchResults, setSearchResults] = useState<PanasonicCatalogSummary[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [isLoadingDeck, setIsLoadingDeck] = useState(false);
  const [deckProfiles, setDeckProfiles] = useState<Record<string, DynamicKeymapProfile>>({});
  const [activeDeckCodesetId, setActiveDeckCodesetId] = useState<string>(DECK_CODESET_MAP.DVD);
  const [verifiedMacros, setVerifiedMacros] = useState<VerifiedIrMacro[]>([]);
  const [exportedXml, setExportedXml] = useState<string | null>(null);

  const deckCacheRef = useRef<Record<string, DynamicKeymapProfile>>({});
  const activeDeckCodesetRef = useRef<string>(DECK_CODESET_MAP.DVD);

  const loadCodesetIntoCache = useCallback(
    async (codesetId: string): Promise<DynamicKeymapProfile | null> => {
      const existing = deckCacheRef.current[codesetId];
      if (existing) return existing;
      setIsLoadingDeck(true);
      try {
        const res = await irAssembleCodeset(codesetId);
        deckCacheRef.current = { ...deckCacheRef.current, [codesetId]: res.profile };
        setDeckProfiles(deckCacheRef.current);
        return res.profile;
      } catch (err) {
        dispatch(irCommanderActions.setError(getErrorMessage(err)));
        return null;
      } finally {
        setIsLoadingDeck(false);
      }
    },
    [dispatch],
  );

  useEffect(() => {
    let active = true;
    void (async () => {
      try {
        const [stats, initialSearch, macros] = await Promise.all([
          irCatalogStats(),
          irSearchCatalog('', 'ALL', 60),
          irGetVerifiedMacros(),
        ]);
        if (!active) return;
        setTotalCodesets(stats.codeset_count);
        setTotalAliases(stats.command_count);
        setSearchResults(initialSearch);
        setVerifiedMacros(macros);
        await loadCodesetIntoCache(DECK_CODESET_MAP.DVD);
      } catch (err) {
        if (!active) return;
        dispatch(irCommanderActions.setError(getErrorMessage(err)));
      }
    })();

    return () => {
      active = false;
    };
  }, [dispatch, loadCodesetIntoCache]);

  const refreshSearch = useCallback(
    async (query: string, category: PanasonicCategory) => {
      setIsSearching(true);
      try {
        const hits = await irSearchCatalog(query, category, 60);
        setSearchResults(hits);
      } catch (err) {
        dispatch(irCommanderActions.setError(getErrorMessage(err)));
      } finally {
        setIsSearching(false);
      }
    },
    [dispatch],
  );

  const activateDeckCodeset = useCallback(
    async (codesetId: string) => {
      activeDeckCodesetRef.current = codesetId;
      setActiveDeckCodesetId(codesetId);
      await loadCodesetIntoCache(codesetId);
    },
    [loadCodesetIntoCache],
  );

  const selectMode = useCallback(
    async (mode: RemoteMode) => {
      dispatch(irCommanderActions.setMode(mode));
      if (mode !== 'CATALOG') {
        await activateDeckCodeset(DECK_CODESET_MAP[mode]);
      }
    },
    [activateDeckCodeset, dispatch],
  );

  const selectComboDeck = useCallback(async () => {
    dispatch(irCommanderActions.setMode('DVD'));
    await activateDeckCodeset(COMBO_DECK_CODESET_ID);
  }, [activateDeckCodeset, dispatch]);

  const activeDeckProfile = useMemo(
    () => deckProfiles[activeDeckCodesetId] ?? null,
    [activeDeckCodesetId, deckProfiles],
  );

  const selectCodeset = useCallback(
    async (codesetId: string) => {
      const profile = await loadCodesetIntoCache(codesetId);
      if (profile) {
        dispatch(irCommanderActions.setActiveProfile(profile));
      }
    },
    [dispatch, loadCodesetIntoCache],
  );

  const updateActiveDeckKey = useCallback((updated: DynamicRemoteKey) => {
    const deckId = activeDeckCodesetRef.current;
    const deck = deckCacheRef.current[deckId];
    if (!deck) return;
    const nextDeck = replaceProfileKey(deck, updated);
    deckCacheRef.current = { ...deckCacheRef.current, [deckId]: nextDeck };
    setDeckProfiles(deckCacheRef.current);
  }, []);

  const importProfileText = useCallback(
    async (format: 'irplus' | 'flipper', content: string, name: string) => {
      try {
        const prof = await irImportProfile(format, content, name);
        dispatch(irCommanderActions.setActiveProfile(prof));
        showToast({
          type: 'success',
          title: prof.name,
          message: t('irCommander.catalog.importedToast', { count: prof.keys.length }),
        });
      } catch (err) {
        dispatch(irCommanderActions.setError(getErrorMessage(err)));
      }
    },
    [dispatch, showToast, t],
  );

  const exportProfileIrplus = useCallback(async (profile: DynamicKeymapProfile | null) => {
    if (!profile) return;
    const xml = await irExportIrplus(profile);
    setExportedXml(xml);
  }, []);

  return {
    totalCodesets,
    totalAliases,
    searchResults,
    isSearching,
    isLoadingDeck,
    activeDeckProfile,
    verifiedMacros,
    exportedXml,
    clearExportedXml: useCallback(() => setExportedXml(null), []),
    refreshSearch,
    selectMode,
    selectComboDeck,
    selectCodeset,
    updateActiveDeckKey,
    importProfileText,
    exportProfileIrplus,
  };
}
