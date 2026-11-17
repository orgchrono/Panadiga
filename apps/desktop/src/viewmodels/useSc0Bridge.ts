import { useCallback, useEffect, useState } from 'react';
import { PICO_DEFAULT_BAUD_RATE, SC0_BRIDGE_MAX_LINES } from '@/constants';
import { translate } from '@/i18n/useTranslation';
import type {
  BridgeStreamLine,
  PicoDiagnosticResult,
  Sc0BridgeInputMode,
  Sc0BridgeStatus,
  SerialLogEntry,
} from '@/models';
import {
  getSc0BridgeStatus,
  sendSc0BridgeInput,
  startSc0Bridge,
  stopSc0Bridge,
  subscribePicoBridgeData,
} from '@/services/picoService';
import { updatePicoSessionSc0Status } from '@/store/picoSessionBridge';
import { createIdGenerator } from '@/utils/formatters';
import { registerAsyncSubscriptions } from '@/utils/subscription';

export type { BridgeStreamLine };

export interface UseSc0BridgeOptions {
  selectedPort: string;
  onExecuteDiagnostic: (commandId: string, arg?: string) => Promise<PicoDiagnosticResult | null>;
  onLog?: (
    level: SerialLogEntry['level'],
    source: SerialLogEntry['source'],
    message: string,
  ) => void;
}

const DEFAULT_BRIDGE_STATUS: Sc0BridgeStatus = Object.freeze({
  running: false,
  port_name: '',
  baud_rate: PICO_DEFAULT_BAUD_RATE,
  boot_mode: 'JIG:350',
  bytes_rx: 0,
  bytes_tx: 0,
  last_event: 'Standby',
});

const nextBridgeLineId = createIdGenerator('bridge');

/**
 * MVVM ViewModel for Interactive SC0 Bridge (`BRIDGE`, exit `'~~~'`),
 * `WAITTX` triggers, `PANA_RSP` quick probes (`0x15`, `0E0402`, `$g#67`, `$Z0#8A`),
 * and optical drive serial commands (`PASS`, `START`, `LDD`, `DSCREG`, `FEP`).
 */
