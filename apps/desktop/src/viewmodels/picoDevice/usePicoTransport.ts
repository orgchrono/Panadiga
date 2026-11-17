/**
 * @file viewmodels/picoDevice/usePicoTransport.ts
 * @brief Transport hook facade composing port discovery and connection lifecycle operations cleanly without refs.
 */

import { useEffect, useState, type Dispatch, type SetStateAction } from 'react';
import { translate } from '@/i18n/useTranslation';
import type { DigaModelProfile, PicoDeviceStatus } from '@/models';
import {
  pickPreferredSerialPort,
  PICO_PORT_PATTERN,
  type UsePicoDeviceOptions,
} from './stateHelpers';
import { usePicoConnectionOps } from './usePicoConnectionOps';
import { usePicoPortDiscovery } from './usePicoPortDiscovery';

interface UsePicoTransportParams {
  deviceStatus: PicoDeviceStatus | null;
  setDeviceStatus: Dispatch<SetStateAction<PicoDeviceStatus | null>>;
  isLoading: boolean;
  setIsLoading: Dispatch<SetStateAction<boolean>>;
  availablePorts: string[];
  setAvailablePorts: Dispatch<SetStateAction<string[]>>;
  selectedPort: string;
  setSelectedPort: Dispatch<SetStateAction<string>>;
  modelCatalog: DigaModelProfile[];
  setModelCatalog: Dispatch<SetStateAction<DigaModelProfile[]>>;
  setSelectedModel: Dispatch<SetStateAction<string>>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export function usePicoTransport({
  deviceStatus,
  setDeviceStatus,
  isLoading,
  setIsLoading,
  availablePorts,
  setAvailablePorts,
  selectedPort,
  setSelectedPort,
  modelCatalog,
  setModelCatalog,
  setSelectedModel,
  onLog,
}: UsePicoTransportParams) {
  const [isConnecting, setIsConnecting] = useState<boolean>(false);
  const [manualDisconnect, setManualDisconnect] = useState<boolean>(false);
  const [autoAttemptedPort, setAutoAttemptedPort] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  const { pollDeviceStatus, changeBreakinMode, toggleConnection } = usePicoConnectionOps({
    deviceStatus,
    setDeviceStatus,
    setIsLoading,
    selectedPort,
    isConnecting,
    setIsConnecting,
    setManualDisconnect,
    onLog,
  });

  const { refreshModelCatalog, selectTargetModel, refreshPorts } = usePicoPortDiscovery({
    selectedPort,
    setSelectedPort,
    setAvailablePorts,
    modelCatalog,
    setModelCatalog,
    setSelectedModel,
    setIsLoading,
    onLog,
  });

  useEffect(() => {
    const targetDefault = pickPreferredSerialPort(availablePorts);
    if (
      !targetDefault ||
      !PICO_PORT_PATTERN.test(targetDefault) ||
      Boolean(deviceStatus?.connected) ||
      isConnecting ||
      manualDisconnect ||
      autoAttemptedPort === targetDefault
    ) {
      return undefined;
    }

    const controller = new AbortController();
    void Promise.resolve().then(async () => {
      if (controller.signal.aborted) return;
      setAutoAttemptedPort(targetDefault);
      onLog?.('info', 'HOST', translate('errors.pico.autoDetectedPort', { port: targetDefault }));
      try {
        setError(null);
        await toggleConnection(targetDefault);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    });
    return () => controller.abort();
  }, [
    availablePorts,
    autoAttemptedPort,
    deviceStatus?.connected,
    isConnecting,
    manualDisconnect,
    onLog,
    toggleConnection,
  ]);

  return {
    isLoading: isLoading || isConnecting,
    error,
    refreshModelCatalog,
    selectTargetModel,
    refreshPorts,
    pollDeviceStatus,
    changeBreakinMode,
    toggleConnection,
  };
}
