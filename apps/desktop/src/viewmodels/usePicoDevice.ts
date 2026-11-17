/**
 * @file viewmodels/usePicoDevice.ts
 * @brief ViewModel facade composing Pico USB/OTG/BLE transport, BOOTSEL flashing, and SSOT safety telemetry.
 */

import { useCallback, useState } from 'react';
import {
  PICO_MAX_VDD_CEILING_VOLTS,
  PICO_MIN_VDD_THRESHOLD_VOLTS,
  PICO_SAFE_OPERATING_MIN_VOLTS,
} from '@/constants';
import type { DigaModelProfile, PicoDeviceStatus } from '@/models';
import { getPicoSessionSnapshot, updatePicoSessionStatus } from '@/store/picoSessionBridge';
import { isTargetPowered, isTargetVoltageNominal } from '@/utils/formatters';
import {
  DEFAULT_DISCONNECTED_STATUS,
  deriveCoreStateFromVoltage,
  resolveTransportMode,
  type UsePicoDeviceOptions,
} from './picoDevice/stateHelpers';
import { usePicoBootFlash } from './picoDevice/usePicoBootFlash';
import { usePicoTelemetryEffects } from './picoDevice/usePicoTelemetryEffects';
import { usePicoTransport } from './picoDevice/usePicoTransport';

export { DEFAULT_DISCONNECTED_STATUS, resolveTransportMode, type UsePicoDeviceOptions };

/**
 * Single Responsibility: Manages Pico hardware link (Desktop USB CDC, Android USB-C OTG,
 * Bluetooth LE NUS), BOOTSEL flashing, SSOT voltage safety checks, and reactive telemetry streams.
 */
export function usePicoDevice({ onLog }: UsePicoDeviceOptions = {}) {
  const [initialSession] = useState(() => getPicoSessionSnapshot());
  const [deviceStatus, setDeviceStatus] = useState<PicoDeviceStatus | null>(initialSession.status);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [isFlashing, setIsFlashing] = useState<boolean>(false);
  const [availablePorts, setAvailablePorts] = useState<string[]>([]);
  const [selectedPort, setSelectedPort] = useState<string>(initialSession.port);
  const [modelCatalog, setModelCatalog] = useState<DigaModelProfile[]>([]);
  const [selectedModel, setSelectedModel] = useState<string>(initialSession.model);

  const applyMeasuredVoltage = useCallback(
    (vddVolts: number, commandId?: string, commandSuccess?: boolean) => {
      if (typeof vddVolts !== 'number' || isNaN(vddVolts)) return;
      const powered = isTargetPowered(vddVolts, PICO_MIN_VDD_THRESHOLD_VOLTS);
      setDeviceStatus((prev) => {
        if (!prev) return prev;
        const nextVdd =
          vddVolts > 0
            ? vddVolts
            : commandId === 'STATUS' || !prev.connected
              ? vddVolts
              : prev.vdd_volts;
        const nextPowered = isTargetPowered(nextVdd, PICO_MIN_VDD_THRESHOLD_VOLTS);
        const updated: PicoDeviceStatus = {
          ...prev,
          vdd_volts: nextVdd,
          target_powered: nextPowered,
          core_state: deriveCoreStateFromVoltage(
            prev.core_state,
            nextVdd,
            commandId,
            commandSuccess,
          ),
        };
        updatePicoSessionStatus(
          nextPowered || !prev.connected
            ? updated
            : { ...updated, vdd_volts: vddVolts, target_powered: powered },
        );
        return updated;
      });
    },
    [],
  );

  const {
    refreshModelCatalog,
    selectTargetModel,
    refreshPorts,
    pollDeviceStatus,
    changeBreakinMode,
    toggleConnection,
  } = usePicoTransport({
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
  });

  const { triggerBootselReboot, flashPicoFirmware } = usePicoBootFlash({
    selectedPort,
    setSelectedPort,
    setIsFlashing,
    refreshPorts,
    toggleConnection,
    onLog,
  });

  usePicoTelemetryEffects({
    deviceStatus,
    setDeviceStatus,
    isLoading,
    selectedPort,
    selectedModel,
    applyMeasuredVoltage,
    refreshPorts,
    refreshModelCatalog,
    pollDeviceStatus,
    onLog,
  });

  const isVoltageSafe = deviceStatus
    ? isTargetVoltageNominal(
        deviceStatus.vdd_volts,
        PICO_SAFE_OPERATING_MIN_VOLTS,
        PICO_MAX_VDD_CEILING_VOLTS,
      )
    : false;
  const isHardwareReady = Boolean(
    deviceStatus?.connected &&
      isTargetPowered(deviceStatus.vdd_volts, PICO_MIN_VDD_THRESHOLD_VOLTS) &&
      deviceStatus.vdd_volts <= PICO_MAX_VDD_CEILING_VOLTS,
  );

  return {
    deviceStatus,
    setDeviceStatus,
    applyMeasuredVoltage,
    isLoading,
    isFlashing,
    availablePorts,
    selectedPort,
    setSelectedPort,
    transportMode: resolveTransportMode(selectedPort),
    modelCatalog,
    selectedModel,
    selectTargetModel,
    refreshPorts,
    pollDeviceStatus,
    changeBreakinMode,
    toggleConnection,
    triggerBootselReboot,
    flashPicoFirmware,
    isVoltageSafe,
    isHardwareReady,
  };
}
