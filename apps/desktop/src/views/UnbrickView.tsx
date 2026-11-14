import React from 'react';
import { Loader2, RefreshCw, ShieldAlert } from 'lucide-react';
import { useUnbrickViewModel } from '@/viewmodels/useUnbrickViewModel';
import {
  SdKitBuilderCard,
  TbcPassthroughCard,
  UnbrickArchitectureCard,
} from '@/components/unbrick';
import { Alert, AlertDescription, AlertTitle, Badge, Button } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';

export const UnbrickView: React.FC = () => {
  const { t } = useTranslation();
  const {
    workbench,
    sdKit,
    verify,
    extract,
    tbc,
    rspPlan,
    norStub,
    loadWorkbench,
    buildSdKit,
    verifySdMedia,
    extractFrmModules,
    runMainPassthrough,
    generateRspPlan,
    buildNorStub,
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

          <div className="grid grid-cols-1 xl:grid-cols-2 gap-6 items-start">
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
            />

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
          </div>
        </>
      )}
    </div>
  );
};
