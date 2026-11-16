/**
 * @file viewmodels/useHardwareSsot.ts
 * @brief Redux-connected ViewModels for Rust Hardware SSOT and live shell telemetry.
 */

import { useCallback, useEffect } from 'react';
import type { LiveNvramCaptureEvent, PicoDeviceStatus } from '@/models';
import { listenEvent } from '@/services/ipc';
import { recordLiveNvramCapture } from '@/services/liveNvramCaptureStore';
import { store } from '@/store';
import { useAppDispatch, useAppSelector } from '@/store/hooks';
import {
  applyTelemetryStatus,
  fetchHardwareSsot,
  resetSsotState,
} from '@/store/slices/shellSlice';
import type { ShellTelemetryStatus } from '@/types/workbench';

export type { ShellTelemetryStatus };

export function resetHardwareSsotCacheForTests(): void {
  store.dispatch(resetSsotState());
}

/**
 * Top-level App shell ViewModel: listens for reactive Pico telemetry and live NVRAM capture events
 * and pushes updates into Redux so shell views stay decoupled from IPC internals.
 */
export function useShellTelemetryListener(): ShellTelemetryStatus {
  const dispatch = useAppDispatch();
  const hardwareStatus = useAppSelector(state => state.shell.hardwareStatus);

  useEffect(() => {
    let unlistenLegacy: (() => void) | undefined;
    let unlistenStream: (() => void) | undefined;
    let unlistenLiveNvram: (() => void) | undefined;

    const onTelemetry = (incoming: PicoDeviceStatus) => {
      if (incoming) {
        dispatch(applyTelemetryStatus(incoming));
      }
    };

    listenEvent<PicoDeviceStatus>('pico_telemetry', onTelemetry).then(fn => {
      unlistenLegacy = fn;
    });
    listenEvent<PicoDeviceStatus>('pico://telemetry', onTelemetry).then(fn => {
      unlistenStream = fn;
    });
    listenEvent<LiveNvramCaptureEvent>('pico://nvram-live-capture', evt => {
      if (evt?.report) {
        recordLiveNvramCapture(evt);
      }
    }).then(fn => {
      unlistenLiveNvram = fn;
    });

    return () => {
      unlistenLegacy?.();
      unlistenStream?.();
      unlistenLiveNvram?.();
    };
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
