/**
 * @file services/settingsService.ts
 * @brief Tauri IPC adapter for canonical Rust RuntimeSettings persistence and Pino->tracing telemetry bridge.
 */

import { DEFAULT_RUNTIME_SETTINGS } from '@/constants';
import { translate } from '@/i18n/useTranslation';
import type { RuntimeSettings } from '@/models';
import { logger } from '@/utils/logger';
import { parseRuntimeSettings, runtimeSettingsSchema } from '@/utils/runtimeSettingsSchema';
import { invokeCommand, isTauriEnvironment } from './ipc';

export async function fetchRuntimeSettings(): Promise<RuntimeSettings> {
  const raw = await invokeCommand<unknown>('get_runtime_settings');
  return parseRuntimeSettings(raw);
}

export async function persistRuntimeSettings(settings: RuntimeSettings): Promise<RuntimeSettings> {
  const parsed = runtimeSettingsSchema.safeParse(settings);
  if (!parsed.success) {
    throw new Error(translate('settings.runtime.validationFailed'));
  }
  const saved = await invokeCommand<unknown>('save_runtime_settings', {
    settings: parsed.data,
  });
  return parseRuntimeSettings(saved);
}

export async function resetRuntimeSettingsOnBackend(): Promise<RuntimeSettings> {
  const reset = await invokeCommand<unknown>('reset_runtime_settings');
  return parseRuntimeSettings(reset) ?? DEFAULT_RUNTIME_SETTINGS;
}

export function isRustSettingsBacked(): boolean {
  return isTauriEnvironment();
}

/**
 * Installs a lightweight telemetry forwarder pushing Pino UI log events into Rust `tracing` + OTel.
 */
export function installRustTelemetryBridge(): () => void {
  if (!isTauriEnvironment()) {
    return () => {};
  }
  return logger.addSink((entry) => {
    void invokeCommand('log_frontend_event', {
      severity: entry.severity,
      subsystem: entry.subsystem,
      message: entry.message,
      details: entry.details === undefined ? null : JSON.stringify(entry.details),
    }).catch(() => {
      // Telemetry transport errors must never disrupt UI state
    });
  });
}
