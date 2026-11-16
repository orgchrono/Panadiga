import React, { useCallback, Suspense, lazy } from 'react';
import { Provider } from 'react-redux';
import { motion, MotionConfig } from 'framer-motion';
import { Sidebar, MobileSidebarDrawer, ViewKey } from './components/Sidebar';
import { Header } from './components/Header';
import { useHardwareSsot, useShellTelemetryListener } from './viewmodels/useHardwareSsot';
import { ToastProvider } from './context/ToastContext';
import { WorkbenchModeProvider, useWorkbenchMode } from './context/WorkbenchModeContext';
import { ErrorBoundary } from './components/ui/error-boundary';
import { DraggableBenchHud, GuidedPipelineBar } from './components/ui';
import { store } from './store';
import { useAppDispatch, useAppSelector } from './store/hooks';
import { setCurrentView, setMobileNavOpen } from './store/slices/shellSlice';
import { Loader2 } from 'lucide-react';

const DashboardView = lazy(() =>
  import('./views/DashboardView').then(m => ({ default: m.DashboardView }))
);
const NvramView = lazy(() =>
  import('./views/NvramView').then(m => ({ default: m.NvramView }))
);
const HddStorageView = lazy(() =>
  import('./views/HddStorageView').then(m => ({ default: m.HddStorageView }))
);
const PicoBenchView = lazy(() =>
  import('./views/PicoBenchView').then(m => ({ default: m.PicoBenchView }))
);
const FrmCryptoView = lazy(() =>
  import('./views/FrmCryptoView').then(m => ({ default: m.FrmCryptoView }))
);
const ManualsView = lazy(() =>
  import('./views/ManualsView').then(m => ({ default: m.ManualsView }))
);
const HexInspectorView = lazy(() =>
  import('./views/HexInspectorView').then(m => ({ default: m.HexInspectorView }))
);
const UnbrickView = lazy(() =>
  import('./views/UnbrickView').then(m => ({ default: m.UnbrickView }))
);
const SettingsView = lazy(() =>
  import('./views/SettingsView').then(m => ({ default: m.SettingsView }))
);

const ViewLoadingFallback: React.FC = () => (
  <div className="flex flex-col items-center justify-center min-h-[400px] h-full gap-3 text-slate-400">
    <Loader2 className="w-8 h-8 animate-spin text-cyan-400" />
    <span className="text-sm font-mono tracking-wide animate-pulse">
      Carregando subsistema de bancada...
    </span>
  </div>
);

const AppShell: React.FC = () => {
  const dispatch = useAppDispatch();
  const { ssot, isLoading } = useHardwareSsot();
  const hardwareStatus = useShellTelemetryListener();
  const { pipeline, accessibility } = useWorkbenchMode();
  const currentView = useAppSelector(state => state.shell.currentView);
  const mobileNavOpen = useAppSelector(state => state.shell.mobileNavOpen);

  const handleSelectView = useCallback(
    (view: ViewKey) => dispatch(setCurrentView(view)),
    [dispatch],
  );
  const handleMobileNavChange = useCallback(
    (open: boolean) => dispatch(setMobileNavOpen(open)),
    [dispatch],
  );

  const viewTitles: Record<ViewKey, string> = {
    dashboard: 'Visão Geral do Gravador & Status da Bancada',
    nvram: 'Flash Spansion S29JL064H & Cirurgia NVRAM U99',
    hdd: 'Armazenamento MEIHDFS & Forensic do Disco Rígido',
    picobench: 'Bancada Raspberry Pi Pico — P7501 UART (115200 8N1)',
    frmcrypto: 'Criptoanálise Óptica PANA_DVD.FRM & Dicionário ECB',
    manuals: `Manuais de Serviço & Diagramas Esquemáticos (${ssot ? `${ssot.indexed_manuals_count} PDFs` : 'SSOT'})`,
    hexinspector: 'Inspetor Hexadecimal & Alinhamento LBA',
    unbrick: 'Centro Unbrick Pre-MAIN, Kit SD & TBC Autônomo',
    settings: 'Configurações, Temas, Acessibilidade & Idioma',
  };

  return (
    <MotionConfig reducedMotion={accessibility.reducedMotion ? 'always' : 'user'}>
      <div className="flex h-screen w-screen overflow-hidden bg-bench-950 text-slate-100 font-sans">
      <Sidebar
        currentView={currentView}
        onSelectView={handleSelectView}
        vddVolts={hardwareStatus.vdd_volts}
        hardwareConnected={hardwareStatus.connected}
        nvramCleaned={pipeline.nvramCleaned}
        indexedManualsCount={ssot?.indexed_manuals_count ?? null}
      />

      <MobileSidebarDrawer
        open={mobileNavOpen}
        onOpenChange={handleMobileNavChange}
        currentView={currentView}
        onSelectView={handleSelectView}
        vddVolts={hardwareStatus.vdd_volts}
        hardwareConnected={hardwareStatus.connected}
        nvramCleaned={pipeline.nvramCleaned}
        indexedManualsCount={ssot?.indexed_manuals_count ?? null}
      />

      <div className="flex-1 flex flex-col min-w-0 h-full overflow-hidden">
        <Header
          currentViewTitle={viewTitles[currentView]}
          onOpenMobileNav={() => handleMobileNavChange(true)}
          onOpenSettings={() => handleSelectView('settings')}
          hardwareConnected={hardwareStatus.connected}
          vddVolts={hardwareStatus.vdd_volts}
        />

        <main className="flex-1 overflow-y-auto overflow-x-hidden p-3 sm:p-6 scrollbar-thin scrollbar-thumb-slate-800">
          <div className="max-w-7xl mx-auto">
            {isLoading && !ssot && (
              <div className="mb-3 flex items-center gap-2 text-xs font-mono text-cyan-400">
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>Sincronizando especificação SSOT do hardware...</span>
              </div>
            )}
            <GuidedPipelineBar currentView={currentView} onNavigate={handleSelectView} />
          </div>

          <ErrorBoundary fallbackTitle={`Falha ao carregar ${viewTitles[currentView]}`}>
            <Suspense fallback={<ViewLoadingFallback />}>
              <motion.div
                key={currentView}
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.16 }}
              >
                {currentView === 'dashboard' && <DashboardView onNavigate={handleSelectView} />}
                {currentView === 'nvram' && <NvramView onNavigate={handleSelectView} />}
                {currentView === 'hdd' && <HddStorageView onNavigate={handleSelectView} />}
                {currentView === 'picobench' && <PicoBenchView />}
                {currentView === 'frmcrypto' && <FrmCryptoView />}
                {currentView === 'manuals' && <ManualsView />}
                {currentView === 'hexinspector' && <HexInspectorView />}
                {currentView === 'unbrick' && <UnbrickView />}
                {currentView === 'settings' && <SettingsView />}
              </motion.div>
            </Suspense>
          </ErrorBoundary>
        </main>
      </div>

      <DraggableBenchHud
        vddVolts={hardwareStatus.vdd_volts}
        hardwareConnected={hardwareStatus.connected}
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
