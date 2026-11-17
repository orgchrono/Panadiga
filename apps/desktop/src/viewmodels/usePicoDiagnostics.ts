import { useCallback, useEffect, useState } from 'react';
import {
  PICO_DIAGNOSTIC_DEFAULT_TIMEOUT_MS,
  PICO_DIAGNOSTIC_HISTORY_CAP,
} from '@/constants';
import { translate } from '@/i18n/useTranslation';
import type {
  HardwareSsotSpec,
  PicoCommandCategory,
  PicoDiagnosticResult,
  SerialLogEntry,
} from '@/models';
import { sendAndroidUsbOtgCommand } from '@/services/androidBridge';
import { bleService } from '@/services/bleService';
import { parseDiagnosticOutput } from '@/services/diagnosticParser';
import {
  executePicoDiagnostic,
  parsePicoDiagnosticOutput,
  subscribePicoDiagnosticResult,
} from '@/services/picoService';
import { ingestP7501DumpOutput } from '@/store/nvramCaptureBridge';
import {
  updatePicoSessionDiagnostic,
  updatePicoSessionRunningCommand,
} from '@/store/picoSessionBridge';
import { registerAsyncSubscriptions } from '@/utils/subscription';

export interface UsePicoDiagnosticsOptions {
  selectedPort: string;
  ssot: HardwareSsotSpec | null;
  onLog?: (
    level: SerialLogEntry['level'],
    source: SerialLogEntry['source'],
    message: string,
  ) => void;
  onTelemetryUpdate?: (result: PicoDiagnosticResult) => void;
}

/**
 * Granular Diagnostic Studio ViewModel:
 * Executes any firmware CLI command (`PINCHECK`, `BUSDIAG`, `SNIFF`, `DUMP`, `CFI`, `BOOTCAP`, `FMU*`)
 * transparently across Desktop USB CDC, Android USB-C OTG, and Bluetooth LE NUS.
 */
export function usePicoDiagnostics({
  selectedPort,
  ssot,
  onLog,
  onTelemetryUpdate,
}: UsePicoDiagnosticsOptions) {
  const [history, setHistory] = useState<PicoDiagnosticResult[]>([]);
  const [latestResult, setLatestResult] = useState<PicoDiagnosticResult | null>(null);
  const [runningCommandId, setRunningCommandId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedCategory, setSelectedCategory] = useState<PicoCommandCategory>('electrical');

  useEffect(
    () =>
      registerAsyncSubscriptions([
        () =>
          subscribePicoDiagnosticResult((payload) => {
            if (!payload || !payload.command_id) return;
            setLatestResult(payload);
            updatePicoSessionDiagnostic(payload);
            setHistory((prev) => [payload, ...prev].slice(0, PICO_DIAGNOSTIC_HISTORY_CAP));
            onTelemetryUpdate?.(payload);
          }),
      ]),
    [onTelemetryUpdate],
  );

  const commitDiagnosticResult = useCallback(
    (result: PicoDiagnosticResult, logReply = false): PicoDiagnosticResult => {
      setLatestResult(result);
      updatePicoSessionDiagnostic(result);
      setHistory((prev) => {
        if (prev[0]?.raw_command === result.raw_command && prev[0]?.elapsed_ms === result.elapsed_ms) {
          return prev;
        }
        return [result, ...prev].slice(0, PICO_DIAGNOSTIC_HISTORY_CAP);
      });
      if (logReply) {
        onLog?.(result.success ? 'rx' : 'error', 'PICO', result.summary);
      }
      onTelemetryUpdate?.(result);
      return result;
    },
    [onLog, onTelemetryUpdate],
  );

  const executeCommand = useCallback(
    async (commandId: string, arg?: string): Promise<PicoDiagnosticResult | null> => {
      const norm = commandId.trim().toUpperCase();
      const spec = ssot?.pico_commands.find((c) => c.id === norm);
      const cleanArg = arg?.trim() ? arg.trim() : undefined;
      const rawCommand = cleanArg ? `${norm} ${cleanArg}` : norm;
      const timeoutMs = spec?.timeout_ms ?? PICO_DIAGNOSTIC_DEFAULT_TIMEOUT_MS;

      setRunningCommandId(norm);
      updatePicoSessionRunningCommand(norm);
      setError(null);

      try {
        const executeDirect = async (
          transportTag: string,
          runTransport: () => Promise<{ ok: boolean; output: string }>,
        ): Promise<PicoDiagnosticResult> => {
          onLog?.('tx', 'HOST', `> [${transportTag}] ${rawCommand}`);
          const started = performance.now();
          const reply = await runTransport();
          const elapsed = Math.round(performance.now() - started);
          const rawParsed = await parsePicoDiagnosticOutput(
            norm,
            rawCommand,
            reply.output,
            elapsed,
          ).catch(() => parseDiagnosticOutput(norm, rawCommand, reply.output, elapsed));
          const parsed: PicoDiagnosticResult = reply.ok
            ? rawParsed
            : { ...rawParsed, success: false };
          if (parsed.success && norm === 'DUMP') {
            void ingestP7501DumpOutput(reply.output, transportTag, selectedPort).catch(() => {});
          }
          return commitDiagnosticResult(parsed, true);
        };

        if (selectedPort.startsWith('ANDROID-USB-OTG')) {
          return await executeDirect('USB-OTG', async () =>
            sendAndroidUsbOtgCommand(rawCommand, timeoutMs),
          );
        }

        if (selectedPort.startsWith('BLE_') && bleService.isConnected()) {
          return await executeDirect('BLE NUS', async () => ({
            ok: true,
            output: await bleService.sendAndAwaitReply(rawCommand, timeoutMs),
          }));
        }

        if (!selectedPort) {
          const msg = translate('errors.pico.noDiagnosticProbe');
          setError(msg);
          throw new Error(msg);
        }

        const result = await executePicoDiagnostic(selectedPort, norm, cleanArg ?? null);
        if (norm === 'DUMP' && result.success && result.raw_output) {
          await ingestP7501DumpOutput(result.raw_output, 'DESKTOP_CDC', selectedPort);
        }
        return commitDiagnosticResult(result);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        onLog?.(
          'error',
          'HOST',
          translate('errors.pico.diagnosticFailed', { cmd: rawCommand, reason: msg }),
        );
        return null;
      } finally {
        setRunningCommandId(null);
        updatePicoSessionRunningCommand(null);
      }
    },
    [selectedPort, ssot, onLog, commitDiagnosticResult],
  );

  const clearHistory = useCallback(() => {
    setHistory([]);
    setLatestResult(null);
    setError(null);
  }, []);

  return {
    history,
    latestResult,
    runningCommandId,
    isExecuting: runningCommandId !== null,
    isLoading: runningCommandId !== null,
    error,
    selectedCategory,
    setSelectedCategory,
    executeCommand,
    clearHistory,
  };
}
