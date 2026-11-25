/**
 * @file viewmodels/useSsotStudioViewModel.ts
 * @brief MVVM closure hook managing DigaOS SSOT bundle loading, custom MMIO firewall probes, theme tuning & save/sync.
 */

import { useCallback, useState } from 'react';
import { useAsyncMountEffect } from '@/hooks/useAsyncMountEffect';
import {
  DIGAOS_CONFIG,
  DIGAOS_SSOT_CONTRACT_HASH_STR,
  type DigaOsConfigSpec,
} from '@/models/generated/digaos';
import {
  fetchSsotStudioBundle,
  rgb565ToCssHex,
  saveSsotCustomConfig,
  validateSsotCustomConfig,
  type SsotStudioBundle,
} from '@/services/ssotStudioService';

export { rgb565ToCssHex };

const DEFAULT_MMIO_PROBE = '0x8800002C';
const DEFAULT_CONTRAST_LEVEL = 82;
const MIN_CONTRAST = 1;
const MAX_CONTRAST = 100;

export interface SsotStudioViewModel {
  readonly config: DigaOsConfigSpec;
  readonly contractHashHex: string;
  readonly mmioInput: string;
  readonly setMmioInput: (val: string) => void;
  readonly contrastValue: string;
  readonly updateThemeContrast: (val: string) => void;
  readonly selectedMenuId: string;
  readonly setSelectedMenuId: (id: string) => void;
  readonly validationError: string | null;
  readonly validationStatus: string | null;
  readonly saveMessage: string | null;
  readonly isLoading: boolean;
  readonly busy: boolean;
  readonly validateMmioProbe: () => Promise<void>;
  readonly saveConfiguration: () => Promise<void>;
}

export function useSsotStudioViewModel(): SsotStudioViewModel {
  const [config, setConfig] = useState<DigaOsConfigSpec>(DIGAOS_CONFIG);
  const [contractHashHex, setContractHashHex] = useState<string>(
    `0x${DIGAOS_SSOT_CONTRACT_HASH_STR}`,
  );
  const [mmioInput, setMmioInput] = useState<string>(DEFAULT_MMIO_PROBE);
  const [contrastValue, setContrastValue] = useState<string>(
    String(DIGAOS_CONFIG.themes[0]?.contrast_level ?? DEFAULT_CONTRAST_LEVEL),
  );
  const [selectedMenuId, setSelectedMenuId] = useState<string>(
    DIGAOS_CONFIG.menus[0]?.id ?? 'core_diagnostics',
  );
  const [validationError, setValidationError] = useState<string | null>(null);
  const [validationStatus, setValidationStatus] = useState<string | null>(null);
  const [saveMessage, setSaveMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState<boolean>(false);

  const applyBundle = useCallback((bundle: SsotStudioBundle) => {
    if (!bundle?.config?.menus) {
      return;
    }
    setConfig(bundle.config);
    setContractHashHex(bundle.contractHashHex);
    setContrastValue(String(bundle.config.themes[0]?.contrast_level ?? DEFAULT_CONTRAST_LEVEL));
    if (bundle.config.menus[0]) {
      setSelectedMenuId(bundle.config.menus[0].id);
    }
  }, []);

  const isLoading = useAsyncMountEffect(fetchSsotStudioBundle, applyBundle);

  const buildConfigWithContrast = useCallback(
    (contrastRaw: string): DigaOsConfigSpec => {
      const parsed = Number.parseInt(contrastRaw, 10);
      const clamped = Number.isFinite(parsed)
        ? Math.min(MAX_CONTRAST, Math.max(MIN_CONTRAST, parsed))
        : (config.themes[0]?.contrast_level ?? DEFAULT_CONTRAST_LEVEL);
      const updatedThemes = config.themes.map((theme, idx) =>
        idx === 0 ? { ...theme, contrast_level: clamped } : theme,
      );
      return { ...config, themes: updatedThemes };
    },
    [config],
  );

  const updateThemeContrast = useCallback(
    (rawVal: string) => {
      setContrastValue(rawVal);
      setSaveMessage(null);
      const parsed = Number.parseInt(rawVal, 10);
      if (Number.isFinite(parsed) && parsed >= MIN_CONTRAST && parsed <= MAX_CONTRAST) {
        setConfig((prev) => ({
          ...prev,
          themes: prev.themes.map((theme, idx) =>
            idx === 0 ? { ...theme, contrast_level: parsed } : theme,
          ),
        }));
      }
    },
    [],
  );

  const validateMmioProbe = useCallback(async () => {
    const cleanAddr = mmioInput.trim();
    if (!cleanAddr) {
      setValidationError('Enter a target MMIO hex address before validating.');
      setValidationStatus(null);
      return;
    }
    setBusy(true);
    setValidationError(null);
    setValidationStatus(null);
    try {
      const baseCfg = buildConfigWithContrast(contrastValue);
      const candidate: DigaOsConfigSpec = {
        ...baseCfg,
        power_profiles: baseCfg.power_profiles.map((prof, idx) =>
          idx === 0
            ? {
                ...prof,
                allowed_mmio_writes: [
                  ...prof.allowed_mmio_writes,
                  { start: cleanAddr, size: 4, note: 'SSOT studio custom probe' },
                ],
              }
            : prof,
        ),
      };
      const result = await validateSsotCustomConfig(JSON.stringify(candidate, null, 2));
      if (!result.valid) {
        setValidationError(result.errors.join(' | ') || `BLOCKED: unsafe address ${cleanAddr}`);
        return;
      }
      setConfig(candidate);
      setContractHashHex(result.contractHashHex);
      setValidationStatus(
        `VALIDATED: ${cleanAddr} cleared SSOT safety firewall (DIGAOS_SSOT_CONTRACT_HASH ${result.contractHashHex}).`,
      );
    } catch (err) {
      setValidationError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }, [buildConfigWithContrast, contrastValue, mmioInput]);

  const saveConfiguration = useCallback(async () => {
    setBusy(true);
    setValidationError(null);
    setSaveMessage(null);
    try {
      const nextCfg = buildConfigWithContrast(contrastValue);
      const jsonPayload = JSON.stringify(nextCfg, null, 2);
      const res = await saveSsotCustomConfig(jsonPayload, contractHashHex);
      setConfig(nextCfg);
      setContractHashHex(res.contractHashHex);
      setSaveMessage(res.message);
    } catch (err) {
      setValidationError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }, [buildConfigWithContrast, contractHashHex, contrastValue]);

  return {
    config,
    contractHashHex,
    mmioInput,
    setMmioInput,
    contrastValue,
    updateThemeContrast,
    selectedMenuId,
    setSelectedMenuId,
    validationError,
    validationStatus,
    saveMessage,
    isLoading,
    busy,
    validateMmioProbe,
    saveConfiguration,
  };
}
