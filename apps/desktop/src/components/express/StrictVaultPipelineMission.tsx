import React from 'react';
import {
  Cpu,
  FolderArchive,
  Loader2,
  ShieldCheck,
  Usb,
} from 'lucide-react';
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  EmptyState,
  Input,
  Select,
} from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type {
  DigaModelProfile,
  ResolvedModelProfile,
  SdLoaderProfile,
  StrictVaultMissionState,
  VaultArtifactRecord,
  VaultBucket,
} from '@/models';
import { DigaModelSelect } from './DigaModelSelect';
import { ExpressModeChoiceGroup } from './ExpressModeChoiceGroup';
import { VaultMediaAndGatePanel } from './VaultMediaAndGatePanel';

const SHA_PREFIX_LEN = 12;

export interface StrictVaultPipelineMissionProps {
  readonly modelCatalog: readonly DigaModelProfile[];
  readonly selectedModel: string;
  readonly onSelectModel: (model: string) => void;
  readonly vm: StrictVaultMissionState;
}

const ResolvedHardwareSummaryBox: React.FC<{
  readonly resolved: ResolvedModelProfile | null;
  readonly isLoading: boolean;
}> = ({ resolved, isLoading }) => {
  const { t } = useTranslation();
  if (!resolved) {
    return (
      <div className="text-xs text-slate-400">
        {isLoading ? t('express.vault.loadingSpec') : t('express.vault.noSpecYet')}
      </div>
    );
  }
  const flashPart = resolved.flashChip?.part_number ?? 'UNKNOWN';
  const socLabel = resolved.soc?.part_number ?? resolved.socCore?.isa ?? 'UNKNOWN';
  const nvramHex = resolved.nvramUncachedVaHex ?? 'UNKNOWN';
  const iplRange = resolved.iplProtectedRangeHex ?? 'UNKNOWN';
  const eepromList = resolved.eepromChips.map(e => e.part_number).join(', ') || 'NONE';
  const mediaList = resolved.mediaFormats.map(m => m.filesystem).join(', ') || 'N/A';

  return (
    <div
      data-testid="express-vault-resolved-spec"
      className="rounded-lg border border-slate-800 bg-slate-900/70 p-2.5 text-[11px] font-mono text-slate-300 space-y-1"
    >
      <div className="flex items-center gap-1.5 text-cyan-300">
        <Cpu className="w-3.5 h-3.5" />
        <span>
          {flashPart} • {socLabel}
        </span>
      </div>
      <div>{t('express.vault.resolvedNvramIpl', { nvram: nvramHex, ipl: iplRange })}</div>
      <div>{t('express.vault.resolvedEepromMedia', { eeprom: eepromList, media: mediaList })}</div>
    </div>
  );
};

const BucketArtifactList: React.FC<{
  readonly testId: string;
  readonly title: string;
  readonly emptyText: string;
  readonly items: readonly VaultArtifactRecord[];
  readonly accentClass: string;
}> = ({ testId, title, emptyText, items, accentClass }) => (
  <div data-testid={testId} className="rounded-lg border border-slate-800 bg-slate-950/80 p-3 space-y-2">
    <div className="flex items-center justify-between">
      <span className={`text-xs font-mono font-bold ${accentClass}`}>{title}</span>
      <Badge variant="secondary" className="text-[10px] font-mono">{items.length}</Badge>
    </div>
    {items.length === 0 ? (
      <EmptyState compact title={emptyText} />
    ) : (
      <ul className="space-y-1 text-[11px] font-mono text-slate-300">
        {items.map(item => (
          <li key={item.id} className="flex items-center justify-between gap-2 truncate">
            <span className="truncate">{item.filename}</span>
            <span className="text-slate-500">{item.sha256.slice(0, SHA_PREFIX_LEN)}</span>
          </li>
        ))}
      </ul>
    )}
  </div>
);

