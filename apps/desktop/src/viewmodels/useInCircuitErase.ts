import { useCallback, useState } from 'react';
import { IN_CIRCUIT_LOG_REPLAY_STEP_MS } from '@/constants';
import { useManagedTimeouts } from '@/hooks/useManagedTimeouts';
import { translate } from '@/i18n/useTranslation';
import type { FirstContactBackupResult, InCircuitNvramEraseResult } from '@/models';
import { runPicoFirstContactBackup, runPicoInCircuitErase } from '@/services/picoService';

export interface UseInCircuitEraseOptions {
  selectedPort?: string;
  onLog?: (
    level: 'info' | 'warn' | 'error' | 'success' | 'rx',
    source: 'HOST' | 'PICO',
    message: string,
  ) => void;
  onSuccess?: () => void;
}

/**
 * Closed algebraic state machine for In-Circuit Erasure.
 * Eliminates illegal states at compile time (ADTs / Discriminated Union).
 */
export type InCircuitEraseState =
  | { status: 'idle' }
  | { status: 'erasing' }
  | { status: 'success'; result: InCircuitNvramEraseResult }
  | { status: 'error'; error: string };

/**
 * Single Responsibility: Manages in-circuit RAM loader injection, first-contact backup,
 * and NVRAM sector erasure lifecycle.
 */
export function useInCircuitErase({ selectedPort, onLog, onSuccess }: UseInCircuitEraseOptions = {}) {
  const { scheduleTimeout } = useManagedTimeouts();
  const [state, setState] = useState<InCircuitEraseState>({ status: 'idle' });
  const [backupState, setBackupState] = useState<{
    isBackingUp: boolean;
    result: FirstContactBackupResult | null;
    error: string | null;
  }>({
    isBackingUp: false,
    result: null,
    error: null,
  });

  const isErasing = state.status === 'erasing';
  const eraseResult = state.status === 'success' ? state.result : null;
  const eraseError = state.status === 'error' ? state.error : null;

  const replayPicoLogLines = useCallback(
    (lines: readonly string[]) => {
      lines.forEach((line, index) => {
        scheduleTimeout(() => {
          onLog?.('rx', 'PICO', line);
        }, index * IN_CIRCUIT_LOG_REPLAY_STEP_MS);
      });
    },
    [onLog, scheduleTimeout],
  );

  const executeFirstContactBackup = useCallback(async () => {
    setBackupState({ isBackingUp: true, result: null, error: null });
    onLog?.('info', 'HOST', translate('errors.erase.firstContactStart'));
    try {
      const result = await runPicoFirstContactBackup(selectedPort ?? '');
      setBackupState({ isBackingUp: false, result, error: null });
      replayPicoLogLines(result.log_lines);
      if (result.success) {
        onLog?.(
          'success',
          'HOST',
          translate('errors.erase.firstContactComplete', {
            model: result.detected_model,
            bytes: result.nvram_bytes,
          }),
        );
      } else {
        onLog?.('error', 'HOST', translate('errors.erase.firstContactNoResponse'));
      }
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      setBackupState({ isBackingUp: false, result: null, error: errMsg });
      onLog?.('error', 'HOST', translate('errors.erase.firstContactFailed', { reason: errMsg }));
    }
  }, [selectedPort, onLog, replayPicoLogLines]);

  const injectRamLoaderAndErase = useCallback(async () => {
    setState({ status: 'erasing' });
    onLog?.('info', 'HOST', translate('errors.erase.surgicalStart'));
    try {
      const result = await runPicoInCircuitErase(selectedPort ?? '');
      setState({ status: 'success', result });
      replayPicoLogLines(result.log_lines);
      onSuccess?.();
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      setState({ status: 'error', error: errMsg });
      onLog?.('error', 'HOST', translate('errors.erase.surgicalFailed', { reason: errMsg }));
    }
  }, [selectedPort, onLog, onSuccess, replayPicoLogLines]);

  return {
    state,
    isErasing,
    eraseResult,
    eraseError,
    injectRamLoaderAndErase,
    isBackingUp: backupState.isBackingUp,
    backupResult: backupState.result,
    backupError: backupState.error,
    executeFirstContactBackup,
  };
}
