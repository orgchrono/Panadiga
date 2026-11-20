import React from 'react';
import { HardDrive, RefreshCw, ShieldAlert } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import {
  PipelineStepRunner,
  SafetyConfirmGate,
  SafetyPreflightCard,
  Sha256ArtifactCard,
} from '@/components/pipelines';
import {
  Alert,
  AlertDescription,
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  EmptyState,
  Input,
} from '@/components/ui';
import {
  BUS_TYPE_NVME_ID,
  HDD_BENCH_5GB_CAPACITY,
  ONE_TERABYTE_GB,
  QUICK_CAPACITIES_GB,
} from '@/constants';
import { useTranslation, type TranslationKey } from '@/i18n/useTranslation';
import type {
  Eh55HddFlashReport,
  PipelinePreflightAndGateProps,
  PipelineProgressState,
  PipelineStepDescriptor,
  SafeTargetDiskInfo,
} from '@/models';

export interface HddSafeFlashVm extends PipelinePreflightAndGateProps {
  readonly targetGb: number;
  readonly setTargetGb: (gb: number) => void;
  readonly allowExtended4tb: boolean;
  readonly setAllowExtended4tb: (v: boolean) => void;
  readonly capacityValid: boolean;
  readonly maxAllowedGb: number;
  readonly previewBlocks: number;
  readonly disks: readonly SafeTargetDiskInfo[];
  readonly scanningDisks: boolean;
  readonly scanError: string | null;
  readonly scanSafeDisks: () => Promise<void>;
  readonly selectedDisk: SafeTargetDiskInfo | null;
  readonly selectSafeDisk: (d: SafeTargetDiskInfo) => void;
  readonly imagePath: string;
  readonly setImagePath: (v: string) => void;
  readonly backupDir: string;
  readonly setBackupDir: (v: string) => void;
  readonly flashReport: Eh55HddFlashReport | null;
  readonly flashBusy: boolean;
  readonly flashError: string | null;
  readonly flashSuccess: string | null;
  readonly flashProgress: PipelineProgressState | null;
  readonly flashLogs: readonly string[];
  readonly runSafeFlash: () => Promise<Eh55HddFlashReport | null>;
  readonly resetFlash: () => void;
  readonly steps: readonly PipelineStepDescriptor[];
  readonly copiedHash: boolean;
  readonly copyDigest: (digest: string) => void;
}

function resolveDiskLockBadge(
  disk: SafeTargetDiskInfo,
  t: (k: TranslationKey, p?: Record<string, unknown>) => string,
): { readonly blocked: boolean; readonly label: string } {
  if (disk.device_path.toLowerCase().includes('physicaldrive0')) {
    return { blocked: true, label: t('pipelines.hddFlash.lockouts.disk0') };
  }
  if (disk.is_nvme_blocked || disk.bus_type_id === BUS_TYPE_NVME_ID) {
    return { blocked: true, label: t('pipelines.hddFlash.lockouts.nvme') };
  }
  if (disk.is_os_blocked || !disk.safe_to_flash) {
    return { blocked: true, label: disk.lockout_reason ?? t('pipelines.hddFlash.lockouts.osBoot') };
  }
  return { blocked: false, label: t('pipelines.hddFlash.lockouts.eligible') };
}