export function useSc0Bridge({
  selectedPort,
  onExecuteDiagnostic,
  onLog,
}: UseSc0BridgeOptions) {
  const [bridgeStatus, setBridgeStatus] = useState<Sc0BridgeStatus>(DEFAULT_BRIDGE_STATUS);
  const [bridgeLogs, setBridgeLogs] = useState<BridgeStreamLine[]>([]);
  const [baudRate, setBaudRate] = useState<string>(String(PICO_DEFAULT_BAUD_RATE));
  const [bootMode, setBootMode] = useState<string>('JIG:350');
  const [inputMode, setInputMode] = useState<Sc0BridgeInputMode>('line');
  const [txInput, setTxInput] = useState<string>('');
  const [isBusy, setIsBusy] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(
    () =>
      registerAsyncSubscriptions([
        () =>
          subscribePicoBridgeData((chunk) => {
            if (!chunk) return;
            const line: BridgeStreamLine = { ...chunk, id: nextBridgeLineId() };
            setBridgeLogs((prev) => [...prev, line].slice(-SC0_BRIDGE_MAX_LINES));
            if (chunk.stream === 'rx') {
              setBridgeStatus((prev) => ({
                ...prev,
                bytes_rx: prev.bytes_rx + chunk.bytes.length,
              }));
            } else if (chunk.stream === 'tx') {
              setBridgeStatus((prev) => ({
                ...prev,
                bytes_tx: prev.bytes_tx + chunk.bytes.length,
              }));
            } else if (chunk.text.includes('finalizada') || chunk.text.includes('BRIDGE: OK')) {
              setBridgeStatus((prev) => ({ ...prev, running: false, last_event: chunk.text }));
            }
          }).catch((err) => {
            setError(translate('errors.pico.sc0SubscribeFailed', { reason: String(err) }));
            return () => {};
          }),
      ]),
    [],
  );

  useEffect(() => {
    updatePicoSessionSc0Status(bridgeStatus.running);
  }, [bridgeStatus.running]);

  const refreshBridgeStatus = useCallback(async (): Promise<void> => {
    try {
      const status = await getSc0BridgeStatus();
      if (status && typeof status.running === 'boolean') {
        setBridgeStatus(status);
      }
    } catch {
      /* Ignore transient poll errors in browser/test harness */
    }
  }, []);

  const startBridge = useCallback(async (): Promise<void> => {
    const targetPort = selectedPort || 'AUTO';
    setIsBusy(true);
    setError(null);
    try {
      const parsedBaud = Number.parseInt(baudRate, 10) || PICO_DEFAULT_BAUD_RATE;
      const status = await startSc0Bridge(targetPort, parsedBaud, bootMode);
      setBridgeStatus(status);
      onLog?.(
        'info',
        'HOST',
        translate('errors.pico.sc0Started', {
          baud: parsedBaud,
          port: status.port_name || targetPort,
          mode: bootMode,
        }),
      );
    } catch (err) {
      const msg = String(err);
      setError(msg);
      onLog?.('error', 'HOST', translate('errors.pico.sc0StartFailed', { reason: msg }));
    } finally {
      setIsBusy(false);
    }
  }, [selectedPort, baudRate, bootMode, onLog]);

  const stopBridge = useCallback(async (): Promise<void> => {
    setIsBusy(true);
    setError(null);
    try {
      const status = await stopSc0Bridge();
      setBridgeStatus(status);
      onLog?.('info', 'HOST', translate('errors.pico.sc0Stopped'));
    } catch (err) {
      const msg = String(err);
      setError(msg);
      onLog?.('error', 'HOST', translate('errors.pico.sc0StopFailed', { reason: msg }));
    } finally {
      setIsBusy(false);
    }
  }, [onLog]);

  const sendBridgePayload = useCallback(
    async (mode: Sc0BridgeInputMode, payload: string): Promise<void> => {
      if (!payload.trim()) return;
      setError(null);
      try {
        await sendSc0BridgeInput(mode, payload);
        await refreshBridgeStatus();
      } catch (err) {
        setError(String(err));
      }
    },
    [refreshBridgeStatus],
  );

  const submitTxInput = useCallback(async (): Promise<void> => {
    const trimmed = txInput.trim();
    if (!trimmed) return;
    await sendBridgePayload(inputMode, trimmed);
    setTxInput('');
  }, [txInput, inputMode, sendBridgePayload]);

  const clearBridgeLogs = useCallback((): void => {
    setBridgeLogs([]);
    setError(null);
  }, []);

  const triggerInteractiveWaittx = useCallback(
    async (spec: string): Promise<void> => {
      setError(null);
      await onExecuteDiagnostic('WAITTX', spec);
    },
    [onExecuteDiagnostic],
  );

  const sendPanaRspPreset = useCallback(
    async (
      mode: Sc0BridgeInputMode,
      bridgePayload: string,
      standalonePanarspArg: string,
    ): Promise<void> => {
      if (bridgeStatus.running) {
        await sendBridgePayload(mode, bridgePayload);
      } else {
        await onExecuteDiagnostic('PANARSP', standalonePanarspArg);
      }
    },
    [bridgeStatus.running, sendBridgePayload, onExecuteDiagnostic],
  );

  const sendDriveCommand = useCallback(
    async (driveCmd: string): Promise<void> => {
      if (bridgeStatus.running) {
        await sendBridgePayload('line', driveCmd);
      } else {
        await onExecuteDiagnostic('DRVPANA', driveCmd);
      }
    },
    [bridgeStatus.running, sendBridgePayload, onExecuteDiagnostic],
  );

  return {
    bridgeStatus,
    bridgeLogs,
    baudRate,
    setBaudRate,
    bootMode,
    setBootMode,
    inputMode,
    setInputMode,
    txInput,
    setTxInput,
    isBusy,
    error,
    startBridge,
    stopBridge,
    submitTxInput,
    clearBridgeLogs,
    triggerInteractiveWaittx,
    sendPanaRspPreset,
    sendDriveCommand,
  };
}
