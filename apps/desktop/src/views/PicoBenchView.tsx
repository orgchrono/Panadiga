import React from 'react';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import { useTranslation } from '@/i18n/useTranslation';
import type { ViewKey } from '@/models';
import { usePicoBgoDavisViewModel } from '@/viewmodels/pipelines';
import { usePicoBenchViewModel } from '@/viewmodels/usePicoBenchViewModel';
import { useSc0Bridge } from '@/viewmodels/useSc0Bridge';
import {
  PicoBenchTopBar,
  PicoBenchMetrics,
  PicoBenchSsotLoading,
  PicoBenchSsotError,
  PicoBenchActionBanners,
  PicoBenchSectionToolbar,
  GdbServerCard,
  PicoDiagnosticsStudio,
  Sc0BridgePanel,
  BgoFtlBenchPanel,
  DavisIdeProbePanel,
  PicoBenchTerminalSplit,
} from '@/components/pico';

/**
 * MVVM View: Panasonic P7501 Async UART Hardware Bench Controller.
 * Composes TopBar, Telemetry Metrics, Granular Diagnostics Studio (Radix Tabs),
 * SC0 Bridge, GDB Server Card, BGO FTL + DAVIS Probe panels, Break-In Modes / Pinout, and Live Serial Console.
 */
export const PicoBenchView: React.FC<{ onNavigate?: (view: ViewKey) => void }> = ({
  onNavigate,
}) => {
  const { t } = useTranslation();
  const { isAdvanced } = useWorkbenchMode();
  const {
    ssot,
    isSsotLoading,
    ssotError,
    reloadSsot,
    sectionFilter,
    setSectionFilter,
    showsSection,
    deviceStatus,
    isFlashing,
    availablePorts,
    selectedPort,
    setSelectedPort,
    transportMode,
    modelCatalog,
    selectedModel,
    selectTargetModel,
    refreshPorts,
    refreshStatus,
    diagnosticHistory,
    latestDiagnostic,
    runningCommandId,
    diagnosticError,
    diagnosticCategory,
    setDiagnosticCategory,
    executeDiagnostic,
    clearDiagnosticHistory,
    logs,
    logContainerRef,
    isErasing,
    eraseResult,
    isBackingUp,
    backupResult,
    commandInput,
    setCommandInput,
    gdbStatus,
    am34Registers,
    gdbTelemetry,
    isGdbStarting,
    gdbError,
    isAndroidHost,
    startGdbServer,
    stopGdbServer,
    launchExternalTool,
    copyGdbCommand,
    changeBreakinMode,
    toggleConnection,
    triggerBootselReboot,
    flashPicoFirmware,
    injectRamLoaderAndErase,
    executeFirstContactBackup,
    sendCommand,
    clearLogs,
  } = usePicoBenchViewModel();

  const sc0Bridge = useSc0Bridge({
    selectedPort,
    onExecuteDiagnostic: executeDiagnostic,
  });

  const { bgo, davis } = usePicoBgoDavisViewModel(selectedPort, () => onNavigate?.('hdd'));

  if (isSsotLoading && !ssot) {
    return <PicoBenchSsotLoading />;
  }

  if (!ssot) {
    return <PicoBenchSsotError ssotError={ssotError} onReload={reloadSsot} />;
  }

  return (
    <div
      className="space-y-6 max-w-7xl mx-auto pb-12 font-mono"
      aria-label={t('pico.title')}
    >
      <PicoBenchActionBanners backupResult={backupResult} eraseResult={eraseResult} />

      <PicoBenchTopBar
        deviceStatus={deviceStatus}
        latestDiagnostic={latestDiagnostic}
        availablePorts={availablePorts}
        selectedPort={selectedPort}
        transportMode={transportMode}
        modelCatalog={modelCatalog}
        selectedModel={selectedModel}
        isErasing={isErasing}
        isBackingUp={isBackingUp}
        isFlashing={isFlashing}
        onSelectPort={setSelectedPort}
        onSelectModel={selectTargetModel}
        onToggleConnection={toggleConnection}
        onInjectAndErase={injectRamLoaderAndErase}
        onFirstContactBackup={executeFirstContactBackup}
        onRefreshPorts={refreshPorts}
        onRefreshStatus={refreshStatus}
        onQuickPincheck={() => {
          void executeDiagnostic('PINCHECK');
        }}
        onBootselReboot={triggerBootselReboot}
        onFlashFirmware={flashPicoFirmware}
      />

      <PicoBenchMetrics
        deviceStatus={deviceStatus}
        ssot={ssot}
        detectedFlashMb={latestDiagnostic?.detected_flash_mb}
      />

      <PicoBenchSectionToolbar sectionFilter={sectionFilter} onSelectFilter={setSectionFilter} />

      {showsSection('diag') && (
        <PicoDiagnosticsStudio
          ssot={ssot}
          latestResult={latestDiagnostic}
          history={diagnosticHistory}
          runningCommandId={runningCommandId}
          error={diagnosticError}
          selectedCategory={diagnosticCategory}
          onSelectCategory={setDiagnosticCategory}
          onExecute={executeDiagnostic}
          onClearHistory={clearDiagnosticHistory}
        />
      )}

      {showsSection('bridge') && (isAdvanced || sectionFilter === 'bridge') && (
        <Sc0BridgePanel
          status={sc0Bridge.bridgeStatus}
          logs={sc0Bridge.bridgeLogs}
          baudRate={sc0Bridge.baudRate}
          onChangeBaudRate={sc0Bridge.setBaudRate}
          bootMode={sc0Bridge.bootMode}
          onChangeBootMode={sc0Bridge.setBootMode}
          inputMode={sc0Bridge.inputMode}
          onChangeInputMode={sc0Bridge.setInputMode}
          txInput={sc0Bridge.txInput}
          onChangeTxInput={sc0Bridge.setTxInput}
          isBusy={sc0Bridge.isBusy}
          error={sc0Bridge.error}
          onStart={() => {
            void sc0Bridge.startBridge();
          }}
          onStop={() => {
            void sc0Bridge.stopBridge();
          }}
          onSubmitTx={() => {
            void sc0Bridge.submitTxInput();
          }}
          onClearLogs={sc0Bridge.clearBridgeLogs}
          onTriggerWaittx={spec => {
            void sc0Bridge.triggerInteractiveWaittx(spec);
          }}
          onSendRspPreset={(mode, bridgePayload, panarspArg) => {
            void sc0Bridge.sendPanaRspPreset(mode, bridgePayload, panarspArg);
          }}
          onSendDriveCommand={cmd => {
            void sc0Bridge.sendDriveCommand(cmd);
          }}
          onRunDiagnostic={(command, arg) => {
            void executeDiagnostic(command, arg);
          }}
        />
      )}

      {showsSection('gdb') && (isAdvanced || sectionFilter === 'gdb') && (
        <GdbServerCard
          gdbStatus={gdbStatus}
          registers={am34Registers}
          telemetry={gdbTelemetry}
          isStarting={isGdbStarting}
          error={gdbError}
          isAndroidHost={isAndroidHost}
          onStart={startGdbServer}
          onStop={stopGdbServer}
          onLaunchTool={launchExternalTool}
          onCopyCommand={copyGdbCommand}
        />
      )}

      {showsSection('bgo') && (isAdvanced || sectionFilter === 'bgo') && (
        <BgoFtlBenchPanel vm={bgo} />
      )}

      {showsSection('davis') && (isAdvanced || sectionFilter === 'davis') && (
        <DavisIdeProbePanel vm={davis} />
      )}

      {showsSection('terminal') && (
        <PicoBenchTerminalSplit
          ssot={ssot}
          currentMode={deviceStatus?.breakin_mode ?? 2}
          onSelectMode={changeBreakinMode}
          logs={logs}
          clearLogs={clearLogs}
          logContainerRef={logContainerRef}
          commandInput={commandInput}
          setCommandInput={setCommandInput}
          sendCommand={sendCommand}
        />
      )}
    </div>
  );
};