export const HddCapacityAndFlashPipeline: React.FC<{ readonly vm: HddSafeFlashVm }> = ({ vm }) => {
  const { t } = useTranslation();
  const backupInvalid = !vm.backupDir.trim();
  const quickCapacities: readonly number[] = QUICK_CAPACITIES_GB;
  const artifacts: readonly { readonly id: string; readonly title: string; readonly path: string; readonly sha256: string; readonly bytes?: number }[] = vm.flashReport
    ? [
        { id: 'backup64k', title: t('pipelines.hddFlash.report.backupShaLabel'), path: vm.flashReport.backup_bin_path, sha256: vm.flashReport.backup_64k_sha256 },
        { id: 'primary', title: t('pipelines.hddFlash.report.primaryShaLabel'), path: vm.flashReport.device_path, sha256: vm.flashReport.primary_sha256, bytes: vm.flashReport.primary_bytes_written },
        ...(vm.flashReport.mirror_sha256
          ? [{ id: 'mirror', title: t('pipelines.hddFlash.report.mirrorShaLabel'), path: t('pipelines.hddFlash.report.mirrorLbaLabel', { lba: vm.flashReport.mirror_main_lba }), sha256: vm.flashReport.mirror_sha256, bytes: vm.flashReport.mirror_bytes_written }]
          : []),
      ]
    : [];

  return (
    <Card className="bg-bench-900 border-slate-800 space-y-4">
      <CardHeader className="pb-2">
        <CardTitle className="text-base text-slate-100 flex items-center gap-2 font-mono">
          <HardDrive className="w-4 h-4 text-emerald-400" />
          <span>{t('pipelines.hddFlash.title')}</span>
        </CardTitle>
        <CardDescription className="text-xs text-slate-400">{t('pipelines.hddFlash.subtitle')}</CardDescription>
      </CardHeader>

      <CardContent className="space-y-4 font-mono">
        <div className="p-4 rounded-xl bg-slate-950 border border-slate-800 space-y-3 text-xs">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="font-bold text-slate-200">{t('pipelines.hddFlash.capacityLabel')}</span>
            {vm.targetGb === HDD_BENCH_5GB_CAPACITY && (
              <Badge variant="success" data-testid="hdd-safe-5gb-badge">{t('pipelines.hddFlash.badge5gbBench')}</Badge>
            )}
          </div>
          <div className="flex flex-wrap gap-1.5">
            {quickCapacities.length === 0 ? null : quickCapacities.map(gb => (
              <button
                key={gb}
                type="button"
                data-testid={`hdd-safe-cap-${gb}`}
                onClick={() => vm.setTargetGb(gb)}
                className={`px-2.5 py-1 rounded border text-[11px] font-bold ${
                  vm.targetGb === gb ? 'bg-emerald-500/20 border-emerald-500 text-emerald-300' : 'bg-bench-900 border-slate-800 text-slate-300'
                }`}
              >
                {gb >= ONE_TERABYTE_GB ? `${gb / ONE_TERABYTE_GB}TB` : `${gb}GB`}
              </button>
            ))}
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 items-end">
            <div className="space-y-1">
              <label htmlFor="hdd-safe-gb-input" className="text-slate-400 block">{t('pipelines.hddFlash.customGbLabel')}</label>
              <Input id="hdd-safe-gb-input" data-testid="hdd-safe-gb-input" type="number" value={vm.targetGb} aria-invalid={!vm.capacityValid} onChange={e => vm.setTargetGb(Number(e.target.value))} />
              {!vm.capacityValid && <span className="text-[11px] text-rose-400">{t('pipelines.hddFlash.invalidCapacity', { max: vm.maxAllowedGb })}</span>}
            </div>
            <label className="flex items-center gap-2 text-slate-300 cursor-pointer pb-2">
              <input type="checkbox" checked={vm.allowExtended4tb} onChange={e => vm.setAllowExtended4tb(e.target.checked)} />
              <span>{t('pipelines.hddFlash.allowExtended4tb')} (RTSD 0x{vm.previewBlocks.toString(16).toUpperCase()})</span>
            </label>
          </div>
        </div>

        <div className="space-y-2 text-xs">
          <div className="flex items-center justify-between gap-2">
            <span className="font-bold text-slate-200">{t('pipelines.hddFlash.steps.targetLockout')}</span>
            <Button type="button" size="sm" variant="outline" data-testid="hdd-safe-scan-btn" disabled={vm.scanningDisks} onClick={() => void vm.scanSafeDisks()} className="gap-1.5">
              <RefreshCw className={`w-3.5 h-3.5 ${vm.scanningDisks ? 'animate-spin' : ''}`} />
              <span>{t('pipelines.hddFlash.refreshDisksBtn')}</span>
            </Button>
          </div>
          {vm.scanError && <Alert variant="destructive"><AlertDescription>{vm.scanError}</AlertDescription></Alert>}
          {vm.disks.length === 0 ? (
            <EmptyState compact title={t('pipelines.hddFlash.emptyDisks')} />
          ) : (
            <div className="space-y-2">
              {vm.disks.map((disk, idx) => {
                const { blocked, label } = resolveDiskLockBadge(disk, t);
                const selected = vm.selectedDisk?.device_path === disk.device_path;
                return (
                  <button
                    key={disk.device_path}
                    type="button"
                    data-testid={`hdd-safe-disk-${idx}`}
                    disabled={blocked}
                    onClick={() => vm.selectSafeDisk(disk)}
                    className={`w-full text-left p-3 rounded-lg border flex flex-wrap items-center justify-between gap-2 transition-colors ${
                      blocked ? 'bg-slate-950/60 border-rose-500/40 opacity-60 cursor-not-allowed' : selected ? 'bg-emerald-950/30 border-emerald-500 text-slate-100' : 'bg-slate-950 border-slate-800 hover:border-slate-700'
                    }`}
                  >
                    <div>
                      <div className="font-bold text-slate-100">{disk.display_name} ({disk.size_gb} GB)</div>
                      <div className="text-[11px] text-slate-400">
                        {disk.device_path} • {t('pipelines.hddFlash.lockouts.busPrefix', { bus: disk.bus_type, id: disk.bus_type_id })}
                        {disk.mirror_main_lba ? ` • ${t('pipelines.hddFlash.lockouts.mirrorLba', { lba: disk.mirror_main_lba })}` : ''}
                      </div>
                    </div>
                    <StatusBadge variant={blocked ? 'error' : 'success'} label={label} />
                  </button>
                );
              })}
            </div>
          )}
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
          <div className="space-y-1">
            <label htmlFor="hdd-safe-backup-dir" className="text-slate-400 block">{t('pipelines.hddFlash.inputs.backupDir')}</label>
            <Input id="hdd-safe-backup-dir" data-testid="hdd-safe-backup-dir" type="text" value={vm.backupDir} aria-invalid={backupInvalid} onChange={e => vm.setBackupDir(e.target.value)} />
            {backupInvalid && <span className="text-[11px] text-rose-400">{t('pipelines.hddFlash.inputs.backupDirRequired')}</span>}
          </div>
          <div className="space-y-1">
            <label htmlFor="hdd-safe-image-path" className="text-slate-400 block">{t('pipelines.hddFlash.inputs.imagePath')}</label>
            <Input id="hdd-safe-image-path" type="text" value={vm.imagePath} onChange={e => vm.setImagePath(e.target.value)} />
          </div>
        </div>

        <SafetyPreflightCard checks={vm.preflightChecks} checkedIds={vm.checkedPreflightIds} onToggleCheck={vm.togglePreflight} severity="critical" />
        <SafetyConfirmGate
          expectedToken={vm.expectedConfirmToken || '---'}
          typedValue={vm.confirmTokenInput}
          onChange={vm.setConfirmTokenInput}
          onConfirm={() => void vm.runSafeFlash()}
          isBusy={vm.flashBusy}
          preflightReady={vm.preflightReady}
          description={vm.selectedDisk ? t('pipelines.hddFlash.inputs.gateDesc', { token: vm.expectedConfirmToken, path: vm.selectedDisk.device_path }) : t('pipelines.hddFlash.inputs.selectTargetPrompt')}
          inputTestId="hdd-safe-confirm-input"
          submitTestId="hdd-safe-flash-btn"
        />

        <PipelineStepRunner
          title={t('pipelines.hddFlash.title')}
          steps={vm.steps}
          isBusy={vm.flashBusy}
          error={vm.flashError}
          successMessage={vm.flashSuccess}
          progress={vm.flashProgress}
          logs={vm.flashLogs}
          onReset={vm.resetFlash}
        />

        {vm.flashReport && (
          <div className="space-y-3">
            <div className="flex items-center justify-between p-3 rounded-lg bg-slate-950 border border-slate-800 text-xs">
              <span className="font-bold text-slate-200 flex items-center gap-1.5">
                <ShieldAlert className="w-4 h-4 text-emerald-400" />
                {t('pipelines.hddFlash.report.title')}
              </span>
              <StatusBadge variant={vm.flashReport.readback_verified ? 'success' : 'error'} label={vm.flashReport.readback_verified ? t('pipelines.hddFlash.report.readbackOk') : t('pipelines.hddFlash.report.readbackFail')} />
            </div>
            {artifacts.length === 0 ? null : artifacts.map(item => (
              <Sha256ArtifactCard key={item.id} title={item.title} outputPath={item.path} outputSha256={item.sha256} bytesProcessed={item.bytes} verified={vm.flashReport?.readback_verified} copied={vm.copiedHash} onCopyHash={vm.copyDigest} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
};
