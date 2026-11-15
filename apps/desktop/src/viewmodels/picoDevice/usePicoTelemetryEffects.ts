/**
 * @file viewmodels/picoDevice/usePicoTelemetryEffects.ts
 * @brief Reactive telemetry stream subscription, SSOT voltage alert logging, and session persistence effects.
 */

import { useEffect, type Dispatch, type MutableRefObject, type SetStateAction } from 'react';
import type { PicoDeviceStatus, PicoDiagnosticResult } from '@/models';
import { PICO_MAX_VDD_CEILING_VOLTS, PICO_MIN_VDD_THRESHOLD_VOLTS } from '@/constants';
import { isTargetPowered } from '@/utils/formatters';
import { emitLocalEvent, listenEvent } from '@/services/ipc';
import {
  classifyVoltageAlert,
  DEFAULT_DISCONNECTED_STATUS,
  picoSessionStore,
  type UsePicoDeviceOptions,
  type VoltageAlertLevel,
} from './stateHelpers';

interface UsePicoTelemetryEffectsParams {
  deviceStatus: PicoDeviceStatus | null;
  setDeviceStatus: Dispatch<SetStateAction<PicoDeviceStatus | null>>;
  selectedPort: string;
  selectedModel: string;
  lastAlertRef: MutableRefObject<VoltageAlertLevel>;
  applyMeasuredVoltage: (vddVolts: number, commandId?: string, commandSuccess?: boolean) => void;
  refreshPorts: () => Promise<void>;
  refreshModelCatalog: () => Promise<void>;
  pollDeviceStatus: () => Promise<void>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export function usePicoTelemetryEffects({
  deviceStatus,
  setDeviceStatus,
  selectedPort,
  selectedModel,
  lastAlertRef,
  applyMeasuredVoltage,
  refreshPorts,
  refreshModelCatalog,
  pollDeviceStatus,
  onLog,
}: UsePicoTelemetryEffectsParams) {
  useEffect(() => {
    refreshPorts();
    refreshModelCatalog();
  }, [refreshPorts, refreshModelCatalog]);

  useEffect(() => {
    let unlistenTelemetry: (() => void) | undefined;
    let unlistenDiag: (() => void) | undefined;

    listenEvent<PicoDeviceStatus>('pico://telemetry', incoming => {
      if (incoming && typeof incoming.vdd_volts === 'number') {
        setDeviceStatus(prev => {
          if (!prev?.connected && !incoming.connected) return prev;
          const preservedVdd =
            incoming.vdd_volts > 0
              ? incoming.vdd_volts
              : prev?.connected && incoming.connected && prev.vdd_volts > 0
                ? prev.vdd_volts
                : incoming.vdd_volts;
          const powered = isTargetPowered(preservedVdd, PICO_MIN_VDD_THRESHOLD_VOLTS);
          const updated: PicoDeviceStatus = {
            ...(prev || DEFAULT_DISCONNECTED_STATUS),
            ...incoming,
            vdd_volts: preservedVdd,
            target_powered: powered,
          };
          picoSessionStore.setStatus(updated);
          return updated;
        });
      }
    }).then(fn => {
      unlistenTelemetry = fn;
    });

    listenEvent<PicoDiagnosticResult>('pico://diagnostic-result', diag => {
      if (diag && typeof diag.vdd_volts === 'number' && diag.vdd_volts > 0) {
        applyMeasuredVoltage(diag.vdd_volts, diag.command_id, diag.success);
      }
    }).then(fn => {
      unlistenDiag = fn;
    });

    return () => {
      unlistenTelemetry?.();
      unlistenDiag?.();
    };
  }, [applyMeasuredVoltage, setDeviceStatus]);

  useEffect(() => {
    picoSessionStore.setStatus(deviceStatus);
    if (deviceStatus) {
      const alertLevel = classifyVoltageAlert(deviceStatus.vdd_volts);
      if (alertLevel === 'over' && lastAlertRef.current !== 'over') {
        lastAlertRef.current = 'over';
        onLog?.(
          'error',
          'PICO',
          `PERIGO CRÍTICO: Sobretensão detectada em VDD (${deviceStatus.vdd_volts.toFixed(2)}V > ${PICO_MAX_VDD_CEILING_VOLTS.toFixed(2)}V)! Desconecte imediatamente!`
        );
      } else if (alertLevel === 'low' && lastAlertRef.current !== 'low') {
        lastAlertRef.current = 'low';
        onLog?.(
          'warn',
          'PICO',
          `ALERTA REATIVO: Subtensão em VDD (${deviceStatus.vdd_volts.toFixed(2)}V < ${PICO_MIN_VDD_THRESHOLD_VOLTS.toFixed(2)}V). Alvo desligado ou pino VDD desconectado.`
        );
      } else if (alertLevel === 'ok') {
        lastAlertRef.current = 'ok';
      }
      emitLocalEvent('pico_telemetry', deviceStatus);
    }
  }, [deviceStatus, lastAlertRef, onLog]);

  useEffect(() => {
    picoSessionStore.setPort(selectedPort);
  }, [selectedPort]);

  useEffect(() => {
    picoSessionStore.setModel(selectedModel);
  }, [selectedModel]);

  useEffect(() => {
    const snap = picoSessionStore.getSnapshot();
    if (snap.status?.connected && snap.port) {
      void pollDeviceStatus();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}
