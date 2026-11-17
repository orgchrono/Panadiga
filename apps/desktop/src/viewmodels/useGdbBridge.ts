import { useCallback, useEffect, useState } from 'react';
import { TELEMETRY_POLL_INTERVAL_MS } from '@/constants';
import { translate } from '@/i18n/useTranslation';
import type {
  Am34RegisterState,
  GdbBridgeStatus,
  GdbPacketTelemetryEvent,
  SerialLogEntry,
} from '@/models';
import { copyToClipboard } from '@/services/browserActions';
import {
  getAm34Registers,
  getGdbBridgeStatus,
  getGdbTelemetry,
  launchExternalTool,
  startGdbBridge,
  stopGdbBridge,
  type ExternalToolId,
} from '@/services/picoService';
import { updatePicoSessionGdbStatus } from '@/store/picoSessionBridge';
import { getRuntimeSettingsSnapshot } from '@/store/runtimeSettingsBridge';
import { logger } from '@/utils/logger';

export interface UseGdbBridgeOptions {
  selectedPort: string;
  onLog?: (level: SerialLogEntry['level'], source: SerialLogEntry['source'], message: string) => void;
}

/**
 * Closed algebraic state machine for GDB RSP server bridge lifecycle.
 * Eliminates illegal states at compile time (ADTs / Discriminated Union).
 */
export type GdbBridgeState =
  | { status: 'stopped' }
  | { status: 'starting'; listenAddr: string }
  | { status: 'running'; listenAddr: string; clientCount: number }
  | { status: 'error'; message: string };

/**
 * Single Responsibility: Manages the GDB RSP TCP server bridge lifecycle, AM34 register file,
 * and introspective RSP telemetry stream.
 */
export function useGdbBridge({ selectedPort, onLog }: UseGdbBridgeOptions) {
  const [gdbStatus, setGdbStatus] = useState<GdbBridgeStatus | null>(null);
  const [am34Registers, setAm34Registers] = useState<Am34RegisterState | null>(null);
  const [telemetry, setTelemetry] = useState<GdbPacketTelemetryEvent[]>([]);
  const [state, setState] = useState<GdbBridgeState>({ status: 'stopped' });

  const isGdbStarting = state.status === 'starting';

  const pollGdbState = useCallback(async () => {
    try {
      const gdb = await getGdbBridgeStatus();
      setGdbStatus(gdb);
      if (gdb?.running) {
        setState({
          status: 'running',
          listenAddr: gdb.listen_addr,
          clientCount: gdb.connected_clients,
        });
        const [regs, events] = await Promise.all([
          getAm34Registers(),
          getGdbTelemetry().catch(() => []),
        ]);
        setAm34Registers(regs);
        if (events) {
          setTelemetry(events);
        }
      } else {
        setState((prev) => (prev.status === 'starting' ? prev : { status: 'stopped' }));
      }
    } catch (err) {
      logger.warn('GdbBridge', 'Failed querying GDB status', err);
    }
  }, []);

  const startGdbServer = useCallback(
    async (listenAddr = getRuntimeSettingsSnapshot().serial.gdb_listen_addr) => {
      const targetPort = selectedPort || 'AUTO';
      setState({ status: 'starting', listenAddr });
      onLog?.(
        'info',
        'HOST',
        translate('errors.pico.gdbStarting', {
          addr: listenAddr,
          port: selectedPort || 'AUTO / Stub AM34',
        }),
      );
      try {
        const status = await startGdbBridge(listenAddr, targetPort);
        setGdbStatus(status);
        setState({
          status: 'running',
          listenAddr: status.listen_addr,
          clientCount: status.connected_clients,
        });
        onLog?.(
          'success',
          'HOST',
          translate('errors.pico.gdbStarted', {
            port: status.port_name,
            addr: status.listen_addr || listenAddr,
          }),
        );
      } catch (err) {
        const errMsg = err instanceof Error ? err.message : String(err);
        setState({ status: 'error', message: errMsg });
        onLog?.('error', 'HOST', translate('errors.pico.gdbStartFailed', { reason: errMsg }));
      }
    },
    [selectedPort, onLog],
  );

  const stopGdbServer = useCallback(async () => {
    onLog?.('warn', 'HOST', translate('errors.pico.gdbStopping'));
    try {
      const status = await stopGdbBridge();
      setGdbStatus(status);
      setState({ status: 'stopped' });
      onLog?.('info', 'HOST', translate('errors.pico.gdbStopped'));
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      setState({ status: 'error', message: errMsg });
      onLog?.('error', 'HOST', translate('errors.pico.gdbStopFailed', { reason: errMsg }));
    }
  }, [onLog]);

  const launchTool = useCallback(
    async (tool: ExternalToolId) => {
      onLog?.(
        'info',
        'HOST',
        translate('errors.pico.gdbLaunchingTool', { tool: tool.toUpperCase() }),
      );
      try {
        const msg = await launchExternalTool(tool);
        onLog?.('success', 'HOST', msg);
      } catch (err) {
        onLog?.(
          'error',
          'HOST',
          translate('errors.pico.gdbToolFailed', { tool, reason: String(err) }),
        );
      }
    },
    [onLog],
  );

  useEffect(() => {
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        void pollGdbState();
      }
    });
    const interval = setInterval(() => {
      void pollGdbState();
    }, TELEMETRY_POLL_INTERVAL_MS);
    return () => {
      controller.abort();
      clearInterval(interval);
    };
  }, [pollGdbState]);

  useEffect(() => {
    const running = state.status === 'running';
    const addr =
      state.status === 'running'
        ? state.listenAddr
        : getRuntimeSettingsSnapshot().serial.gdb_listen_addr;
    updatePicoSessionGdbStatus(running, addr);
  }, [state]);

  const copyConnectionCommand = useCallback((cmd: string) => {
    void copyToClipboard(cmd);
  }, []);

  return {
    state,
    gdbStatus,
    am34Registers,
    telemetry,
    isLoading: isGdbStarting,
    isGdbStarting,
    gdbError: state.status === 'error' ? state.message : null,
    startGdbServer,
    stopGdbServer,
    pollGdbState,
    launchTool,
    copyConnectionCommand,
  };
}
