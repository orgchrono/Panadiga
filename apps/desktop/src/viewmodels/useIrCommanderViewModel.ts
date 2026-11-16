import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react';
import irCommanderReducer, {
  initialIrCommanderState,
  irCommanderActions,
} from '@/store/slices/irCommanderSlice';
import { useToast } from '@/context/ToastContext';
import { useTranslation } from '@/i18n/useTranslation';
import {
  getAndroidIrSettings,
  saveAndroidCustomProfilesJson,
  saveAndroidIrSettings,
} from '@/services/androidIrBridge';
import {
  irAssembleCodeset,
  irCatalogStats,
  irExportIrplus,
  irGetVerifiedMacros,
  irImportProfile,
  irSearchCatalog,
} from '@/services/irIpc';
import {
  dispatchIrSignal,
  readAndroidIrStatus,
  SAFE_PICO_IR_GPIOS,
} from '@/services/irTransportService';
import type {
  AndroidIrCapabilities,
  DynamicKeymapProfile,
  DynamicRemoteKey,
  IrTimingOverride,
  IrTransportKind,
  PanasonicCatalogSummary,
  PanasonicCategory,
  RemoteMode,
  RemoteUnitSet,
  VerifiedIrMacro,
} from '@/models/ir';

const DECK_CODESET_MAP: Record<Exclude<RemoteMode, 'CATALOG'>, string> = {
  DVD: 'curated_diga_eh55_eur7659',
  VCR: 'curated_panasonic_vcr_144',
  TV: 'curated_viera_tv_128',
};

function sleepAbortable(ms: number, signal: AbortSignal): Promise<boolean> {
  if (ms <= 0) return Promise.resolve(!signal.aborted);
  return new Promise(resolve => {
    if (signal.aborted) {
      resolve(false);
      return;
    }
    const timer = setTimeout(() => {
      signal.removeEventListener('abort', onAbort);
      resolve(true);
    }, ms);
    const onAbort = () => {
      clearTimeout(timer);
      resolve(false);
    };
    signal.addEventListener('abort', onAbort, { once: true });
  });
}