export const StrictVaultPipelineMission: React.FC<StrictVaultPipelineMissionProps> = ({
  modelCatalog,
  selectedModel,
  onSelectModel,
  vm,
}) => {
  const { t } = useTranslation();

  return (
    <Card data-testid="express-mission-vault" className="border-slate-800 bg-bench-900/90 shadow-xl">
      <CardHeader>
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div>
            <CardTitle className="flex items-center gap-2 text-base text-slate-100">
              <FolderArchive className="w-5 h-5 text-cyan-400" />
              <span>{t('express.vault.title')}</span>
            </CardTitle>
            <CardDescription className="mt-1">{t('express.vault.desc')}</CardDescription>
          </div>
          <Badge variant="success" className="font-mono text-[10px]">
            {t('express.vault.badgeZeroBlobs')}
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="space-y-5">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-3">
            <h3 className="text-xs font-mono font-bold text-cyan-300">
              {t('express.vault.step1ProfileTitle')}
            </h3>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
              <DigaModelSelect
                ariaLabel={t('express.sdMission.modelLabel')}
                testId="express-vault-model-select"
                modelCatalog={modelCatalog}
                value={selectedModel}
                onChange={onSelectModel}
              />
              <Select
                aria-label={t('express.vault.loaderProfileLabel')}
                data-testid="express-vault-loader-select"
                value={vm.loaderProfile}
                onChange={e =>
                  vm.setLoaderProfile(e.target.value === 'ver063' ? 'ver063' : 'ver061' satisfies SdLoaderProfile)
                }
              >
                <option value="ver061">{t('express.vault.loaderVer061')}</option>
                <option value="ver063">{t('express.vault.loaderVer063')}</option>
              </Select>
            </div>

            <ResolvedHardwareSummaryBox
              resolved={vm.resolvedProfile}
              isLoading={vm.isLoading}
            />

            {vm.sessionError && (
              <Alert variant="destructive" className="py-2">
                <AlertDescription className="text-xs font-mono">{vm.sessionError}</AlertDescription>
              </Alert>
            )}

            <Button
              type="button"
              variant="default"
              data-testid="express-vault-init-btn"
              disabled={vm.sessionBusy}
              onClick={() => void vm.initializeVaultSession()}
              className="w-full gap-2"
            >
              {vm.sessionBusy ? <Loader2 className="w-4 h-4 animate-spin" /> : <ShieldCheck className="w-4 h-4" />}
              <span>{t('express.vault.initBtn')}</span>
            </Button>
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-3">
            <h3 className="text-xs font-mono font-bold text-cyan-300">
              {t('express.vault.step3PicoTitle')}
            </h3>
            <p className="text-[11px] text-slate-400">{t('express.vault.step3PicoDesc')}</p>
            <ExpressModeChoiceGroup
              activeValue={vm.uf2Variant}
              activeVariant="emerald"
              onSelect={val => vm.setUf2Variant(val)}
              options={[
                { value: 'rp2040', label: 'Pico RP2040', testId: 'express-vault-uf2-rp2040' },
                { value: 'rp2350', label: 'Pico 2 RP2350', testId: 'express-vault-uf2-rp2350' },
              ]}
            />
            <div className="text-[11px] font-mono text-slate-400">
              {t('express.vault.bootselLabel')}:{' '}
              {vm.bootselDrives.length > 0 ? vm.bootselDrives.join(', ') : t('express.vault.noBootsel')}
            </div>
            {vm.uf2Error && (
              <Alert variant="destructive" className="py-2">
                <AlertDescription className="text-xs font-mono">{vm.uf2Error}</AlertDescription>
              </Alert>
            )}
            <div className="flex gap-2">
              <Button type="button" variant="secondary" onClick={() => void vm.detectBootsel()} className="flex-1">
                {t('express.vault.detectBootselBtn')}
              </Button>
              <Button
                type="button"
                variant="emerald"
                data-testid="express-vault-flash-uf2-btn"
                disabled={vm.uf2Busy}
                onClick={() => void vm.flashPicoUf2()}
                className="flex-1 gap-1"
              >
                {vm.uf2Busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Usb className="w-3.5 h-3.5" />}
                <span>{t('express.vault.flashUf2Btn')}</span>
              </Button>
            </div>
            {vm.uf2Report && (
              <Alert variant="success" className="py-2">
                <AlertTitle className="text-xs font-mono">{vm.uf2Report.message}</AlertTitle>
              </Alert>
            )}
          </div>
        </div>

        <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4 space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="text-xs font-mono font-bold text-cyan-300">
              {t('express.vault.step4VaultTitle')}
            </h3>
            <span className="text-[11px] font-mono text-slate-400">{t('express.vault.segregationRule')}</span>
          </div>

          <div className="flex flex-col sm:flex-row gap-2">
            <Input
              aria-label={t('express.vault.ingestPathLabel')}
              data-testid="express-vault-ingest-input"
              value={vm.ingestPathInput}
              onChange={e => vm.setIngestPathInput(e.target.value)}
              placeholder="ic5001_nor_8mb.bin | PANA_DVD.FRM"
              className="flex-1"
            />
            <Select
              aria-label={t('express.vault.bucketOverrideLabel')}
              data-testid="express-vault-ingest-bucket"
              value={vm.ingestBucketOverride}
              onChange={e => {
                const v = e.target.value;
                const parsed: 'auto' | VaultBucket =
                  v === 'raw' || v === 'frm' || v === 'recompiled' ? v : 'auto';
                vm.setIngestBucketOverride(parsed);
              }}
              className="sm:w-44"
            >
              <option value="auto">{t('express.vault.bucketAuto')}</option>
              <option value="raw">{t('express.vault.bucketRawOption')}</option>
              <option value="frm">{t('express.vault.bucketFrmOption')}</option>
            </Select>
            <Button
              type="button"
              variant="secondary"
              data-testid="express-vault-ingest-btn"
              disabled={vm.ingestBusy}
              onClick={() => void vm.ingestIntoVault()}
            >
              {t('express.vault.ingestBtn')}
            </Button>
          </div>

          {vm.ingestError && (
            <Alert variant="destructive" data-testid="express-vault-ingest-error" className="py-2">
              <AlertDescription className="text-xs font-mono">{vm.ingestError}</AlertDescription>
            </Alert>
          )}

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <BucketArtifactList
              testId="express-vault-bucket-raw"
              title={t('express.vault.bucketRawTitle')}
              emptyText={t('express.vault.emptyBucket')}
              items={vm.activeSession?.rawArtifacts ?? []}
              accentClass="text-amber-300"
            />
            <BucketArtifactList
              testId="express-vault-bucket-frm"
              title={t('express.vault.bucketFrmTitle')}
              emptyText={t('express.vault.emptyBucket')}
              items={vm.activeSession?.frmArtifacts ?? []}
              accentClass="text-cyan-300"
            />
            <BucketArtifactList
              testId="express-vault-bucket-recompiled"
              title={t('express.vault.bucketRecompiledTitle')}
              emptyText={t('express.vault.emptyBucket')}
              items={vm.activeSession?.recompiledArtifacts ?? []}
              accentClass="text-emerald-300"
            />
          </div>
        </div>

        <VaultMediaAndGatePanel
          opticalBurners={vm.opticalBurners}
          dvdTargetMode={vm.dvdTargetMode}
          onSelectDvdTargetMode={vm.setDvdTargetMode}
          selectedBurnerId={vm.selectedBurnerId}
          onSelectBurnerId={vm.setSelectedBurnerId}
          dvdConfirmToken={vm.dvdConfirmToken}
          onChangeDvdConfirmToken={vm.setDvdConfirmToken}
          dvdBusy={vm.dvdBusy}
          dvdProgress={vm.dvdProgress}
          dvdReport={vm.dvdReport}
          dvdError={vm.dvdError}
          onRefreshBurners={() => void vm.refreshBurners()}
          onExecuteDvdCreate={() => void vm.executeDvdCreate()}
          emuGateBusy={vm.emuGateBusy}
          emuGateReport={vm.emuGateReport}
          emuGateError={vm.emuGateError}
          onExecuteMandatoryEmuGate={() => void vm.executeMandatoryEmuGate()}
          hddTargets={vm.hddTargets}
          hddDevicePath={vm.hddDevicePath}
          onChangeHddDevicePath={vm.setHddDevicePath}
          hddConfirmToken={vm.hddConfirmToken}
          onChangeHddConfirmToken={vm.setHddConfirmToken}
          hddBusy={vm.hddBusy}
          hddReport={vm.hddReport}
          hddError={vm.hddError}
          onExecuteComposedHddFlash={() => void vm.executeComposedHddFlash()}
        />
      </CardContent>
    </Card>
  );
};
