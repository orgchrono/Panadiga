import React from 'react';
import { Loader2, RefreshCw, ShieldAlert, Wrench } from 'lucide-react';
import { useUnbrickViewModel } from '@/viewmodels/useUnbrickViewModel';
import { useWorkbenchMode } from '@/context/WorkbenchModeContext';
import {
  CalibrationRegionPipeline,
  MatshitaEmulatorCard,
  SdKitBuilderCard,
  TbcLabAndSdHardenPipeline,
  TbcPassthroughCard,
  UnbrickArchitectureCard,
} from '@/components/unbrick';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui';
import { useCalibrationTbcViewModel } from '@/viewmodels/pipelines';
import { useTranslation } from '@/i18n/useTranslation';

export const UnbrickView: React.FC = () => {
  const { t } = useTranslation();
  const { isAdvanced, setMode } = useWorkbenchMode();
  const { calibration, tbcSd } = useCalibrationTbcViewModel();
  const {
    workbench,
    sdKit,
    verify,
    sdDisk,
    extract,
    tbc,
    rspPlan,
    norStub,
    matshitaBoot,
    matshitaInject,
    matshitaSerial,
    frmVerify,
    frmPack,
    loadWorkbench,
    buildSdKit,
    verifySdMedia,
    inspectSdDisk,
    extractFrmModules,
    runMainPassthrough,
    generateRspPlan,
    buildNorStub,
    runMatshitaBoot,
    injectMatshitaWrite,
    runMatshitaSerial,
    verifyUnlockFrm,
    packUnlockFrm,
    sdDrives,
    sdDriveFilter,
    setSdDriveFilter,
    sdDrivesLoading,
    refreshSdDrives,
    autoSdDiskTarget,
    autoSdMountDir,
    sdFat16Image,
    createFat16Image,
  } = useUnbrickViewModel();

  const data = workbench.data;

  return (
    <div className="space-y-6">
      <div className="rounded-2xl bg-bench-900/90 border border-slate-800 p-5 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 shadow-xl">
        <div className="space-y-1">
          <div className="flex flex-wrap items-center gap-2">
            <ShieldAlert className="w-5 h-5 text-emerald-400" />
            <h2 className="text-base font-bold text-slate-100 font-mono">
              {t('unbrick.header.title')}
            </h2>
            <Badge variant="default">MN103S / BOOT.BIN + PROG061 + MAIN3410PQ</Badge>
          </div>
          <p className="text-xs text-slate-400 max-w-3xl">{t('unbrick.header.subtitle')}</p>
        </div>

        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={workbench.loading}
          onClick={() => void loadWorkbench()}
        >
          {workbench.loading ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
          ) : (
            <RefreshCw className="w-3.5 h-3.5 mr-1.5" />
          )}
          <span>{t('unbrick.header.reloadBtn')}</span>
        </Button>
      </div>

      {workbench.error && (
        <Alert variant="destructive">
          <AlertTitle>{t('unbrick.header.errorTitle')}</AlertTitle>
          <AlertDescription>{workbench.error}</AlertDescription>
        </Alert>
      )}

      {workbench.loading && !data ? (
        <div className="rounded-2xl border border-slate-800 bg-bench-900/60 p-10 flex items-center justify-center gap-3 text-xs font-mono text-slate-400">
          <Loader2 className="w-4 h-4 animate-spin text-cyan-400" />
          <span>{t('unbrick.header.loadingText')}</span>
        </div>
      ) : (
        <>
          <UnbrickArchitectureCard
            workbench={data}
            extractLoading={extract.loading}
            extractResult={extract.data}
            onExtractFrm={extractFrmModules}
          />

          <Tabs defaultValue="tbc-sd-harden" className="space-y-4">
            <TabsList aria-label={t('unbrick.tabs.tablistAria')} className="flex flex-wrap h-auto gap-1">
              <TabsTrigger value="tbc-sd-harden" data-testid="unbrick-tab-tbc-sd">
                {t('unbrick.tabs.tbcSdHarden')}
              </TabsTrigger>
              <TabsTrigger value="calibration-region" data-testid="unbrick-tab-calibration">
                {t('unbrick.tabs.calibrationRegion')}
              </TabsTrigger>
              <TabsTrigger value="matshita-emu" data-testid="unbrick-tab-matshita">
                {t('unbrick.tabs.matshitaEmu')}
              </TabsTrigger>
            </TabsList>

            <TabsContent value="tbc-sd-harden" className="space-y-6">
              <div className={isAdvanced ? 'grid grid-cols-1 xl:grid-cols-2 gap-6 items-start' : 'space-y-6'}>
                <SdKitBuilderCard
                  discoveredPanaDvd={data?.discovered_pana_dvd ?? null}
                  discoveredSdDir={data?.discovered_sd_dir ?? null}
                  kitLoading={sdKit.loading}
                  kitResult={sdKit.data}
                  kitError={sdKit.error}
                  verifyLoading={verify.loading}
                  verifyResult={verify.data}
                  verifyError={verify.error}
                  onBuildKit={buildSdKit}
                  onVerifyMedia={verifySdMedia}
                  diskLoading={sdDisk.loading}
                  diskResult={sdDisk.data}
                  diskError={sdDisk.error}
                  onInspectSdDisk={inspectSdDisk}
                  sdDrives={sdDrives}
                  sdDriveFilter={sdDriveFilter}
                  onChangeSdDriveFilter={setSdDriveFilter}
                  sdDrivesLoading={sdDrivesLoading}
                  onRefreshSdDrives={() => void refreshSdDrives()}
                  autoSdDiskTarget={autoSdDiskTarget}
                  autoSdMountDir={autoSdMountDir}
                  fat16Loading={sdFat16Image.loading}
                  fat16Report={sdFat16Image.data}
                  fat16Error={sdFat16Image.error}
                  onCreateFat16Image={createFat16Image}
                  hardenVm={tbcSd}
                />

                {isAdvanced && (
                  <TbcPassthroughCard
                    discoveredMainPath={data?.discovered_main_3410pq ?? null}
                    tbcLoading={tbc.loading}
                    tbcReport={tbc.data}
                    tbcError={tbc.error}
                    rspLoading={rspPlan.loading}
                    rspPlan={rspPlan.data}
                    norStubLoading={norStub.loading}
                    norStubResult={norStub.data}
                    onRunTbc={runMainPassthrough}
                    onRefreshRspPlan={generateRspPlan}
                    onBuildNorStub={buildNorStub}
                  />
                )}
              </div>
              <TbcLabAndSdHardenPipeline vm={tbcSd} hideSdHardenSection />
            </TabsContent>

            <TabsContent value="calibration-region">
              <CalibrationRegionPipeline vm={calibration} />
            </TabsContent>

            <TabsContent value="matshita-emu">
              {isAdvanced ? (
                <MatshitaEmulatorCard
                  bootState={matshitaBoot}
                  injectState={matshitaInject}
                  serialState={matshitaSerial}
                  frmVerifyState={frmVerify}
                  frmPackState={frmPack}
                  onRunBoot={runMatshitaBoot}
                  onInjectWrite={injectMatshitaWrite}
                  onRunSerial={runMatshitaSerial}
                  onVerifyFrm={verifyUnlockFrm}
                  onPackFrm={packUnlockFrm}
                />
              ) : (
                <div className="rounded-xl bg-bench-900/70 border border-slate-800 px-4 py-3 flex flex-wrap items-center justify-between gap-3 text-xs">
                  <span className="text-slate-400 font-sans">{t('unbrick.basicModeBanner')}</span>
                  <Button type="button" variant="outline" size="sm" onClick={() => setMode('engineering')} className="gap-1.5 text-xs">
                    <Wrench className="w-3.5 h-3.5 text-cyan-400" />
                    <span>{t('unbrick.revealAdvancedBtn')}</span>
                  </Button>
                </div>
              )}
            </TabsContent>
          </Tabs>
        </>
      )}
    </div>
  );
};