export function useIrCommanderViewModel() {
  const [state, dispatch] = useReducer(irCommanderReducer, initialIrCommanderState);
  const { showToast } = useToast();
  const { t, locale } = useTranslation();

  const [capabilities, setCapabilities] = useState<AndroidIrCapabilities | null>(() =>
    readAndroidIrStatus(),
  );
  const [totalCodesets, setTotalCodesets] = useState(0);
  const [totalAliases, setTotalAliases] = useState(0);
  const [searchResults, setSearchResults] = useState<PanasonicCatalogSummary[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [isLoadingDeck, setIsLoadingDeck] = useState(false);
  const [deckProfiles, setDeckProfiles] = useState<Record<string, DynamicKeymapProfile>>({});
  const [verifiedMacros, setVerifiedMacros] = useState<VerifiedIrMacro[]>([]);
  const [exportedXml, setExportedXml] = useState<string | null>(null);
  const [editingKey, setEditingKey] = useState<DynamicRemoteKey | null>(null);

  const macroAbortRef = useRef<AbortController | null>(null);
  const beaconAbortRef = useRef<AbortController | null>(null);
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
        dispatch(
          irCommanderActions.setError(err instanceof Error ? err.message : String(err)),
        );
        return null;
      } finally {
        setIsLoadingDeck(false);
      }
    },
    [dispatch],
  );

  useEffect(() => {
    let active = true;
    const settings = getAndroidIrSettings();
    if (settings) {
      dispatch(irCommanderActions.setTimingUnitOverride(settings.timingUnitOverride));
      dispatch(irCommanderActions.setRepeatCount(settings.repeatCount));
      try {
        const parsed = JSON.parse(settings.customProfilesJson) as DynamicKeymapProfile[];
        if (Array.isArray(parsed)) {
          dispatch(irCommanderActions.setSavedProfiles(parsed));
        }
      } catch {
        // Ignore empty or corrupt stored custom profiles
      }
    }

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
        dispatch(
          irCommanderActions.setError(err instanceof Error ? err.message : String(err)),
        );
      }
    })();

    return () => {
      active = false;
      macroAbortRef.current?.abort();
      beaconAbortRef.current?.abort();
    };
  }, [dispatch, loadCodesetIntoCache]);

  const refreshSearch = useCallback(
    async (query: string, category: PanasonicCategory) => {
      setIsSearching(true);
      try {
        const hits = await irSearchCatalog(query, category, 60);
        setSearchResults(hits);
      } catch (err) {
        dispatch(
          irCommanderActions.setError(err instanceof Error ? err.message : String(err)),
        );
      } finally {
        setIsSearching(false);
      }
    },
    [dispatch],
  );

  const selectMode = useCallback(
    async (mode: RemoteMode) => {
      dispatch(irCommanderActions.setMode(mode));
      if (mode !== 'CATALOG') {
        const codesetId = DECK_CODESET_MAP[mode];
        activeDeckCodesetRef.current = codesetId;
        await loadCodesetIntoCache(codesetId);
      }
    },
    [dispatch, loadCodesetIntoCache],
  );

  const selectComboDeck = useCallback(async () => {
    dispatch(irCommanderActions.setMode('DVD'));
    activeDeckCodesetRef.current = 'curated_diga_combo_vhs_dvd';
    await loadCodesetIntoCache('curated_diga_combo_vhs_dvd');
  }, [dispatch, loadCodesetIntoCache]);

  const activeDeckProfile = useMemo(
    () => deckProfiles[activeDeckCodesetRef.current] ?? null,
    // eslint-disable-next-line react-hooks/exhaustive-deps -- activeDeckCodesetRef updates alongside mode/deckProfiles
    [deckProfiles, state.mode],
  );

  const triggerKey = useCallback(
    async (key: DynamicRemoteKey) => {
      dispatch(irCommanderActions.setTransmitting(true));
      try {
        const outcome = await dispatchIrSignal({
          protocol: key.protocol,
          device: key.device,
          subdevice: key.subdevice,
          obc: key.obc,
          respectRemoteMode: key.respect_remote_mode,
          remoteUnit: state.remoteUnit,
          prontoHex: key.pronto_hex,
          transport: state.transport,
          picoGpioPin: state.picoGpioPin,
          unitOverride: state.timingUnitOverride,
          repeatCount: state.repeatCount,
        });

        dispatch(
          irCommanderActions.setTransmissionTelemetry({
            primary: `TX: ${key.label.toUpperCase()}`,
            secondary: `${outcome.prepared.carrier_hz} Hz · ${outcome.prepared.effective_unit} · x${state.repeatCount}`,
            wireHex: outcome.prepared.wire_hex,
            prontoHex: outcome.prepared.pronto_hex,
            backendUsed: outcome.backendUsed,
          }),
        );

        if (!outcome.ok) {
          const msg = outcome.error ?? t('irCommander.vfd.txFailed');
          dispatch(irCommanderActions.setError(msg));
          showToast({ type: 'error', title: key.label, message: msg });
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        dispatch(irCommanderActions.setError(msg));
        showToast({ type: 'error', title: key.label, message: msg });
      } finally {
        dispatch(irCommanderActions.setTransmitting(false));
      }
    },
    [
      dispatch,
      showToast,
      state.picoGpioPin,
      state.remoteUnit,
      state.repeatCount,
      state.timingUnitOverride,
      state.transport,
      t,
    ],
  );

  const executeMacro = useCallback(
    async (macro: VerifiedIrMacro) => {
      macroAbortRef.current?.abort();
      const ctrl = new AbortController();
      macroAbortRef.current = ctrl;
      const total = macro.steps.length;
      const title = locale === 'pt-BR' ? macro.title_pt : macro.title_en;

      dispatch(irCommanderActions.setTransmitting(true));
      try {
        for (let i = 0; i < total; i += 1) {
          if (ctrl.signal.aborted) break;
          const step = macro.steps[i];
          dispatch(irCommanderActions.setMacroProgress({ id: macro.id, step: i + 1, total }));
          const outcome = await dispatchIrSignal({
            protocol: 'KASEIKYO',
            device: step.device,
            subdevice: step.subdevice,
            obc: step.obc,
            respectRemoteMode: step.device === 0xb0,
            remoteUnit: state.remoteUnit,
            transport: state.transport,
            picoGpioPin: state.picoGpioPin,
            unitOverride: state.timingUnitOverride,
            repeatCount: Math.max(state.repeatCount, step.hold_ms > 1000 ? 2 : 1),
          });
          dispatch(
            irCommanderActions.setTransmissionTelemetry({
              primary: `${title} [${i + 1}/${total}]`,
              secondary: `${step.label} · delay ${step.post_delay_ms}ms`,
              wireHex: outcome.prepared.wire_hex,
              prontoHex: outcome.prepared.pronto_hex,
              backendUsed: outcome.backendUsed,
            }),
          );
          if (!outcome.ok) {
            throw new Error(outcome.error ?? `Macro step ${i + 1} (${step.label}) failed`);
          }
          const continued = await sleepAbortable(step.hold_ms + step.post_delay_ms, ctrl.signal);
          if (!continued) break;
        }
        if (ctrl.signal.aborted) {
          showToast({ type: 'warning', title, message: t('irCommander.macro.abortedToast') });
        } else {
          showToast({ type: 'success', title, message: t('irCommander.macro.completedToast') });
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        dispatch(irCommanderActions.setError(msg));
        showToast({ type: 'error', title, message: msg });
      } finally {
        dispatch(irCommanderActions.setMacroProgress({ id: null, step: 0, total: 0 }));
        dispatch(irCommanderActions.setTransmitting(false));
      }
    },
    [
      dispatch,
      locale,
      showToast,
      state.picoGpioPin,
      state.remoteUnit,
      state.repeatCount,
      state.timingUnitOverride,
      state.transport,
      t,
    ],
  );

  const abortMacro = useCallback(() => {
    macroAbortRef.current?.abort();
  }, []);

  const toggleCameraBeacon = useCallback(async () => {
    if (state.isBeaconRunning) {
      beaconAbortRef.current?.abort();
      return;
    }
    const ctrl = new AbortController();
    beaconAbortRef.current = ctrl;
    dispatch(irCommanderActions.setBeaconRunning(true));
    try {
      for (let pulse = 1; pulse <= 30; pulse += 1) {
        if (ctrl.signal.aborted) break;
        await dispatchIrSignal({
          protocol: 'KASEIKYO',
          device: 0xb0,
          subdevice: 0x00,
          obc: 0xfe,
          respectRemoteMode: true,
          remoteUnit: state.remoteUnit,
          transport: state.transport,
          picoGpioPin: state.picoGpioPin,
          unitOverride: state.timingUnitOverride,
          repeatCount: 1,
        });
        const ok = await sleepAbortable(250, ctrl.signal);
        if (!ok) break;
      }
      showToast({
        type: 'info',
        title: t('irCommander.diag.beaconTitle'),
        message: t('irCommander.diag.beaconDone'),
      });
    } finally {
      dispatch(irCommanderActions.setBeaconRunning(false));
    }
  }, [
    dispatch,
    showToast,
    state.isBeaconRunning,
    state.picoGpioPin,
    state.remoteUnit,
    state.timingUnitOverride,
    state.transport,
    t,
  ]);

  const selectCodeset = useCallback(
    async (codesetId: string) => {
      const profile = await loadCodesetIntoCache(codesetId);
      if (profile) {
        dispatch(irCommanderActions.setActiveProfile(profile));
      }
    },
    [dispatch, loadCodesetIntoCache],
  );

  const saveEditedKey = useCallback(
    (updated: DynamicRemoteKey) => {
      if (state.activeProfile && state.activeProfile.keys.some(k => k.id === updated.id)) {
        const nextProfile: DynamicKeymapProfile = {
          ...state.activeProfile,
          is_user_modified: true,
          keys: state.activeProfile.keys.map(k => (k.id === updated.id ? updated : k)),
        };
        dispatch(irCommanderActions.setActiveProfile(nextProfile));
        return;
      }
      const deckId = activeDeckCodesetRef.current;
      const deck = deckCacheRef.current[deckId];
      if (deck) {
        const nextDeck: DynamicKeymapProfile = {
          ...deck,
          is_user_modified: true,
          keys: deck.keys.map(k => (k.id === updated.id ? updated : k)),
        };
        deckCacheRef.current = { ...deckCacheRef.current, [deckId]: nextDeck };
        setDeckProfiles(deckCacheRef.current);
      }
    },
    [dispatch, state.activeProfile],
  );

  const deleteEditedKey = useCallback(
    (keyId: string) => {
      if (state.activeProfile && state.activeProfile.keys.some(k => k.id === keyId)) {
        dispatch(
          irCommanderActions.setActiveProfile({
            ...state.activeProfile,
            is_user_modified: true,
            keys: state.activeProfile.keys.filter(k => k.id !== keyId),
          }),
        );
      }
    },
    [dispatch, state.activeProfile],
  );

  const addBlankKey = useCallback(() => {
    const base = state.activeProfile ?? activeDeckProfile;
    if (!base) return;
    const newKey: DynamicRemoteKey = {
      id: `custom_${Date.now()}`,
      label: 'Custom Key',
      section: 'CUSTOM',
      protocol: 'KASEIKYO',
      device: 0xb0,
      subdevice: 0x00,
      obc: 0x92,
      respect_remote_mode: true,
      pronto_hex: null,
      tone: 'NEUTRAL',
      confirm_required: false,
      wire_preview: '02 20 B0 00 92 22',
    };
    dispatch(
      irCommanderActions.setActiveProfile({
        ...base,
        is_user_modified: true,
        keys: [...base.keys, newKey],
      }),
    );
    setEditingKey(newKey);
  }, [activeDeckProfile, dispatch, state.activeProfile]);

  const saveCurrentProfile = useCallback(() => {
    if (!state.activeProfile) return;
    const others = state.savedProfiles.filter(p => p.id !== state.activeProfile?.id);
    const updated = [state.activeProfile, ...others];
    dispatch(irCommanderActions.setSavedProfiles(updated));
    saveAndroidCustomProfilesJson(JSON.stringify(updated));
    showToast({
      type: 'success',
      title: state.activeProfile.name,
      message: t('irCommander.catalog.savedToast'),
    });
  }, [dispatch, showToast, state.activeProfile, state.savedProfiles, t]);

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
        dispatch(
          irCommanderActions.setError(err instanceof Error ? err.message : String(err)),
        );
      }
    },
    [dispatch, showToast, t],
  );

  const exportCurrentIrplus = useCallback(async () => {
    const prof = state.activeProfile ?? activeDeckProfile;
    if (!prof) return;
    const xml = await irExportIrplus(prof);
    setExportedXml(xml);
  }, [activeDeckProfile, state.activeProfile]);

  const changeTimingUnit = useCallback(
    (unit: IrTimingOverride) => {
      dispatch(irCommanderActions.setTimingUnitOverride(unit));
      saveAndroidIrSettings(unit, state.repeatCount);
    },
    [dispatch, state.repeatCount],
  );

  const changeRepeatCount = useCallback(
    (repeat: number) => {
      dispatch(irCommanderActions.setRepeatCount(repeat));
      saveAndroidIrSettings(state.timingUnitOverride, repeat);
    },
    [dispatch, state.timingUnitOverride],
  );

  return {
    state,
    capabilities,
    totalCodesets,
    totalAliases,
    searchResults,
    isSearching,
    isLoadingDeck,
    activeDeckProfile,
    verifiedMacros,
    exportedXml,
    editingKey,
    safePicoGpios: SAFE_PICO_IR_GPIOS,
    setEditingKey,
    clearExportedXml: () => setExportedXml(null),
    selectMode,
    selectComboDeck,
    selectRemoteUnit: (u: RemoteUnitSet) => dispatch(irCommanderActions.setRemoteUnit(u)),
    changeTransport: (tr: IrTransportKind) => dispatch(irCommanderActions.setTransport(tr)),
    changePicoGpio: (pin: number) => dispatch(irCommanderActions.setPicoGpioPin(pin)),
    changeTimingUnit,
    changeRepeatCount,
    toggleEditMode: (en: boolean) => dispatch(irCommanderActions.setEditMode(en)),
    setSearchQuery: (q: string) => {
      dispatch(irCommanderActions.setSearchQuery(q));
      void refreshSearch(q, state.selectedCategory);
    },
    setSelectedCategory: (cat: PanasonicCategory) => {
      dispatch(irCommanderActions.setSelectedCategory(cat));
      void refreshSearch(state.searchQuery, cat);
    },
    selectCodeset,
    selectSavedProfile: (id: string) => {
      const found = state.savedProfiles.find(p => p.id === id) ?? null;
      dispatch(irCommanderActions.setActiveProfile(found));
    },
    triggerKey,
    executeMacro,
    abortMacro,
    toggleCameraBeacon,
    saveEditedKey,
    deleteEditedKey,
    addBlankKey,
    saveCurrentProfile,
    importProfileText,
    exportCurrentIrplus,
    refreshCapabilities: () => setCapabilities(readAndroidIrStatus()),
  };
}
