/**
 * @file viewmodels/picoDevice/usePicoTransport.ts
 * @brief Transport hook facade composing port discovery and connection lifecycle operations.
 */

import {
  useEffect,
  useRef,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from 'react';
import type { DigaModelProfile, PicoDeviceStatus } from '@/models';
import type { UsePicoDeviceOptions } from './stateHelpers';
import { usePicoConnectionOps } from './usePicoConnectionOps';
import { usePicoPortDiscovery } from './usePicoPortDiscovery';

interface UsePicoTransportParams {
  deviceStatus: PicoDeviceStatus | null;
  setDeviceStatus: Dispatch<SetStateAction<PicoDeviceStatus | null>>;
  setIsLoading: Dispatch<SetStateAction<boolean>>;
  setAvailablePorts: Dispatch<SetStateAction<string[]>>;
  selectedPort: string;
  setSelectedPort: Dispatch<SetStateAction<string>>;
  modelCatalog: DigaModelProfile[];
  setModelCatalog: Dispatch<SetStateAction<DigaModelProfile[]>>;
  setSelectedModel: Dispatch<SetStateAction<string>>;
  isConnectingRef: MutableRefObject<boolean>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export function usePicoTransport({
  deviceStatus,
  setDeviceStatus,
  setIsLoading,
  setAvailablePorts,
  selectedPort,
  setSelectedPort,
  modelCatalog,
  setModelCatalog,
  setSelectedModel,
  isConnectingRef,
  onLog,
}: UsePicoTransportParams) {
  const manualDisconnectRef = useRef<boolean>(false);
  const isConnectedRef = useRef<boolean>(Boolean(deviceStatus?.connected));
  const toggleConnectionRef = useRef<((overridePort?: unknown) => Promise<void>) | null>(null);

  useEffect(() => {
    isConnectedRef.current = Boolean(deviceStatus?.connected);
  }, [deviceStatus?.connected]);

  const { sendDirectRawCommand, refreshModelCatalog, selectTargetModel, refreshPorts } =
    usePicoPortDiscovery({
      selectedPort,
      setSelectedPort,
      setAvailablePorts,
      modelCatalog,
      setModelCatalog,
      setSelectedModel,
      setIsLoading,
      isConnectedRef,
      isConnectingRef,
      manualDisconnectRef,
      toggleConnectionRef,
      onLog,
    });

  const { pollDeviceStatus, changeBreakinMode, toggleConnection } = usePicoConnectionOps({
    deviceStatus,
    setDeviceStatus,
    setIsLoading,
    selectedPort,
    isConnectingRef,
    manualDisconnectRef,
    toggleConnectionRef,
    sendDirectRawCommand,
    onLog,
  });

  return {
    refreshModelCatalog,
    selectTargetModel,
    refreshPorts,
    pollDeviceStatus,
    changeBreakinMode,
    toggleConnection,
  };
}
