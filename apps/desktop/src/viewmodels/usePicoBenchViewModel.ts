import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { PicoBenchSectionFilter, PicoDiagnosticResult } from '@/models';
import { isAndroidNativeBridgeAvailable } from '@/services/androidBridge';
import { isTargetPowered, resolveEffectiveVddVolts } from '@/utils/formatters';
import { useHardwareSsot } from './useHardwareSsot';
import { usePicoDevice } from './usePicoDevice';
import { usePicoDiagnostics } from './usePicoDiagnostics';
import { useGdbBridge } from './useGdbBridge';
import { useInCircuitErase } from './useInCircuitErase';
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
  const telemetryRef = useRef<((result: PicoDiagnosticResult) => void) | undefined>(undefined);
  const console = useSerialConsole({
    onCommandTelemetry: useCallback((result: PicoDiagnosticResult) => {
      telemetryRef.current?.(result);
    }, []),
  });

  const device = usePicoDevice({
    onLog: console.addLog,
  });
  const { setDeviceStatus, applyMeasuredVoltage } = device;

  const handleDiagnosticTelemetry = useCallback(
    (result: PicoDiagnosticResult) => {
      if (result.vdd_volts !== null && result.vdd_volts !== undefined) {
        applyMeasuredVoltage(result.vdd_volts, result.command_id, result.success);
      }
    },
    [applyMeasuredVoltage]
  );
  useEffect(() => {
    telemetryRef.current = handleDiagnosticTelemetry;
  }, [handleDiagnosticTelemetry]);

  const diagnostics = usePicoDiagnostics({
    selectedPort: device.selectedPort,
    ssot,
    onLog: console.addLog,
    onTelemetryUpdate: handleDiagnosticTelemetry,
  });

  const gdb = useGdbBridge({
    selectedPort: device.selectedPort,
    onLog: console.addLog,
  });

  const erase = useInCircuitErase({
    selectedPort: device.selectedPort,
    onLog: console.addLog,
    onSuccess: useCallback(() => {
      setDeviceStatus(prev => (prev ? { ...prev, ram_loader_injected: true } : null));
    }, [setDeviceStatus]),
  });

  const [sectionFilter, setSectionFilter] = useState<PicoBenchSectionFilter>('all');
  const showsSection = useCallback(
    (section: Exclude<PicoBenchSectionFilter, 'all'>): boolean =>
      sectionFilter === 'all' || sectionFilter === section,
    [sectionFilter]
  );

  const effectiveDeviceStatus = useMemo(() => {
    if (!device.deviceStatus || !ssot) {
      return device.deviceStatus;
    }
    const effectiveVddVolts = resolveEffectiveVddVolts(
      device.deviceStatus.vdd_volts,
      diagnostics.latestResult?.vdd_volts
    );
    return {
      ...device.deviceStatus,
      vdd_volts: effectiveVddVolts,
      target_powered:
        isTargetPowered(effectiveVddVolts, ssot.thresholds.min_vdd_threshold_volts) ||
        device.deviceStatus.target_powered,
    };
  }, [device.deviceStatus, diagnostics.latestResult?.vdd_volts, ssot]);

  const handleSendCommand = useCallback(() => {
    return console.sendCommand(device.selectedPort);
  }, [console, device.selectedPort]);

  return {
    // Hardware/Firmware SSOT Spec
    ssot,
    isSsotLoaded,
    isSsotLoading,
    ssotError,
    reloadSsot,

    // Section Filter & Derived Effective Rail Telemetry (MVVM Strict)
    sectionFilter,
    setSectionFilter,
    showsSection,

    // Device & Connection Subsystem
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

    // Granular Pico Diagnostic Studio
    diagnosticHistory: diagnostics.history,
    latestDiagnostic: diagnostics.latestResult,
    runningCommandId: diagnostics.runningCommandId,
    isDiagnosticRunning: diagnostics.isExecuting,
    diagnosticError: diagnostics.error,
    diagnosticCategory: diagnostics.selectedCategory,
    setDiagnosticCategory: diagnostics.setSelectedCategory,
    executeDiagnostic: diagnostics.executeCommand,
    clearDiagnosticHistory: diagnostics.clearHistory,

    // GDB Remote RSP Server Subsystem
    gdbStatus: gdb.gdbStatus,
    am34Registers: gdb.am34Registers,
    gdbTelemetry: gdb.telemetry,
    isGdbStarting: gdb.isGdbStarting,
    gdbError: gdb.gdbError,
    isAndroidHost: isAndroidNativeBridgeAvailable(),
    startGdbServer: gdb.startGdbServer,
    stopGdbServer: gdb.stopGdbServer,
    launchExternalTool: gdb.launchTool,

    // In-Circuit NVRAM Erase Subsystem
    isErasing: erase.isErasing,
    eraseResult: erase.eraseResult,
    injectRamLoaderAndErase: erase.injectRamLoaderAndErase,
    isBackingUp: erase.isBackingUp,
    backupResult: erase.backupResult,
    executeFirstContactBackup: erase.executeFirstContactBackup,

    // Interactive Serial Console Subsystem
    logs: console.logs,
    logContainerRef: console.logContainerRef,
    commandInput: console.commandInput,
    setCommandInput: console.setCommandInput,
    sendCommand: handleSendCommand,
    clearLogs: console.clearLogs,
  };
}
