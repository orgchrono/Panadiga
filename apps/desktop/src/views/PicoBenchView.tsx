import React from 'react';
import { usePicoBenchViewModel } from '@/viewmodels/usePicoBenchViewModel';
import { useSc0Bridge } from '@/viewmodels/useSc0Bridge';
import { TerminalView } from '@/components/TerminalView';
import { Button, ResizableSplitPane } from '@/components/ui';
import {
  PicoBenchTopBar,
  PicoBenchMetrics,
  PicoBenchSsotLoading,
  PicoBenchSsotError,
  PicoBenchActionBanners,
  PicoBenchSectionToolbar,
  GdbServerCard,
  ColdBreakinSelector,
  P7501PinoutCard,
  PicoDiagnosticsStudio,
  Sc0BridgePanel,
} from '@/components/pico';
import { useTranslation } from '@/i18n/useTranslation';
import { Send } from 'lucide-react';

/**
 * MVVM View: Panasonic P7501 Async UART Hardware Bench Controller.
 * Composes TopBar, Telemetry Metrics, Granular Diagnostics Studio (Radix Tabs),
 * SC0 Bridge, GDB Server Card, Break-In Modes / Pinout, and Live Serial Console.
 */
export const PicoBenchView: React.FC = () => {
  const { t } = useTranslation();
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

  if (isSsotLoading && !ssot) {
    return <PicoBenchSsotLoading />;
  }

  if (!ssot) {
    return <PicoBenchSsotError ssotError={ssotError} onReload={reloadSsot} />;
  }

  return (
    <div className="space-y-6 max-w-7xl mx-auto pb-12 font-mono">
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

      {showsSection('bridge') && (
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

      {showsSection('gdb') && (
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
        />
      )}

      {showsSection('terminal') && (
        <ResizableSplitPane
          initialLeftPercent={42}
          minLeftPercent={28}
          maxLeftPercent={62}
          storageKey="panadiga.split.picobench"
          ariaLabel="Divisor entre pinagem P7501 e terminal serial"
          left={
            <div className="flex flex-col space-y-4 lg:min-h-[720px]">
              <ColdBreakinSelector
                currentMode={deviceStatus?.breakin_mode ?? 2}
                onSelectMode={changeBreakinMode}
                breakinModes={ssot.breakin_modes}
              />
              <div className="flex-1 min-h-0 flex flex-col">
                <P7501PinoutCard ssot={ssot} />
              </div>
            </div>
          }
          right={
            <div className="flex flex-col space-y-3 min-h-[420px] lg:min-h-[720px]">
              <div className="flex-1 min-h-0 flex flex-col">
                <TerminalView logs={logs} onClear={clearLogs} containerRef={logContainerRef} />
              </div>

              <div className="flex items-center gap-2 p-2 rounded-xl bg-bench-900 border border-slate-800 shrink-0">
                <span className="text-slate-500 text-xs font-mono pl-2">{'>'}</span>
                <input
                  type="text"
                  aria-label={t('pico.title')}
                  placeholder="Digite comando CDC (STATUS, RESET, ADC, PINCHECK, DUMP, GDB)..."
                  value={commandInput}
                  onChange={e => setCommandInput(e.target.value)}
                  onKeyDown={e => e.key === 'Enter' && sendCommand()}
                  className="flex-1 bg-transparent border-none text-xs text-slate-100 placeholder-slate-500 focus:outline-none font-mono"
                />
                <Button
                  type="button"
                  variant="default"
                  size="sm"
                  onClick={sendCommand}
                  className="font-bold text-xs gap-1.5"
                >
                  <Send className="w-3.5 h-3.5" />
                  <span>Enviar</span>
                </Button>
              </div>
            </div>
          }
        />
      )}
    </div>
  );
};
