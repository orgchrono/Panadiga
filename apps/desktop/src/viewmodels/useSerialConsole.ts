import { useCallback, useEffect, useRef, useState } from 'react';
import {
  PICO_BLE_DUMP_REPLY_TIMEOUT_MS,
  PICO_OTG_COMMAND_TIMEOUT_MS,
  SERIAL_CONSOLE_LOG_CAP,
  SERIAL_LOG_TIMESTAMP_END,
  SERIAL_LOG_TIMESTAMP_START,
} from '@/constants';
import { translate } from '@/i18n/useTranslation';
import type { PicoDiagnosticResult, SerialConsoleState, SerialLogEntry } from '@/models';
import { sendAndroidUsbOtgCommand } from '@/services/androidBridge';
import { bleService } from '@/services/bleService';
import { parseDiagnosticOutput } from '@/services/diagnosticParser';
import { isPicoIpcAvailable, sendPicoSerialCommand, subscribePicoSerialLog } from '@/services/picoService';
import { ingestP7501DumpOutput } from '@/store/nvramCaptureBridge';
import { updatePicoSessionSerialLogs } from '@/store/picoSessionBridge';
import { createIdGenerator } from '@/utils/formatters';
import { registerAsyncSubscriptions } from '@/utils/subscription';

const nextSerialLogId = createIdGenerator('serial');

export type { SerialConsoleState };

export interface UseSerialConsoleOptions {
  selectedPort?: string;
  initialLogs?: SerialLogEntry[];
  onCommandTelemetry?: (result: PicoDiagnosticResult) => void;
}

const TELEMETRY_RELEVANT_VERBS = new Set([
  'STATUS',
  'VDD',
  'PINCHECK',
  'HANDSHAKE',
  'ISOLATE',
  'HIZ',
]);

function currentLogTimestamp(): string {
  return new Date().toISOString().slice(SERIAL_LOG_TIMESTAMP_START, SERIAL_LOG_TIMESTAMP_END);
}

/**
 * Single Responsibility: Manages interactive serial CDC / BLE / USB-OTG console logs
 * and reactive `pico://serial-log` streaming events.
 */
