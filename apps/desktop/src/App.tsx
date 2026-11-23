import React, { useCallback, Suspense, lazy } from 'react';
import { Provider } from 'react-redux';
import { motion, MotionConfig } from 'framer-motion';
import { Loader2 } from 'lucide-react';
import { HEADER_VIEW_Y_OFFSET_PX, VIEW_TRANSITION_DURATION_SEC } from './constants';
import { isAndroidCommanderEnabled } from './config/platform';
import { Sidebar, MobileSidebarDrawer, type ViewKey } from './components/Sidebar';
import { Header } from './components/Header';
import { ExpressView } from './views/ExpressView';
import { ErrorBoundary } from './components/ui/error-boundary';
import {
  BottomConsoleDrawer,
  ComplianceDialog,
  DraggableBenchHud,
  GuidedPipelineBar,
  LeftCommandDock,
  RightTelemetryDock,
  WorkbenchStatusBar,
} from './components/ui';
import { ToastProvider } from './context/ToastContext';
import { WorkbenchModeProvider, useWorkbenchMode } from './context/WorkbenchModeContext';
import { useTranslation } from './i18n/useTranslation';
import { store } from './store';
import { useAppDispatch, useAppSelector } from './store/hooks';
import { setCurrentView, setMobileNavOpen } from './store/slices/shellSlice';
import { useHardwareSsot, useShellTelemetryListener } from './viewmodels/useHardwareSsot';
import { useWorkbenchDocksViewModel } from './viewmodels/useWorkbenchDocksViewModel';

const DashboardView = lazy(() => import('./views/DashboardView').then((m) => ({ default: m.DashboardView })));
const NvramView = lazy(() => import('./views/NvramView').then((m) => ({ default: m.NvramView })));
const HddStorageView = lazy(() => import('./views/HddStorageView').then((m) => ({ default: m.HddStorageView })));
const PicoBenchView = lazy(() => import('./views/PicoBenchView').then((m) => ({ default: m.PicoBenchView })));
const FrmCryptoView = lazy(() => import('./views/FrmCryptoView').then((m) => ({ default: m.FrmCryptoView })));
const ManualsView = lazy(() => import('./views/ManualsView').then((m) => ({ default: m.ManualsView })));
const HexInspectorView = lazy(() => import('./views/HexInspectorView').then((m) => ({ default: m.HexInspectorView })));
const UnbrickView = lazy(() => import('./views/UnbrickView').then((m) => ({ default: m.UnbrickView })));
const SettingsView = lazy(() => import('./views/SettingsView').then((m) => ({ default: m.SettingsView })));
const IrCommanderView =
  import.meta.env.MODE === 'android' || !import.meta.env.PROD
    ? lazy(() => import('./views/IrCommanderView').then((m) => ({ default: m.IrCommanderView })))
    : null;

const ViewLoadingFallback: React.FC = () => {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-center justify-center min-h-[400px] h-full gap-3 text-slate-400">
      <Loader2 className="w-8 h-8 animate-spin text-cyan-400" />
      <span className="text-sm font-mono tracking-wide animate-pulse">{t('app.loading')}</span>
    </div>
  );
};

interface ActiveViewSwitchProps {
  readonly currentView: ViewKey;
  readonly onNavigate: (view: ViewKey) => void;
}

const ActiveViewSwitch: React.FC<ActiveViewSwitchProps> = ({ currentView, onNavigate }) => {
  if (IrCommanderView && isAndroidCommanderEnabled() && currentView === 'ircommander') {
    return <IrCommanderView />;
  }
  switch (currentView) {
    case 'dashboard': return <DashboardView onNavigate={onNavigate} />;
    case 'nvram': return <NvramView onNavigate={onNavigate} />;
    case 'hdd': return <HddStorageView onNavigate={onNavigate} />;
    case 'picobench': return <PicoBenchView onNavigate={onNavigate} />;
    case 'frmcrypto': return <FrmCryptoView />;
    case 'manuals': return <ManualsView />;
    case 'hexinspector': return <HexInspectorView />;
    case 'unbrick': return <UnbrickView />;
    case 'settings': return <SettingsView />;
    default: return null;
  }
};

