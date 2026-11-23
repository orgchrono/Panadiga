import React, { useState } from 'react';
import { Cpu, Disc3, FolderArchive, Radio } from 'lucide-react';
import {
  ExpressHeader,
  NorDumpPatchMission,
  PicoConnectionSafetySheet,
  SdRecoveryMission,
  StrictVaultPipelineMission,
  StrictWorkflowStepper,
  U99QuickRescueMission,
} from '@/components/express';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { ViewKey } from '@/models';
import { useExpressViewModel } from '@/viewmodels/useExpressViewModel';
import { useStrictVaultViewModel } from '@/viewmodels/useStrictVaultViewModel';

export interface ExpressViewProps {
  readonly onEnterEngineering: (view?: ViewKey) => void;
}

export const ExpressView: React.FC<ExpressViewProps> = ({ onEnterEngineering }) => {
  const { t } = useTranslation();
  const vm = useExpressViewModel(view => onEnterEngineering(view));
  const vaultVm = useStrictVaultViewModel(vm.selectedModel);
  const [activeTab, setActiveTab] = useState('mission-sd');
  const effectiveGateReport = vaultVm.emuGateReport ?? vm.sdReport?.emu_gate ?? vm.u99Report?.emuGate ?? null;

  return (
    <div
      data-testid="express-shell"
      className="flex flex-col h-dvh w-screen overflow-hidden bg-bench-950 text-slate-100 font-sans"
    >
      <ExpressHeader onEnterEngineering={() => onEnterEngineering()} />

      <main className="flex-1 min-h-0 overflow-y-auto p-4 sm:p-6">
        <div className="max-w-6xl mx-auto space-y-5">
          <section className="rounded-2xl border border-slate-800 bg-bench-900/75 p-5 space-y-1.5 shadow-lg">
            <h2 className="text-lg font-bold font-mono text-slate-100">
              {t('express.hero.title')}
            </h2>
            <p className="text-xs sm:text-sm text-slate-400 max-w-3xl">
              {t('express.hero.subtitle')}
            </p>
          </section>

          <StrictWorkflowStepper
            activeSession={vaultVm.activeSession}
            emuGateReport={effectiveGateReport}
            onJumpToVault={() => setActiveTab('mission-vault')}
          />

          <Tabs value={activeTab} onValueChange={setActiveTab} className="space-y-4">
            <TabsList
              aria-label={t('express.tabs.tablistAria')}
              className="flex flex-wrap h-auto gap-1.5 p-1.5 bg-bench-900 border border-slate-800 rounded-xl"
            >
              <TabsTrigger
                value="mission-sd"
                data-testid="express-tab-sd"
                className="gap-1.5 font-mono text-xs"
              >
                <Disc3 className="w-3.5 h-3.5 text-emerald-400" />
                <span>{t('express.tabs.mission1')}</span>
              </TabsTrigger>
              <TabsTrigger
                value="mission-u99"
                data-testid="express-tab-u99"
                className="gap-1.5 font-mono text-xs"
              >
                <Radio className="w-3.5 h-3.5 text-cyan-400" />
                <span>{t('express.tabs.mission2')}</span>
              </TabsTrigger>
              <TabsTrigger
                value="mission-nor"
                data-testid="express-tab-nor"
                className="gap-1.5 font-mono text-xs"
              >
                <Cpu className="w-3.5 h-3.5 text-amber-400" />
                <span>{t('express.tabs.mission3')}</span>
              </TabsTrigger>
              <TabsTrigger
                value="mission-vault"
                data-testid="express-tab-vault"
                className="gap-1.5 font-mono text-xs"
              >
                <FolderArchive className="w-3.5 h-3.5 text-emerald-400" />
                <span>{t('express.tabs.mission4Vault')}</span>
              </TabsTrigger>
            </TabsList>

            <TabsContent value="mission-sd">
              <SdRecoveryMission
                modelCatalog={vm.modelCatalog}
                selectedModel={vm.selectedModel}
                onSelectModel={vm.setSelectedModel}
                includePanaSd={vm.includePanaSd}
                onToggleIncludePanaSd={vm.setIncludePanaSd}
                sdTargets={vm.sdTargets}
                sdTargetsLoading={vm.sdTargetsLoading}
                sdTargetMode={vm.sdTargetMode}
                onSelectTargetMode={vm.setSdTargetMode}
                selectedDevicePath={vm.selectedDevicePath}
                onSelectDevicePath={vm.setSelectedDevicePath}
                selectedSdDrive={vm.selectedSdDrive}
                sdConfirmToken={vm.sdConfirmToken}
                onChangeConfirmToken={vm.setSdConfirmToken}
                sdFlashing={vm.sdFlashing}
                sdProgress={vm.sdProgress}
                sdError={vm.sdError}
                sdReport={vm.sdReport}
                onRefreshTargets={() => void vm.refreshSdTargets()}
                onExecuteFlash={() => void vm.executeSdFlash()}
                onOpenEngineering={() => vm.openInEngineering('unbrick')}
              />
            </TabsContent>

            <TabsContent value="mission-u99">
              <U99QuickRescueMission
                picoPorts={vm.picoPorts}
                picoPortsLoading={vm.picoPortsLoading}
                picoPortInput={vm.picoPortInput}
                onChangePortInput={vm.setPicoPortInput}
                onRefreshPorts={() => void vm.refreshPicoPorts()}
                u99Cleaning={vm.u99Cleaning}
                u99Progress={vm.u99Progress}
                u99Error={vm.u99Error}
                u99Report={vm.u99Report}
                onExecuteRescue={vm.executeU99QuickRescue}
                onOpenEngineering={() => vm.openInEngineering('nvram')}
              />
            </TabsContent>

            <TabsContent value="mission-nor">
              <NorDumpPatchMission
                norSourceMode={vm.norSourceMode}
                onSelectSourceMode={vm.setNorSourceMode}
                norCapturing={vm.norCapturing}
                norProgress={vm.norProgress}
                norDumpReport={vm.norDumpReport}
                norAuditing={vm.norAuditing}
                norAuditNvram={vm.norAuditNvram}
                norAuditBgo={vm.norAuditBgo}
                norPatching={vm.norPatching}
                norRepairResult={vm.norRepairResult}
                norError={vm.norError}
                onExecuteInspect={vm.executeNorInspect}
                onExecuteSafeRepair={live => void vm.executeNorSafeRepair(live)}
                onOpenEngineering={() => vm.openInEngineering('picobench')}
              />
            </TabsContent>

            <TabsContent value="mission-vault">
              <StrictVaultPipelineMission
                modelCatalog={vm.modelCatalog}
                selectedModel={vm.selectedModel}
                onSelectModel={vm.setSelectedModel}
                vm={vaultVm}
              />
            </TabsContent>
          </Tabs>
        </div>
      </main>

      <PicoConnectionSafetySheet
        open={vm.safetySheetOpen}
        onCancel={vm.cancelPicoSafetySheet}
        onConfirm={vm.confirmPicoSafetySheet}
      />
    </div>
  );
};