export function useSerialConsole({
  selectedPort,
  initialLogs = [],
  onCommandTelemetry,
}: UseSerialConsoleOptions = {}) {
  const [logs, setLogs] = useState<SerialLogEntry[]>(initialLogs);
  const [commandInput, setCommandInput] = useState<string>('');
  const [consoleState, setConsoleState] = useState<SerialConsoleState>({ status: 'idle' });
  const [latestCommandTelemetry, setLatestCommandTelemetry] =
    useState<PicoDiagnosticResult | null>(null);
  const logContainerRef = useRef<HTMLDivElement>(null);

  const addLog = useCallback(
    (
      level: SerialLogEntry['level'],
      source: SerialLogEntry['source'],
      message: string,
      hexData?: string,
    ) => {
      const newEntry: SerialLogEntry = {
        id: nextSerialLogId(),
        timestamp: currentLogTimestamp(),
        level,
        source,
        message,
        hexData,
      };
      setLogs((prev) => [...prev.slice(-SERIAL_CONSOLE_LOG_CAP), newEntry]);
    },
    [],
  );

  useEffect(() => {
    const unsubscribe = bleService.onReceive((data) => {
      const clean = data.trim();
      if (clean) {
        addLog('rx', 'PICO', `[BLE] ${clean}`);
      }
    });
    return unsubscribe;
  }, [addLog]);

  useEffect(
    () =>
      registerAsyncSubscriptions([
        () =>
          subscribePicoSerialLog((evt) => {
            if (!evt?.message) return;
            setLogs((prev) => {
              if (prev.some((item) => item.id === evt.id)) return prev;
              return [
                ...prev.slice(-SERIAL_CONSOLE_LOG_CAP),
                {
                  id: evt.id || nextSerialLogId(),
                  timestamp: evt.timestamp || currentLogTimestamp(),
                  level: evt.level || 'info',
                  source: evt.source || 'PICO',
                  message: evt.message,
                },
              ];
            });
          }),
      ]),
    [],
  );

  const emitConsoleEffects = useCallback(
    (cmd: string, reply: string, transportTag: string, targetPort: string) => {
      if (cmd.startsWith('DUMP')) {
        void ingestP7501DumpOutput(reply, transportTag, targetPort).catch(() => {});
      }
      const verb = cmd.split(/\s+/)[0] || cmd;
      if (TELEMETRY_RELEVANT_VERBS.has(verb)) {
        const parsed = parseDiagnosticOutput(verb, cmd, reply, 0);
        if (parsed.vdd_volts !== null && parsed.vdd_volts !== undefined) {
          setLatestCommandTelemetry(parsed);
          onCommandTelemetry?.(parsed);
        }
      }
    },
    [onCommandTelemetry],
  );

  useEffect(() => {
    updatePicoSessionSerialLogs(logs);
  }, [logs]);

  const sendCommandLine = useCallback(
    async (commandLine: string, overridePort?: string): Promise<void> => {
      if (!commandLine.trim()) return;
      const cmd = commandLine.trim().toUpperCase();
      const targetPort = overridePort || selectedPort || '';
      setConsoleState({ status: 'sending', command: cmd });
      addLog('tx', 'HOST', `> ${cmd}`);

      if (targetPort.startsWith('ANDROID-USB-OTG')) {
        try {
          const res = sendAndroidUsbOtgCommand(cmd, PICO_OTG_COMMAND_TIMEOUT_MS);
          addLog(res.ok ? 'rx' : 'error', 'PICO', res.output);
          if (res.ok) {
            emitConsoleEffects(cmd, res.output, 'ANDROID_USB_OTG', targetPort);
          }
        } catch (err) {
          addLog(
            'error',
            'HOST',
            translate('errors.serial.usbOtgError', {
              reason: err instanceof Error ? err.message : String(err),
            }),
          );
        } finally {
          setConsoleState({ status: 'idle' });
        }
        return;
      }

      if (bleService.isConnected()) {
        try {
          if (cmd.startsWith('DUMP')) {
            const reply = await bleService.sendAndAwaitReply(cmd, PICO_BLE_DUMP_REPLY_TIMEOUT_MS);
            addLog('rx', 'PICO', reply);
            emitConsoleEffects(cmd, reply, 'BLE_NUS', targetPort || 'BLE_PICO_W');
          } else {
            await bleService.send(cmd);
          }
        } catch (err) {
          addLog('error', 'HOST', translate('errors.serial.bleTxError', { reason: String(err) }));
        } finally {
          setConsoleState({ status: 'idle' });
        }
        return;
      }

      if (!targetPort) {
        addLog('error', 'HOST', translate('errors.serial.noPortSelected'));
        setConsoleState({ status: 'idle' });
        return;
      }

      if (!isPicoIpcAvailable()) {
        addLog('error', 'HOST', translate('errors.serial.tauriUnavailable'));
        setConsoleState({ status: 'idle' });
        return;
      }

      try {
        const response = await sendPicoSerialCommand(targetPort, cmd);
        const source = cmd === 'RESET' ? 'TARGET' : 'PICO';
        addLog('rx', source, response);
        emitConsoleEffects(cmd, response, 'USB_CDC', targetPort);
      } catch (err) {
        addLog(
          'error',
          'HOST',
          translate('errors.serial.cdcError', {
            reason: err instanceof Error ? err.message : String(err),
          }),
        );
      } finally {
        setConsoleState({ status: 'idle' });
      }
    },
    [selectedPort, addLog, emitConsoleEffects],
  );

  const sendCommand = useCallback(
    async (overridePort?: string) => {
      const nextLine = commandInput;
      if (!nextLine.trim()) return;
      setCommandInput('');
      await sendCommandLine(nextLine, overridePort);
    },
    [commandInput, sendCommandLine],
  );

  const clearLogs = useCallback(() => {
    setLogs([]);
  }, []);

  return {
    logs,
    consoleState,
    latestCommandTelemetry,
    isLoading: consoleState.status === 'sending',
    addLog,
    clearLogs,
    commandInput,
    setCommandInput,
    sendCommand,
    sendCommandLine,
    logContainerRef,
  };
}