const AppShell: React.FC = () => {
  const { t } = useTranslation();
  const dispatch = useAppDispatch();
  const { ssot, isLoading } = useHardwareSsot();
  const hardwareStatus = useShellTelemetryListener();
  const docksVm = useWorkbenchDocksViewModel();
  const {
    mode, setMode, toggleMode, hudOpen, setHudOpen, toggleHud, pipeline, accessibility,
    complianceModalOpen, acceptCompliance, setComplianceModalOpen,
    docks, toggleLeftDock, toggleRightDock, toggleBottomDock, toggleBottomExpanded,
  } = useWorkbenchMode();
  const currentView = useAppSelector((state) => state.shell.currentView);
  const mobileNavOpen = useAppSelector((state) => state.shell.mobileNavOpen);

  const handleSelectView = useCallback((view: ViewKey) => dispatch(setCurrentView(view)), [dispatch]);
  const handleMobileNavChange = useCallback((open: boolean) => dispatch(setMobileNavOpen(open)), [dispatch]);
  const handleEnterEngineering = useCallback(
    (targetView?: ViewKey) => {
      if (targetView) {
        dispatch(setCurrentView(targetView));
      }
      setMode('engineering');
    },
    [dispatch, setMode],
  );

  const manualsCountLabel = ssot ? `${ssot.indexed_manuals_count} PDFs` : 'SSOT';
  const viewTitles: Partial<Record<ViewKey, string>> = {
    dashboard: t('shell.viewTitles.dashboard'),
    nvram: t('shell.viewTitles.nvram'),
    hdd: t('shell.viewTitles.hdd'),
    picobench: t('shell.viewTitles.picobench'),
    frmcrypto: t('shell.viewTitles.frmcrypto'),
    manuals: t('shell.viewTitles.manuals', { count: manualsCountLabel }),
    hexinspector: t('shell.viewTitles.hexinspector'),
    unbrick: t('shell.viewTitles.unbrick'),
    settings: t('shell.viewTitles.settings'),
    ...(import.meta.env.MODE === 'android' || !import.meta.env.PROD
      ? { ircommander: t('shell.viewTitles.ircommander') }
      : {}),
  };

  const activeViewTitle = viewTitles[currentView] ?? '';
  const minVddThreshold = ssot?.thresholds.min_vdd_threshold_volts;
  const openCompliance = useCallback(() => setComplianceModalOpen(true), [setComplianceModalOpen]);
  const sharedHwProps = {
    hardwareConnected: hardwareStatus.connected,
    vddVolts: hardwareStatus.vdd_volts,
    minVddThreshold,
  };
  const sharedDockCtrlProps = {
    docks,
    onToggleLeftDock: toggleLeftDock,
    onToggleBottomDock: toggleBottomDock,
    onToggleRightDock: toggleRightDock,
    onOpenCompliance: openCompliance,
  };
  const sidebarProps = {
    currentView,
    onSelectView: handleSelectView,
    ...sharedHwProps,
    nvramCleaned: pipeline.nvramCleaned,
    indexedManualsCount: ssot?.indexed_manuals_count ?? null,
  };

  if (mode === 'express') {
    return (
      <MotionConfig reducedMotion={accessibility.reducedMotion ? 'always' : 'user'}>
        <ExpressView onEnterEngineering={handleEnterEngineering} />
        <ComplianceDialog open={complianceModalOpen} onOpenChange={setComplianceModalOpen} onAccept={acceptCompliance} />
      </MotionConfig>
    );
  }

  return (
    <MotionConfig reducedMotion={accessibility.reducedMotion ? 'always' : 'user'}>
      <div
        data-testid="engineering-shell"
        className="flex flex-col h-dvh w-screen overflow-hidden bg-bench-950 text-slate-100 font-sans"
      >
        <Header
          currentView={currentView}
          currentViewTitle={activeViewTitle}
          mode={mode}
          onToggleMode={toggleMode}
          onOpenMobileNav={() => handleMobileNavChange(true)}
          onOpenSettings={() => handleSelectView('settings')}
          {...sharedHwProps}
          {...sharedDockCtrlProps}
        />

        <Sidebar {...sidebarProps} />
        <MobileSidebarDrawer open={mobileNavOpen} onOpenChange={handleMobileNavChange} {...sidebarProps} />

        <div className="flex-1 min-h-0 flex flex-row overflow-hidden relative">
          <LeftCommandDock
            open={docks.leftOpen}
            onClose={toggleLeftDock}
            onRunCommand={docksVm.runQuickCommand}
            onOpenPicoBench={() => handleSelectView('picobench')}
            runningCommandId={docksVm.runningCommandId}
            hardwareConnected={hardwareStatus.connected}
            breakinMode={docksVm.breakinMode}
            hasSsotCommands={Boolean(ssot?.pico_commands.length)}
          />

          <main className="flex-1 min-w-0 min-h-0 overflow-y-auto overflow-x-hidden p-3 sm:p-6 scrollbar-thin scrollbar-thumb-slate-800">
            <div className="max-w-7xl mx-auto">
              {isLoading && !ssot && (
                <div className="mb-3 flex items-center gap-2 text-xs font-mono text-cyan-400">
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  <span>{t('shell.syncingSsot')}</span>
                </div>
              )}
              <GuidedPipelineBar
                currentView={currentView}
                onNavigate={handleSelectView}
                mode={mode}
                onModeChange={setMode}
                hudOpen={hudOpen}
                onToggleHud={toggleHud}
                pipeline={pipeline}
                hardwareConnected={hardwareStatus.connected}
                vddVolts={hardwareStatus.vdd_volts}
                runningCommandId={docksVm.runningCommandId}
              />
            </div>

            <ErrorBoundary fallbackTitle={t('shell.boundaryFailure', { title: activeViewTitle })}>
              <Suspense fallback={<ViewLoadingFallback />}>
                <motion.div
                  key={currentView}
                  initial={{ opacity: 0, y: HEADER_VIEW_Y_OFFSET_PX }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: VIEW_TRANSITION_DURATION_SEC }}
                >
                  <ActiveViewSwitch currentView={currentView} onNavigate={handleSelectView} />
                </motion.div>
              </Suspense>
            </ErrorBoundary>
          </main>

          <RightTelemetryDock
            open={docks.rightOpen}
            onClose={toggleRightDock}
            {...sharedHwProps}
            portName={docksVm.selectedPort}
            modelId={docksVm.selectedModel}
            transportMode={docksVm.transportMode}
            gdbRunning={docksVm.gdbRunning}
            gdbListenAddr={docksVm.gdbListenAddr}
            sc0Running={docksVm.sc0Active}
            latestDiagnostic={docksVm.latestDiagnostic}
          />
        </div>

        <BottomConsoleDrawer
          open={docks.bottomOpen}
          expanded={docks.bottomExpanded}
          onToggleExpand={toggleBottomExpanded}
          onClose={toggleBottomDock}
          logs={docksVm.serialLogs}
          onClearLogs={docksVm.clearQuickLogs}
          commandInput={docksVm.consoleInput}
          onCommandInputChange={docksVm.setConsoleInput}
          onSendCommand={docksVm.submitConsoleInput}
          hardwareConnected={hardwareStatus.connected}
        />

        <WorkbenchStatusBar
          currentStageLabel={activeViewTitle}
          {...sharedHwProps}
          portName={docksVm.selectedPort}
          runningCommandId={docksVm.runningCommandId}
          logCount={docksVm.serialLogs.length}
          {...sharedDockCtrlProps}
        />

        <ComplianceDialog open={complianceModalOpen} onOpenChange={setComplianceModalOpen} onAccept={acceptCompliance} />
        <DraggableBenchHud
          vddVolts={hardwareStatus.vdd_volts}
          hardwareConnected={hardwareStatus.connected}
          hudOpen={hudOpen}
          onCloseHud={() => setHudOpen(false)}
          onNavigate={handleSelectView}
        />
      </div>
    </MotionConfig>
  );
};

export const App: React.FC = () => (
  <Provider store={store}>
    <WorkbenchModeProvider>
      <ToastProvider>
        <AppShell />
      </ToastProvider>
    </WorkbenchModeProvider>
  </Provider>
);

export default App;
