/**
 * @file store/runtimeSettingsBridge.ts
 * @brief Store-backed RuntimeSettings snapshot accessor for non-context ViewModels & unit tests.
 */

import type { RuntimeSettings } from '@/models';
import { store } from './index';

export function getRuntimeSettingsSnapshot(): RuntimeSettings {
  return store.getState().workbench.runtimeSettings;
}
