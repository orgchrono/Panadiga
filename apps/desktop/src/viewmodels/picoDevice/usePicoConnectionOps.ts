/**
 * @file viewmodels/picoDevice/usePicoConnectionOps.ts
 * @brief Hook managing Pico device connection toggling, telemetry polling, and cold break-in mode switching.
 */

import { useCallback, type Dispatch, type SetStateAction } from 'react';
import { translate } from '@/i18n/useTranslation';
import type { ColdBreakinMode, PicoDeviceStatus } from '@/models';
import { getPicoStatus, setPicoBreakinMode } from '@/services/picoService';
import { logger } from '@/utils/logger';
import {
  disconnectTransportPort,
  establishTransportConnection,
  sendDirectRawCommandForPort,
} from './picoConnectionActions';
import {
  buildMobileDeviceStatus,
  DEFAULT_DISCONNECTED_STATUS,
  isGdbBridgeRunning,
  parseVddFromStatusRaw,
  type UsePicoDeviceOptions,
} from './stateHelpers';

export interface UsePicoConnectionOpsParams {
  deviceStatus: PicoDeviceStatus | null;
  setDeviceStatus: Dispatch<SetStateAction<PicoDeviceStatus | null>>;
  setIsLoading: Dispatch<SetStateAction<boolean>>;
  selectedPort: string;
  isConnecting: boolean;
  setIsConnecting: Dispatch<SetStateAction<boolean>>;
  setManualDisconnect: Dispatch<SetStateAction<boolean>>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export function usePicoConnectionOps({
  deviceStatus,
  setDeviceStatus,
  setIsLoading,
  selectedPort,
  isConnecting,
  setIsConnecting,
  setManualDisconnect,
  onLog,
}: UsePicoConnectionOpsParams) {
  const pollDeviceStatus = useCallback(async () => {
    if (!selectedPort || isConnecting) return;
    if (await isGdbBridgeRunning()) return;
    if (selectedPort.startsWith('ANDROID-USB-OTG') || selectedPort.startsWith('BLE_')) {
      try {
        const raw = await sendDirectRawCommandForPort(selectedPort, 'STATUS');
        const vdd = parseVddFromStatusRaw(raw);
        setDeviceStatus((prev) => buildMobileDeviceStatus(prev, selectedPort, vdd));
      } catch (err) {
        logger.warn('PicoDevice', 'Failed mobile STATUS telemetry poll', err);
      }
      return;
    }

    setIsLoading(true);
    try {
      const status = await getPicoStatus(selectedPort);
      setDeviceStatus(status);
    } catch (err) {
      logger.warn('PicoDevice', 'Failed querying Pico device telemetry', err);
    } finally {
      setIsLoading(false);
    }
  }, [isConnecting, selectedPort, setDeviceStatus, setIsLoading]);

  const changeBreakinMode = useCallback(
    async (mode: ColdBreakinMode) => {
      try {
        if (selectedPort.startsWith('ANDROID-USB-OTG') || selectedPort.startsWith('BLE_')) {
          await sendDirectRawCommandForPort(selectedPort, `BREAK_IN ${mode}`);
          setDeviceStatus((prev) => (prev ? { ...prev, breakin_mode: mode } : prev));
          onLog?.('info', 'HOST', translate('errors.pico.modeChanged', { mode }));
          return;
        }
        const updated = await setPicoBreakinMode(selectedPort, mode);
        setDeviceStatus(updated);
        onLog?.('info', 'HOST', translate('errors.pico.modeChanged', { mode }));
      } catch (err) {
        onLog?.(
          'error',
          'HOST',
          translate('errors.pico.modeChangeFailed', { reason: String(err) }),
        );
      }
    },
    [selectedPort, setDeviceStatus, onLog],
  );

  const toggleConnection = useCallback(
    async (overridePort?: unknown) => {
      const targetPort =
        typeof overridePort === 'string' && overridePort.trim() ? overridePort : selectedPort;
      if (!targetPort) {
        onLog?.('warn', 'HOST', translate('errors.pico.noPortSelected'));
        return;
      }

      if (isConnecting) {
        onLog?.('warn', 'HOST', translate('errors.pico.connectionInProgress'));
        return;
      }

      if (deviceStatus?.connected) {
        setManualDisconnect(true);
        await disconnectTransportPort({
          targetPort,
          setDeviceStatus,
          onLog,
        });
        return;
      }

      if (await isGdbBridgeRunning()) {
        onLog?.('warn', 'HOST', translate('errors.pico.portBusyGdb', { port: targetPort }));
        return;
      }

      setManualDisconnect(false);
      setIsLoading(true);
      setIsConnecting(true);
      onLog?.('info', 'HOST', translate('errors.pico.connectingPort', { port: targetPort }));

      try {
        await establishTransportConnection({ targetPort, setDeviceStatus, onLog });
      } catch (err) {
        const errMsg = err instanceof Error ? err.message : String(err);
        onLog?.(
          'error',
          'HOST',
          translate('errors.pico.connectFailed', { port: targetPort, reason: errMsg }),
        );
        setDeviceStatus((prev) => ({
          ...(prev || DEFAULT_DISCONNECTED_STATUS),
          connected: false,
          port: targetPort,
          core_state: 'DISCONNECTED',
          vdd_volts: 0.0,
          target_powered: false,
        }));
      } finally {
        setIsLoading(false);
        setIsConnecting(false);
      }
    },
    [
      deviceStatus,
      isConnecting,
      onLog,
      selectedPort,
      setDeviceStatus,
      setIsConnecting,
      setIsLoading,
      setManualDisconnect,
    ],
  );

  return {
    pollDeviceStatus,
    changeBreakinMode,
    toggleConnection,
  };
}
