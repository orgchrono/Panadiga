import { useCallback, useEffect, useMemo, useState } from 'react';
import type { PicoBenchSectionFilter, PicoDiagnosticResult } from '@/models';
import { isAndroidNativeBridgeAvailable } from '@/services/androidBridge';
import { useAppSelector } from '@/store/hooks';
import { clearPicoCommandRequest, clearPicoSerialLine } from '@/store/picoSessionBridge';
import { isTargetPowered, resolveEffectiveVddVolts } from '@/utils/formatters';
import { useGdbBridge } from './useGdbBridge';
import { useHardwareSsot } from './useHardwareSsot';
import { useInCircuitErase } from './useInCircuitErase';
import { usePicoDevice } from './usePicoDevice';
import { usePicoDiagnostics } from './usePicoDiagnostics';
import { useSerialConsole } from './useSerialConsole';

/**
 * Coordinating ViewModel: Composes SSOT spec, Pico connection/transport,
 * Granular Diagnostic Studio, GDB RSP Bridge, In-Circuit Erase, and Reactive Console.
 */
export function usePicoBenchViewModel() {
  const {
    ssot,
    isLoaded: isSsotLoaded,
    isLoading: isSsotLoading,
    error: ssotError,
    reload: reloadSsot,
  } = useHardwareSsot();

  const console = useSerialConsole();
  const device = usePicoDevice({ onLog: console.addLog });
  const { setDeviceStatus, applyMeasuredVoltage } = device;

  const handleDiagnosticTelemetry = useCallback(
    (result: PicoDiagnosticResult) => {
      if (result.vdd_volts !== null && result.vdd_volts !== undefined) {
        applyMeasuredVoltage(result.vdd_volts, result.command_id, result.success);
      }
    },
    [applyMeasuredVoltage],
  );

  useEffect(() => {
    const latest = console.latestCommandTelemetry;
    if (!latest) return undefined;
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        handleDiagnosticTelemetry(latest);
      }
    });
    return () => controller.abort();
  }, [console.latestCommandTelemetry, handleDiagnosticTelemetry]);

  const diagnostics = usePicoDiagnostics({
    selectedPort: device.selectedPort,
    ssot,
    onLog: console.addLog,
    onTelemetryUpdate: handleDiagnosticTelemetry,
  });

  const queuedCommandRequest = useAppSelector((s) => s.picoSession.queuedCommandRequest);
  const queuedSerialLine = useAppSelector((s) => s.picoSession.queuedSerialLine);
  const executeDiagnosticCommand = diagnostics.executeCommand;
  const sendSerialLine = console.sendCommandLine;
  const selectedPort = device.selectedPort;

  useEffect(() => {
    if (!queuedCommandRequest || !selectedPort) return;
    clearPicoCommandRequest();
    void executeDiagnosticCommand(queuedCommandRequest.commandId, queuedCommandRequest.arg);
  }, [queuedCommandRequest, selectedPort, executeDiagnosticCommand]);

  useEffect(() => {
    if (!queuedSerialLine || !selectedPort) return;
    clearPicoSerialLine();
    void sendSerialLine(queuedSerialLine.line, selectedPort);
  }, [queuedSerialLine, selectedPort, sendSerialLine]);

  const gdb = useGdbBridge({
    selectedPort: device.selectedPort,
    onLog: console.addLog,
  });

  const erase = useInCircuitErase({
    selectedPort: device.selectedPort,
    onLog: console.addLog,
    onSuccess: useCallback(() => {
      setDeviceStatus((prev) => (prev ? { ...prev, ram_loader_injected: true } : null));
    }, [setDeviceStatus]),
  });

  const [sectionFilter, setSectionFilter] = useState<PicoBenchSectionFilter>('all');
  const showsSection = useCallback(
    (section: Exclude<PicoBenchSectionFilter, 'all'>): boolean =>
      sectionFilter === 'all' || sectionFilter === section,
    [sectionFilter],
  );

  const effectiveDeviceStatus = useMemo(() => {
    if (!device.deviceStatus || !ssot) {
      return device.deviceStatus;
    }
    const effectiveVddVolts = resolveEffectiveVddVolts(
      device.deviceStatus.vdd_volts,
      diagnostics.latestResult?.vdd_volts,
    );
    return {
      ...device.deviceStatus,
      vdd_volts: effectiveVddVolts,
      target_powered:
        isTargetPowered(effectiveVddVolts, ssot.thresholds.min_vdd_threshold_volts) ||
        device.deviceStatus.target_powered,
    };
  }, [device.deviceStatus, diagnostics.latestResult?.vdd_volts, ssot]);

  const handleSendCommand = useCallback(
    () => console.sendCommand(device.selectedPort),
    [console, device.selectedPort],
  );

  return {
    ssot,
    isSsotLoaded,
    isSsotLoading,
    ssotError,
    reloadSsot,

    sectionFilter,
    setSectionFilter,
    showsSection,

    deviceStatus: effectiveDeviceStatus,
    isFlashing: device.isFlashing,
    availablePorts: device.availablePorts,
    selectedPort: device.selectedPort,
    setSelectedPort: device.setSelectedPort,
    transportMode: device.transportMode,
    modelCatalog: device.modelCatalog,
    selectedModel: device.selectedModel,
    selectTargetModel: device.selectTargetModel,
    refreshPorts: device.refreshPorts,
    changeBreakinMode: device.changeBreakinMode,
    toggleConnection: device.toggleConnection,
    triggerBootselReboot: device.triggerBootselReboot,
    flashPicoFirmware: device.flashPicoFirmware,
    refreshStatus: device.pollDeviceStatus,
    isVoltageSafe: device.isVoltageSafe,
    isHardwareReady: device.isHardwareReady,

    diagnosticHistory: diagnostics.history,
    latestDiagnostic: diagnostics.latestResult,
    runningCommandId: diagnostics.runningCommandId,
    isDiagnosticRunning: diagnostics.isExecuting,
    diagnosticError: diagnostics.error,
    diagnosticCategory: diagnostics.selectedCategory,
    setDiagnosticCategory: diagnostics.setSelectedCategory,
    executeDiagnostic: diagnostics.executeCommand,
    clearDiagnosticHistory: diagnostics.clearHistory,

    gdbStatus: gdb.gdbStatus,
    am34Registers: gdb.am34Registers,
    gdbTelemetry: gdb.telemetry,
    isGdbStarting: gdb.isGdbStarting,
    gdbError: gdb.gdbError,
    isAndroidHost: isAndroidNativeBridgeAvailable(),
    startGdbServer: gdb.startGdbServer,
    stopGdbServer: gdb.stopGdbServer,
    launchExternalTool: gdb.launchTool,
    copyGdbCommand: gdb.copyConnectionCommand,

    isErasing: erase.isErasing,
    eraseResult: erase.eraseResult,
    injectRamLoaderAndErase: erase.injectRamLoaderAndErase,
    isBackingUp: erase.isBackingUp,
    backupResult: erase.backupResult,
    executeFirstContactBackup: erase.executeFirstContactBackup,

    logs: console.logs,
    logContainerRef: console.logContainerRef,
    commandInput: console.commandInput,
    setCommandInput: console.setCommandInput,
    sendCommand: handleSendCommand,
    clearLogs: console.clearLogs,
  };
}
