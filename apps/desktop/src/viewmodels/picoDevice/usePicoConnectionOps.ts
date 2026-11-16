/**
 * @file viewmodels/picoDevice/usePicoConnectionOps.ts
 * @brief Hook managing Pico device connection toggling, telemetry polling, and cold break-in mode switching.
 */

import {
  useCallback,
  useEffect,
  type Dispatch,
  type MutableRefObject,
  type SetStateAction,
} from 'react';
import type { ColdBreakinMode, PicoDeviceStatus } from '@/models';
import { invokeCommand } from '@/services/ipc';
import { logger } from '@/utils/logger';
import {
  disconnectTransportPort,
  establishTransportConnection,
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
  isConnectingRef: MutableRefObject<boolean>;
  manualDisconnectRef: MutableRefObject<boolean>;
  toggleConnectionRef: MutableRefObject<((overridePort?: unknown) => Promise<void>) | null>;
  sendDirectRawCommand: (command: string) => Promise<string>;
  onLog?: UsePicoDeviceOptions['onLog'];
}

export function usePicoConnectionOps({
  deviceStatus,
  setDeviceStatus,
  setIsLoading,
  selectedPort,
  isConnectingRef,
  manualDisconnectRef,
  toggleConnectionRef,
  sendDirectRawCommand,
  onLog,
}: UsePicoConnectionOpsParams) {
  const pollDeviceStatus = useCallback(async () => {
    if (!selectedPort || isConnectingRef.current) return;
    if (await isGdbBridgeRunning()) return;
    if (selectedPort.startsWith('ANDROID-USB-OTG') || selectedPort.startsWith('BLE_')) {
      try {
        const raw = await sendDirectRawCommand('STATUS');
        const vdd = parseVddFromStatusRaw(raw);
        setDeviceStatus(prev => buildMobileDeviceStatus(prev, selectedPort, vdd));
      } catch (err) {
        logger.warn('PicoDevice', 'Failed mobile STATUS telemetry poll', err);
      }
      return;
    }

    setIsLoading(true);
    try {
      const status = await invokeCommand<PicoDeviceStatus>('pico_get_status', {
        portName: selectedPort,
        port_name: selectedPort,
      });
      setDeviceStatus(status);
    } catch (err) {
      logger.warn('PicoDevice', 'Failed querying Pico device telemetry', err);
    } finally {
      setIsLoading(false);
    }
  }, [isConnectingRef, selectedPort, sendDirectRawCommand, setDeviceStatus, setIsLoading]);

  const changeBreakinMode = useCallback(
    async (mode: ColdBreakinMode) => {
      try {
        if (selectedPort.startsWith('ANDROID-USB-OTG') || selectedPort.startsWith('BLE_')) {
          await sendDirectRawCommand(`BREAK_IN ${mode}`);
          setDeviceStatus(prev => (prev ? { ...prev, breakin_mode: mode } : prev));
          onLog?.('info', 'HOST', `Modo de Cold Break-In alterado para: Modo ${mode}`);
          return;
        }
        const updated = await invokeCommand<PicoDeviceStatus>('pico_set_breakin_mode', {
          portName: selectedPort,
          port_name: selectedPort,
          mode,
        });
        setDeviceStatus(updated);
        onLog?.('info', 'HOST', `Modo de Cold Break-In alterado para: Modo ${mode}`);
      } catch (err) {
        onLog?.('error', 'HOST', `Erro ao alterar modo: ${err}`);
      }
    },
    [selectedPort, sendDirectRawCommand, setDeviceStatus, onLog]
  );

  const toggleConnection = useCallback(
    async (overridePort?: unknown) => {
      const targetPort =
        typeof overridePort === 'string' && overridePort.trim() ? overridePort : selectedPort;
      if (!targetPort) {
        onLog?.('warn', 'HOST', 'Nenhuma porta selecionada para conexão.');
        return;
      }

      if (isConnectingRef.current) {
        onLog?.('warn', 'HOST', 'Conexão já em andamento — aguarde antes de tentar novamente.');
        return;
      }

      if (deviceStatus?.connected) {
        manualDisconnectRef.current = true;
        await disconnectTransportPort({
          targetPort,
          setDeviceStatus,
          sendDirectRawCommand,
          onLog,
        });
        return;
      }

      if (await isGdbBridgeRunning()) {
        onLog?.(
          'warn',
          'HOST',
          `${targetPort} está em uso pela Ponte GDB RSP (a sessão de depuração já usa o Pico). Pare a ponte para enviar comandos CDC diretos.`
        );
        return;
      }

      manualDisconnectRef.current = false;
      setIsLoading(true);
      isConnectingRef.current = true;
      onLog?.('info', 'HOST', `Estabelecendo conexão física com ${targetPort} (115200 bps)...`);

      try {
        await establishTransportConnection({ targetPort, setDeviceStatus, onLog });
      } catch (err) {
        const errMsg = err instanceof Error ? err.message : String(err);
        onLog?.('error', 'HOST', `Falha ao conectar em ${targetPort}: ${errMsg}`);
        setDeviceStatus(prev => ({
          ...(prev || DEFAULT_DISCONNECTED_STATUS),
          connected: false,
          port: targetPort,
          core_state: 'DISCONNECTED',
          vdd_volts: 0.0,
          target_powered: false,
        }));
      } finally {
        setIsLoading(false);
        isConnectingRef.current = false;
      }
    },
    [
      deviceStatus,
      isConnectingRef,
      manualDisconnectRef,
      onLog,
      selectedPort,
      sendDirectRawCommand,
      setDeviceStatus,
      setIsLoading,
    ]
  );

  useEffect(() => {
    toggleConnectionRef.current = toggleConnection;
  }, [toggleConnection, toggleConnectionRef]);

  return {
    pollDeviceStatus,
    changeBreakinMode,
    toggleConnection,
  };
}
