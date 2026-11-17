/**
 * @file viewmodels/useRuntimeSettingsViewModel.ts
 * @brief MVVM ViewModel managing canonical Rust-persisted RuntimeSettings draft, validation, and sync.
 */

import { useCallback, useEffect, useState } from 'react';
import { DEFAULT_RUNTIME_SETTINGS } from '@/constants';
import { useToast } from '@/context/ToastContext';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { translate } from '@/i18n/useTranslation';
import type {
  IrRuntimeSettings,
  RuntimeSettings,
  SerialRuntimeSettings,
  StorageRuntimeSettings,
  WorkbenchRuntimeSettings,
} from '@/models';
import {
  fetchRuntimeSettings,
  isRustSettingsBacked,
  persistRuntimeSettings,
  resetRuntimeSettingsOnBackend,
} from '@/services/settingsService';
import { runtimeSettingsSchema } from '@/utils/runtimeSettingsSchema';

function splitKeywords(rawText: string): readonly string[] {
  return rawText
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

export function useRuntimeSettingsViewModel() {
  const { showToast } = useToast();
  const { runtimeSettings, setRuntimeSettings } = useWorkbenchMode();
  const [draftOverrides, setDraftOverrides] = useState<RuntimeSettings | null>(null);
  const [keywordsOverride, setKeywordsOverride] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [saving, setSaving] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const draft = draftOverrides ?? runtimeSettings;
  const keywordsText = keywordsOverride ?? draft.storage.forensic_keywords.join(', ');

  const reloadFromBackend = useCallback(async () => {
    if (!isRustSettingsBacked()) return;
    setIsLoading(true);
    setError(null);
    try {
      const loaded = await fetchRuntimeSettings();
      setRuntimeSettings(loaded);
      setDraftOverrides(null);
      setKeywordsOverride(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [setRuntimeSettings]);

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        void reloadFromBackend();
      }
    });
    return () => controller.abort();
  }, [reloadFromBackend]);

  const updateSerialField = useCallback(
    <K extends keyof SerialRuntimeSettings>(key: K, value: SerialRuntimeSettings[K]) => {
      setSuccessMessage(null);
      setDraftOverrides((prev) => {
        const base = prev ?? runtimeSettings;
        return { ...base, serial: { ...base.serial, [key]: value } };
      });
    },
    [runtimeSettings],
  );

  const updateStorageField = useCallback(
    <K extends keyof StorageRuntimeSettings>(key: K, value: StorageRuntimeSettings[K]) => {
      setSuccessMessage(null);
      setDraftOverrides((prev) => {
        const base = prev ?? runtimeSettings;
        return { ...base, storage: { ...base.storage, [key]: value } };
      });
    },
    [runtimeSettings],
  );

  const updateIrField = useCallback(
    <K extends keyof IrRuntimeSettings>(key: K, value: IrRuntimeSettings[K]) => {
      setSuccessMessage(null);
      setDraftOverrides((prev) => {
        const base = prev ?? runtimeSettings;
        return { ...base, ir: { ...base.ir, [key]: value } };
      });
    },
    [runtimeSettings],
  );

  const updateWorkbenchField = useCallback(
    <K extends keyof WorkbenchRuntimeSettings>(key: K, value: WorkbenchRuntimeSettings[K]) => {
      setSuccessMessage(null);
      setDraftOverrides((prev) => {
        const base = prev ?? runtimeSettings;
        return { ...base, workbench: { ...base.workbench, [key]: value } };
      });
    },
    [runtimeSettings],
  );

  const updateKeywordsText = useCallback((nextText: string) => {
    setSuccessMessage(null);
    setKeywordsOverride(nextText);
  }, []);

  const saveRuntimeConfig = useCallback(async () => {
    const candidate: RuntimeSettings = {
      ...draft,
      storage: { ...draft.storage, forensic_keywords: splitKeywords(keywordsText) },
    };

    const parsed = runtimeSettingsSchema.safeParse(candidate);
    if (!parsed.success) {
      const msg = translate('settings.runtime.validationFailed');
      setError(msg);
      setSuccessMessage(null);
      showToast({
        type: 'error',
        title: translate('common.error'),
        message: msg,
      });
      return;
    }

    setSaving(true);
    setError(null);
    try {
      const saved = await persistRuntimeSettings(parsed.data);
      setRuntimeSettings(saved);
      setDraftOverrides(null);
      setKeywordsOverride(null);
      const okMsg = translate('settings.runtime.savedToastMsg');
      setSuccessMessage(okMsg);
      showToast({
        type: 'success',
        title: translate('settings.runtime.savedToastTitle'),
        message: okMsg,
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setSuccessMessage(null);
      showToast({
        type: 'error',
        title: translate('common.error'),
        message: msg,
      });
    } finally {
      setSaving(false);
    }
  }, [draft, keywordsText, setRuntimeSettings, showToast]);

  const resetRuntimeDefaults = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      const restored = isRustSettingsBacked()
        ? await resetRuntimeSettingsOnBackend()
        : DEFAULT_RUNTIME_SETTINGS;
      setRuntimeSettings(restored);
      setDraftOverrides(null);
      setKeywordsOverride(null);
      const resetMsg = translate('settings.runtime.resetToastMsg');
      setSuccessMessage(resetMsg);
      showToast({
        type: 'success',
        title: translate('settings.runtime.resetToastTitle'),
        message: resetMsg,
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
    } finally {
      setSaving(false);
    }
  }, [setRuntimeSettings, showToast]);

  const currentCandidate: RuntimeSettings = {
    ...draft,
    storage: { ...draft.storage, forensic_keywords: splitKeywords(keywordsText) },
  };

  return {
    runtimeSettings,
    draft,
    keywordsText,
    setKeywordsText: updateKeywordsText,
    isLoading,
    saving,
    error,
    successMessage,
    isDirty: JSON.stringify(currentCandidate) !== JSON.stringify(runtimeSettings),
    updateSerialField,
    updateStorageField,
    updateIrField,
    updateWorkbenchField,
    saveRuntimeConfig,
    resetRuntimeDefaults,
  };
}
