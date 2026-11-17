/**
 * @file viewmodels/useRuntimeSettingsViewModel.ts
 * @brief MVVM ViewModel managing canonical Rust-persisted RuntimeSettings draft, validation, and sync.
 */

import { useCallback, useEffect, useState } from 'react';
import { DEFAULT_RUNTIME_SETTINGS } from '@/constants';
import { useToast } from '@/context/ToastContext';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { useAbortSlot } from '@/hooks/useAbortSlot';
import type { TranslationKey } from '@/i18n/translations';
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
import { installDesktopShortcut } from '@/services/systemService';
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
  const [installingShortcut, setInstallingShortcut] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const { acquire: acquireReload, release: releaseReload } = useAbortSlot();
  const { acquire: acquirePersist, release: releasePersist } = useAbortSlot();

  const draft = draftOverrides ?? runtimeSettings;
  const keywordsText = keywordsOverride ?? draft.storage.forensic_keywords.join(', ');

  const reloadFromBackend = useCallback(async () => {
    if (!isRustSettingsBacked()) return;
    const ctrl = acquireReload();
    setIsLoading(true);
    setError(null);
    try {
      const loaded = await fetchRuntimeSettings();
      if (ctrl.signal.aborted) return;
      setRuntimeSettings(loaded);
      setDraftOverrides(null);
      setKeywordsOverride(null);
    } catch (err) {
      if (ctrl.signal.aborted) return;
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      releaseReload(ctrl);
      if (!ctrl.signal.aborted) setIsLoading(false);
    }
  }, [acquireReload, releaseReload, setRuntimeSettings]);

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

  const applyPersistedRuntimeSettings = useCallback(
    (nextSettings: RuntimeSettings, titleKey: TranslationKey, msgKey: TranslationKey) => {
      setRuntimeSettings(nextSettings);
      setDraftOverrides(null);
      setKeywordsOverride(null);
      const okMsg = translate(msgKey);
      setSuccessMessage(okMsg);
      showToast({
        type: 'success',
        title: translate(titleKey),
        message: okMsg,
      });
    },
    [setRuntimeSettings, showToast],
  );

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

    const ctrl = acquirePersist();
    setSaving(true);
    setError(null);
    try {
      const saved = await persistRuntimeSettings(parsed.data);
      if (ctrl.signal.aborted) return;
      applyPersistedRuntimeSettings(
        saved,
        'settings.runtime.savedToastTitle',
        'settings.runtime.savedToastMsg',
      );
    } catch (err) {
      if (ctrl.signal.aborted) return;
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setSuccessMessage(null);
      showToast({
        type: 'error',
        title: translate('common.error'),
        message: msg,
      });
    } finally {
      releasePersist(ctrl);
      if (!ctrl.signal.aborted) setSaving(false);
    }
  }, [acquirePersist, applyPersistedRuntimeSettings, draft, keywordsText, releasePersist, showToast]);

  const resetRuntimeDefaults = useCallback(async () => {
    const ctrl = acquirePersist();
    setSaving(true);
    setError(null);
    try {
      const restored = isRustSettingsBacked()
        ? await resetRuntimeSettingsOnBackend()
        : DEFAULT_RUNTIME_SETTINGS;
      if (ctrl.signal.aborted) return;
      applyPersistedRuntimeSettings(
        restored,
        'settings.runtime.resetToastTitle',
        'settings.runtime.resetToastMsg',
      );
    } catch (err) {
      if (ctrl.signal.aborted) return;
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
    } finally {
      releasePersist(ctrl);
      if (!ctrl.signal.aborted) setSaving(false);
    }
  }, [acquirePersist, applyPersistedRuntimeSettings, releasePersist]);

  const installSystemLauncher = useCallback(async () => {
    setInstallingShortcut(true);
    try {
      const report = await installDesktopShortcut(true);
      showToast({
        type: 'success',
        title: translate('settings.workflow.shortcutCreatedTitle'),
        message: report.message,
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: translate('settings.workflow.shortcutErrorTitle'),
        message: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setInstallingShortcut(false);
    }
  }, [showToast]);

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
    installingShortcut,
    error,
    successMessage,
    isDirty: JSON.stringify(currentCandidate) !== JSON.stringify(runtimeSettings),
    updateSerialField,
    updateStorageField,
    updateIrField,
    updateWorkbenchField,
    saveRuntimeConfig,
    resetRuntimeDefaults,
    installSystemLauncher,
  };
}
