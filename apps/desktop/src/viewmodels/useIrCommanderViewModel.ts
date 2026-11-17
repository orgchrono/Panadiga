import { useCallback, useEffect, useReducer, useState } from 'react';
import irCommanderReducer, {
  initialIrCommanderState,
  irCommanderActions,
} from '@/store/slices/irCommanderSlice';
import { useToast } from '@/context/ToastContext';
import { useTranslation } from '@/i18n/useTranslation';
import {
  dispatchIrSignal,
  readAndroidIrStatus,
  SAFE_PICO_IR_GPIOS,
} from '@/services/irTransportService';
import type {
  AndroidIrCapabilities,
  DynamicRemoteKey,
  IrTimingOverride,
  IrTransportKind,
  PanasonicCategory,
  RemoteUnitSet,
} from '@/models/ir';
import {
  buildKeyTelemetryPayload,
  createBlankCustomKey,
  getErrorMessage,
  loadStoredIrSettings,
  persistIrSettings,
  removeProfileKey,
  replaceProfileKey,
  upsertSavedProfiles,
} from './ir/irStorage';
import { useIrCatalogDeck } from './ir/useIrCatalogDeck';
import { useIrMacroBeaconRunner } from './ir/useIrMacroBeaconRunner';

export function useIrCommanderViewModel() {
  const [state, dispatch] = useReducer(irCommanderReducer, initialIrCommanderState);
  const { showToast } = useToast();
  const { t, locale } = useTranslation();

  const [capabilities, setCapabilities] = useState<AndroidIrCapabilities | null>(() =>
    readAndroidIrStatus(),
  );
  const [editingKey, setEditingKey] = useState<DynamicRemoteKey | null>(null);

  useEffect(() => {
    const settings = loadStoredIrSettings();
    if (!settings) return;
    dispatch(irCommanderActions.setTimingUnitOverride(settings.timingUnitOverride));
    dispatch(irCommanderActions.setRepeatCount(settings.repeatCount));
    if (settings.savedProfiles.length > 0) {
      dispatch(irCommanderActions.setSavedProfiles(settings.savedProfiles));
    }
  }, []);

  const deck = useIrCatalogDeck({ dispatch, t, showToast });
  const runner = useIrMacroBeaconRunner({
    dispatch,
    remoteUnit: state.remoteUnit,
    transport: state.transport,
    picoGpioPin: state.picoGpioPin,
    timingUnitOverride: state.timingUnitOverride,
    repeatCount: state.repeatCount,
    isBeaconRunning: state.isBeaconRunning,
    locale,
    t,
    showToast,
  });

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
          irCommanderActions.setTransmissionTelemetry(
            buildKeyTelemetryPayload(key.label, state.repeatCount, outcome),
          ),
        );

        if (!outcome.ok) {
          const msg = outcome.error ?? t('irCommander.vfd.txFailed');
          dispatch(irCommanderActions.setError(msg));
          showToast({ type: 'error', title: key.label, message: msg });
        }
      } catch (err) {
        const msg = getErrorMessage(err);
        dispatch(irCommanderActions.setError(msg));
        showToast({ type: 'error', title: key.label, message: msg });
      } finally {
        dispatch(irCommanderActions.setTransmitting(false));
      }
    },
    [
      showToast,
      state.picoGpioPin,
      state.remoteUnit,
      state.repeatCount,
      state.timingUnitOverride,
      state.transport,
      t,
    ],
  );

  const saveEditedKey = useCallback(
    (updated: DynamicRemoteKey) => {
      if (state.activeProfile && state.activeProfile.keys.some(k => k.id === updated.id)) {
        dispatch(irCommanderActions.setActiveProfile(replaceProfileKey(state.activeProfile, updated)));
        return;
      }
      deck.updateActiveDeckKey(updated);
    },
    [deck, state.activeProfile],
  );

  const deleteEditedKey = useCallback(
    (keyId: string) => {
      if (state.activeProfile && state.activeProfile.keys.some(k => k.id === keyId)) {
        dispatch(irCommanderActions.setActiveProfile(removeProfileKey(state.activeProfile, keyId)));
      }
    },
    [state.activeProfile],
  );

  const addBlankKey = useCallback(() => {
    const base = state.activeProfile ?? deck.activeDeckProfile;
    if (!base) return;
    const newKey = createBlankCustomKey();
    dispatch(
      irCommanderActions.setActiveProfile({
        ...base,
        is_user_modified: true,
        keys: [...base.keys, newKey],
      }),
    );
    setEditingKey(newKey);
  }, [deck.activeDeckProfile, state.activeProfile]);

  const saveCurrentProfile = useCallback(() => {
    if (!state.activeProfile) return;
    const updated = upsertSavedProfiles(state.activeProfile, state.savedProfiles);
    dispatch(irCommanderActions.setSavedProfiles(updated));
    showToast({
      type: 'success',
      title: state.activeProfile.name,
      message: t('irCommander.catalog.savedToast'),
    });
  }, [showToast, state.activeProfile, state.savedProfiles, t]);

  const exportCurrentIrplus = useCallback(
    () => deck.exportProfileIrplus(state.activeProfile ?? deck.activeDeckProfile),
    [deck, state.activeProfile],
  );

  const changeTimingUnit = useCallback(
    (unit: IrTimingOverride) => {
      dispatch(irCommanderActions.setTimingUnitOverride(unit));
      persistIrSettings(unit, state.repeatCount);
    },
    [state.repeatCount],
  );

  const changeRepeatCount = useCallback(
    (repeat: number) => {
      dispatch(irCommanderActions.setRepeatCount(repeat));
      persistIrSettings(state.timingUnitOverride, repeat);
    },
    [state.timingUnitOverride],
  );

  return {
    state,
    capabilities,
    totalCodesets: deck.totalCodesets,
    totalAliases: deck.totalAliases,
    searchResults: deck.searchResults,
    isSearching: deck.isSearching,
    isLoadingDeck: deck.isLoadingDeck,
    activeDeckProfile: deck.activeDeckProfile,
    verifiedMacros: deck.verifiedMacros,
    exportedXml: deck.exportedXml,
    editingKey,
    safePicoGpios: SAFE_PICO_IR_GPIOS,
    setEditingKey,
    clearExportedXml: deck.clearExportedXml,
    selectMode: deck.selectMode,
    selectComboDeck: deck.selectComboDeck,
    selectRemoteUnit: (u: RemoteUnitSet) => dispatch(irCommanderActions.setRemoteUnit(u)),
    changeTransport: (tr: IrTransportKind) => dispatch(irCommanderActions.setTransport(tr)),
    changePicoGpio: (pin: number) => dispatch(irCommanderActions.setPicoGpioPin(pin)),
    changeTimingUnit,
    changeRepeatCount,
    toggleEditMode: (en: boolean) => dispatch(irCommanderActions.setEditMode(en)),
    setSearchQuery: (q: string) => {
      dispatch(irCommanderActions.setSearchQuery(q));
      void deck.refreshSearch(q, state.selectedCategory);
    },
    setSelectedCategory: (cat: PanasonicCategory) => {
      dispatch(irCommanderActions.setSelectedCategory(cat));
      void deck.refreshSearch(state.searchQuery, cat);
    },
    selectCodeset: deck.selectCodeset,
    selectSavedProfile: (id: string) => {
      const found = state.savedProfiles.find(p => p.id === id) ?? null;
      dispatch(irCommanderActions.setActiveProfile(found));
    },
    triggerKey,
    executeMacro: runner.executeMacro,
    abortMacro: runner.abortMacro,
    toggleCameraBeacon: runner.toggleCameraBeacon,
    saveEditedKey,
    deleteEditedKey,
    addBlankKey,
    saveCurrentProfile,
    importProfileText: deck.importProfileText,
    exportCurrentIrplus,
    refreshCapabilities: () => setCapabilities(readAndroidIrStatus()),
  };
}
