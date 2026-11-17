import React, { useState } from 'react';
import { Radio, ShieldAlert } from 'lucide-react';
import { useToast } from '@/context/ToastContext';
import { useTranslation } from '@/i18n/useTranslation';
import { useIrCommanderViewModel } from '@/viewmodels/useIrCommanderViewModel';
import {
  Alert,
  AlertDescription,
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui';
import { IrVfdBanner } from '@/components/ir/IrVfdBanner';
import { IrRemoteTab } from '@/components/ir/IrRemoteTab';
import { IrCatalogKeymapTab } from '@/components/ir/IrCatalogKeymapTab';
import { IrServiceRecoveryTab } from '@/components/ir/IrServiceRecoveryTab';
import { IrDiagnosticsTab } from '@/components/ir/IrDiagnosticsTab';
import { IrKeyEditorDialog } from '@/components/ir/IrKeyEditorDialog';
import type { DynamicRemoteKey } from '@/models/ir';
import { oneOfGuard } from '@/utils/typeGuards';

const isIrTab = oneOfGuard(['remote', 'catalog', 'service', 'diagnostics'] as const);

export const IrCommanderView: React.FC = () => {
  const { t } = useTranslation();
  const { showToast: toast } = useToast();
  const vm = useIrCommanderViewModel();
  const [activeTab, setActiveTab] = useState<'remote' | 'catalog' | 'service' | 'diagnostics'>('remote');
  const [pendingConfirmKey, setPendingConfirmKey] = useState<DynamicRemoteKey | null>(null);
  const [successFeedback, setSuccessFeedback] = useState<string | null>(null);

  const onRequestTriggerKey = (keyItem: DynamicRemoteKey) => {
    if (keyItem.confirm_required) {
      setPendingConfirmKey(keyItem);
      return;
    }
    void vm.triggerKey(keyItem);
  };

  const confirmProtectedTransmission = () => {
    if (!pendingConfirmKey) return;
    const confirmed = pendingConfirmKey;
    setPendingConfirmKey(null);
    setSuccessFeedback(confirmed.label);
    toast({ type: 'success', title: confirmed.label, message: t('irCommander.remote.confirmKeyBtn') });
    void vm.triggerKey(confirmed);
  };

  return (
    <div className="space-y-4 max-w-5xl mx-auto pb-14 font-mono" data-testid="ir-commander-view">
      <div className="rounded-2xl bg-bench-900 border border-slate-800 p-4 shadow-xl flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="p-2.5 rounded-xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/30">
            <Radio className="w-5 h-5" />
          </div>
          <div>
            <h2 className="text-base font-bold text-slate-100">{t('irCommander.title')}</h2>
            <p className="text-xs text-slate-400 font-sans mt-0.5">{t('irCommander.subtitle')}</p>
          </div>
        </div>
      </div>

      <IrVfdBanner
        vfdPrimary={vm.state.vfdPrimary}
        vfdSecondary={vm.state.vfdSecondary}
        lastWireHex={vm.state.lastWireHex}
        lastBackendUsed={vm.state.lastBackendUsed}
        mode={vm.state.mode}
        remoteUnit={vm.state.remoteUnit}
        transport={vm.state.transport}
        isTransmitting={vm.state.isTransmitting}
        isBeaconRunning={vm.state.isBeaconRunning}
        activeMacroId={vm.state.activeMacroId}
        macroStepIndex={vm.state.macroStepIndex}
        macroTotalSteps={vm.state.macroTotalSteps}
        error={vm.state.error}
        onAbortMacro={vm.abortMacro}
      />

      <Tabs
        value={activeTab}
        onValueChange={(v) => {
          if (isIrTab(v)) setActiveTab(v);
        }}
      >
        <TabsList className="w-full h-auto flex-wrap justify-start gap-1">
          <TabsTrigger value="remote" data-testid="ir-tab-remote">
            {t('irCommander.tabs.remote')}
          </TabsTrigger>
          <TabsTrigger value="catalog" data-testid="ir-tab-catalog">
            {t('irCommander.tabs.catalog')}
          </TabsTrigger>
          <TabsTrigger value="service" data-testid="ir-tab-service">
            {t('irCommander.tabs.service')}
          </TabsTrigger>
          <TabsTrigger value="diagnostics" data-testid="ir-tab-diagnostics">
            {t('irCommander.tabs.diagnostics')}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="remote">
          <IrRemoteTab
            mode={vm.state.mode}
            remoteUnit={vm.state.remoteUnit}
            editMode={vm.state.editMode}
            isLoading={vm.isLoadingDeck}
            isTransmitting={vm.state.isTransmitting}
            profile={vm.activeDeckProfile}
            onSelectMode={m => void vm.selectMode(m)}
            onSelectComboDeck={() => void vm.selectComboDeck()}
            onSelectRemoteUnit={vm.selectRemoteUnit}
            onToggleEditMode={vm.toggleEditMode}
            onTriggerKey={onRequestTriggerKey}
            onEditKey={vm.setEditingKey}
          />
        </TabsContent>

        <TabsContent value="catalog">
          <IrCatalogKeymapTab
            totalCodesets={vm.totalCodesets}
            totalAliases={vm.totalAliases}
            searchQuery={vm.state.searchQuery}
            selectedCategory={vm.state.selectedCategory}
            searchResults={vm.searchResults}
            isSearching={vm.isSearching}
            isTransmitting={vm.state.isTransmitting}
            editMode={vm.state.editMode}
            activeProfile={vm.state.activeProfile}
            savedProfiles={vm.state.savedProfiles}
            exportedIrplusXml={vm.exportedXml}
            onSearchQueryChange={vm.setSearchQuery}
            onCategoryChange={vm.setSelectedCategory}
            onSelectCodeset={id => void vm.selectCodeset(id)}
            onSelectSavedProfile={vm.selectSavedProfile}
            onTriggerKey={onRequestTriggerKey}
            onEditKey={vm.setEditingKey}
            onAddBlankKey={vm.addBlankKey}
            onSaveProfile={vm.saveCurrentProfile}
            onImportText={(fmt, body, name) => void vm.importProfileText(fmt, body, name)}
            onExportIrplus={() => void vm.exportCurrentIrplus()}
            onClearExportXml={vm.clearExportedXml}
          />
        </TabsContent>

        <TabsContent value="service">
          <IrServiceRecoveryTab
            macros={vm.verifiedMacros}
            isExecutingMacro={vm.state.isTransmitting}
            onExecuteMacro={macro => void vm.executeMacro(macro)}
          />
        </TabsContent>

        <TabsContent value="diagnostics">
          <IrDiagnosticsTab
            capabilities={vm.capabilities}
            transport={vm.state.transport}
            picoGpioPin={vm.state.picoGpioPin}
            safePicoGpios={vm.safePicoGpios}
            timingUnitOverride={vm.state.timingUnitOverride}
            repeatCount={vm.state.repeatCount}
            lastProntoHex={vm.state.lastProntoHex}
            isBeaconRunning={vm.state.isBeaconRunning}
            onChangeTransport={vm.changeTransport}
            onChangePicoGpio={vm.changePicoGpio}
            onChangeTimingUnit={vm.changeTimingUnit}
            onChangeRepeatCount={vm.changeRepeatCount}
            onToggleCameraBeacon={() => void vm.toggleCameraBeacon()}
            onRefreshCapabilities={vm.refreshCapabilities}
          />
        </TabsContent>
      </Tabs>

      <IrKeyEditorDialog
        keyItem={vm.editingKey}
        onClose={() => vm.setEditingKey(null)}
        onSave={(updated) => {
          vm.saveEditedKey(updated);
          setSuccessFeedback(updated.label);
          toast({
            type: 'success',
            title: updated.label,
            message: t('irCommander.editor.title'),
          });
        }}
        onDelete={vm.deleteEditedKey}
      />

      {successFeedback && (
        <output className="block">
          <Alert variant="success">
            <AlertDescription>{successFeedback}</AlertDescription>
          </Alert>
        </output>
      )}

      <Dialog
        open={pendingConfirmKey !== null}
        onOpenChange={open => !open && setPendingConfirmKey(null)}
      >
        <DialogContent className="max-w-sm font-mono">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-rose-300">
              <ShieldAlert className="w-4 h-4" />
              <span>{pendingConfirmKey?.label}</span>
            </DialogTitle>
            <DialogDescription>{t('irCommander.remote.confirmKeyDesc')}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setPendingConfirmKey(null)}
            >
              {t('common.cancel')}
            </Button>
            <Button
              type="button"
              variant="destructive"
              size="sm"
              onClick={confirmProtectedTransmission}
              data-testid="ir-confirm-key-btn"
            >
              {t('irCommander.remote.confirmKeyBtn')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};
