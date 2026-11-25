/**
 * @file viewmodels/picoDevice/usePicoTelemetryEffects.ts
 * @brief Reactive telemetry stream subscription, SSOT voltage alert logging, and Redux session sync effects.
 */

import { useEffect, useState, type Dispatch, type SetStateAction } from 'react';
import { PICO_MAX_VDD_CEILING_VOLTS, PICO_MIN_VDD_THRESHOLD_VOLTS } from '@/constants';
import { translate } from '@/i18n/useTranslation';
import type { PicoDeviceStatus } from '@/models';
import { subscribeBrowserUsbHotplug } from '@/services/browserActions';
import {
  emitPicoTelemetry,
  subscribePicoDiagnosticResult,
  subscribePicoPortsChanged,
  subscribePicoTelemetry,
} from '@/services/picoService';
import {
  getPicoSessionSnapshot,
  updatePicoSessionModel,
  updatePicoSessionPort,
  updatePicoSessionStatus,
} from '@/store/picoSessionBridge';
import { isTargetPowered } from '@/utils/formatters';
import { registerAsyncSubscriptions } from '@/utils/subscription';
import {
  classifyVoltageAlert,
  DEFAULT_DISCONNECTED_STATUS,
  type UsePicoDeviceOptions,
  type VoltageAlertLevel,
} from './stateHelpers';

interface UsePicoTelemetryEffectsParams {
  deviceStatus: PicoDeviceStatus | null;
  setDeviceStatus: Dispatch<SetStateAction<PicoDeviceStatus | null>>;
  isLoading: boolean;
  selectedPort: string;
  selectedModel: string;
  applyMeasuredVoltage: (vddVolts: number, commandId?: string, commandSuccess?: boolean) => void;
  refreshPorts: (injectedPortsOrSilent?: unknown) => Promise<void>;
  refreshModelCatalog: () => Promise<void>;
  pollDeviceStatus: () => Promise<void>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export function usePicoTelemetryEffects({
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
}: UsePicoTelemetryEffectsParams) {
  const [lastAlertLevel, setLastAlertLevel] = useState<VoltageAlertLevel>('ok');
  const [didInitialPoll, setDidInitialPoll] = useState<boolean>(false);

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (controller.signal.aborted) return;
      void refreshPorts();
      void refreshModelCatalog();
    });
    return () => controller.abort();
  }, [refreshPorts, refreshModelCatalog]);

  useEffect(() => {
    const onHardwareMutated = () => {
      void refreshPorts(true);
    };
    return subscribeBrowserUsbHotplug(onHardwareMutated, onHardwareMutated);
  }, [refreshPorts]);

  useEffect(
    () =>
      registerAsyncSubscriptions([
        () =>
          subscribePicoTelemetry((incoming) => {
            if (incoming && typeof incoming.vdd_volts === 'number') {
              setDeviceStatus((prev) => {
                if (!prev?.connected && !incoming.connected) return prev;
                const preservedVdd =
                  incoming.vdd_volts > 0
                    ? incoming.vdd_volts
                    : prev?.connected && incoming.connected && prev.vdd_volts > 0
                      ? prev.vdd_volts
                      : incoming.vdd_volts;
                const powered = isTargetPowered(preservedVdd, PICO_MIN_VDD_THRESHOLD_VOLTS);
                return {
                  ...(prev || DEFAULT_DISCONNECTED_STATUS),
                  ...incoming,
                  vdd_volts: preservedVdd,
                  target_powered: powered,
                };
              });
            }
          }).catch((err) => {
            onLog?.(
              'error',
              'PICO',
              translate('errors.pico.telemetrySubscribeFailed', { reason: String(err) }),
            );
            return () => {};
          }),
        () =>
          subscribePicoDiagnosticResult((diag) => {
            if (diag && typeof diag.vdd_volts === 'number' && diag.vdd_volts > 0) {
              applyMeasuredVoltage(diag.vdd_volts, diag.command_id, diag.success);
            }
          }),
        () =>
          subscribePicoPortsChanged((ports) => {
            if (Array.isArray(ports)) {
              void refreshPorts(ports);
            }
          }),
      ]),
    [applyMeasuredVoltage, onLog, refreshPorts, setDeviceStatus],
  );

  useEffect(() => {
    updatePicoSessionStatus(deviceStatus);
    if (!deviceStatus) return undefined;
    emitPicoTelemetry(deviceStatus);
    const alertLevel = classifyVoltageAlert(deviceStatus.vdd_volts);
    if (alertLevel === lastAlertLevel) return undefined;

    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (controller.signal.aborted) return;
      setLastAlertLevel(alertLevel);
      if (alertLevel === 'over') {
        onLog?.(
          'error',
          'PICO',
          translate('errors.pico.vddOvervoltage', {
            vdd: deviceStatus.vdd_volts.toFixed(2),
            limit: PICO_MAX_VDD_CEILING_VOLTS.toFixed(2),
          }),
        );
      } else if (alertLevel === 'low') {
        onLog?.(
          'warn',
          'PICO',
          translate('errors.pico.vddUndervoltage', {
            vdd: deviceStatus.vdd_volts.toFixed(2),
            limit: PICO_MIN_VDD_THRESHOLD_VOLTS.toFixed(2),
          }),
        );
      }
    });
    return () => controller.abort();
  }, [deviceStatus, lastAlertLevel, onLog]);

  useEffect(() => {
    updatePicoSessionPort(selectedPort);
  }, [selectedPort]);

  useEffect(() => {
    updatePicoSessionModel(selectedModel);
  }, [selectedModel]);

  useEffect(() => {
    if (didInitialPoll) return undefined;
    const snap = getPicoSessionSnapshot();
    if (!snap.status?.connected || !snap.port) return undefined;
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (controller.signal.aborted) return;
      setDidInitialPoll(true);
      void pollDeviceStatus();
    });
    return () => controller.abort();
  }, [didInitialPoll, pollDeviceStatus]);

  return { isLoading };
}
