/**
 * @file viewmodels/useHardwareSsot.ts
 * @brief Redux-connected ViewModels for Rust Hardware SSOT and live shell telemetry.
 */

import { useCallback, useEffect } from 'react';
import type { PicoDeviceStatus } from '@/models';
import {
  subscribePicoNvramLiveCapture,
  subscribePicoTelemetry,
} from '@/services/picoService';
import { useAppDispatch, useAppSelector } from '@/store/hooks';
import { recordLiveNvramCapture } from '@/store/nvramCaptureBridge';
import {
  applyTelemetryStatus,
  fetchHardwareSsot,
  resetSsotState,
} from '@/store/slices/shellSlice';
import type { ShellTelemetryStatus } from '@/types/workbench';
import { registerAsyncSubscriptions } from '@/utils/subscription';

export type { ShellTelemetryStatus };

export function resetHardwareSsotCacheForTests(): void {
  storeReset();
}

import { store } from '@/store';
function storeReset(): void {
  store.dispatch(resetSsotState());
}

/**
 * React hook subscribing global shell status bar to P7501 hardware telemetry & NVRAM captures.
 */
export function useShellTelemetryListener(): ShellTelemetryStatus {
  const dispatch = useAppDispatch();
  const hardwareStatus = useAppSelector(state => state.shell.hardwareStatus);

  useEffect(() => {
    const onTelemetry = (incoming: PicoDeviceStatus) => {
      if (incoming) {
        dispatch(applyTelemetryStatus(incoming));
      }
    };

    return registerAsyncSubscriptions([
      () => subscribePicoTelemetry(onTelemetry),
      () =>
        subscribePicoNvramLiveCapture(evt => {
          if (evt?.report) {
            recordLiveNvramCapture(evt);
          }
        }),
    ]);
  }, [dispatch]);

  return hardwareStatus;
}

/**
 * Single Source of Truth ViewModel: fetches the canonical hardware, pinout, memory map,
 * thresholds, and Pico CLI command catalog from Rust backend into Redux state.
 */
export function useHardwareSsot() {
  const dispatch = useAppDispatch();
  const ssot = useAppSelector(state => state.shell.ssot);
  const ssotLoaded = useAppSelector(state => state.shell.ssotLoaded);
  const ssotLoading = useAppSelector(state => state.shell.ssotLoading);
  const error = useAppSelector(state => state.shell.ssotError);

  useEffect(() => {
    if (!ssotLoaded && !ssotLoading && error === null) {
      void dispatch(fetchHardwareSsot(false));
    }
  }, [dispatch, ssotLoaded, ssotLoading, error]);

  const reload = useCallback(() => {
    void dispatch(fetchHardwareSsot(true));
  }, [dispatch]);

  return {
    ssot,
    isLoaded: ssotLoaded && ssot !== null && error === null,
    isLoading: ssotLoading || (!ssotLoaded && error === null),
    error,
    reload,
  };
}
