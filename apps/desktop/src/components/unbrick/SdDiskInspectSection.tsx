import React, { useState } from 'react';
import { HardDrive, Loader2 } from 'lucide-react';
import { KIB_BYTES, SD_DISK_CHECKS_PREVIEW } from '@/constants';
import { StorageDeviceSelector } from '@/components/hdd';
import { Alert, AlertDescription, Badge, Button, EmptyState, Input } from '@/components/ui';
import { useTranslation } from '@/i18n/useTranslation';
import type { SdDiskInspectionReport } from '@/models';
import { resolvePrimaryDriveTarget } from '@/utils/storageDrives';
import {
  resolveSdOverallBadgeVariant,
  validateRequiredText,
  type SdDiskVerificationProps,
} from './unbrickDomain';

const SdDiskReportCard: React.FC<{ readonly diskResult: SdDiskInspectionReport }> = ({
  diskResult,
}) => {
  const { t } = useTranslation();
  const previewChecks = diskResult.checks.slice(0, SD_DISK_CHECKS_PREVIEW);
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/80 p-3 space-y-2 text-[11px] font-mono">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="font-semibold text-cyan-300">{diskResult.target}</span>
        <div className="flex flex-wrap gap-1.5">
          <Badge variant="outline">{diskResult.partition_scheme}</Badge>
          {diskResult.bpb && (
            <Badge variant="secondary">
              {t('unbrick.sdKit.kibPerCluster', {
                kind: diskResult.bpb.computed_fat_kind,
                kib: Math.round(diskResult.bpb.cluster_size_bytes / KIB_BYTES),
              })}
            </Badge>
          )}
          <Badge variant={resolveSdOverallBadgeVariant(diskResult.overall_status)}>
            {diskResult.overall_status}
          </Badge>
        </div>
      </div>
      {diskResult.digadiag_entry && (
        <div className="text-slate-300">
          {diskResult.digadiag_entry.name_8_3}: {diskResult.digadiag_entry.file_size.toLocaleString()} B
          {' • '}
          {diskResult.digadiag_entry.chain_contiguous
            ? t('unbrick.sdKit.contiguous')
            : t('unbrick.sdKit.fragmented')}
          {' • '}
          {diskResult.digadiag_entry.chain_complete
            ? t('unbrick.sdKit.chainOk')
            : t('unbrick.sdKit.chainIncomplete')}
        </div>
      )}
      {diskResult.pana_sd_entry && (
        <div className="text-slate-300">
          {diskResult.pana_sd_entry.name_8_3}: {diskResult.pana_sd_entry.file_size.toLocaleString()} B
          {' • '}
          {diskResult.pana_sd_entry.chain_contiguous
            ? t('unbrick.sdKit.contiguous')
            : t('unbrick.sdKit.fragmented')}
        </div>
      )}
      {previewChecks.length === 0 ? (
        <EmptyState compact title={t('unbrick.sdKit.checksEmpty')} />
      ) : (
        previewChecks.map((chk) => (
          <div key={chk.id} className="text-slate-400">
            [{chk.severity.toUpperCase()}] {chk.title}: {chk.detail}
          </div>
        ))
      )}
    </div>
  );
};

type SdDiskInspectSectionProps = Omit<
  SdDiskVerificationProps,
  'verifyLoading' | 'verifyResult' | 'verifyError' | 'onVerifyMedia' | 'autoSdMountDir'
> & {
  onChangeVerifyDir: (val: string) => void;
};

export const SdDiskInspectSection: React.FC<SdDiskInspectSectionProps> = ({
  diskLoading = false,
  diskResult = null,
  diskError = null,
  onInspectSdDisk,
  sdDrives = [],
  sdDriveFilter = 'sd_card',
  onChangeSdDriveFilter,
  sdDrivesLoading = false,
  onRefreshSdDrives,
  autoSdDiskTarget = null,
  onChangeVerifyDir,
}) => {
  const { t } = useTranslation();
  const [editedDiskTarget, setEditedDiskTarget] = useState<string | null>(null);
  const [diskTargetError, setDiskTargetError] = useState<string | null>(null);
  const diskTarget = editedDiskTarget ?? autoSdDiskTarget ?? 'E:';

  if (!onInspectSdDisk) return null;

  const handleInspectDiskClick = async () => {
    const res = validateRequiredText(diskTarget, 'unbrick.sdKit.inspectDiskRequired');
    if (res.errorKey) {
      setDiskTargetError(t(res.errorKey));
      return;
    }
    setDiskTargetError(null);
    await onInspectSdDisk(res.trimmed);
  };

  return (
    <div className="border-t border-slate-800/80 pt-3 space-y-2.5">
      {onChangeSdDriveFilter && onRefreshSdDrives && (
        <StorageDeviceSelector
          compact
          drives={sdDrives}
          selectedDevicePath={diskTarget}
          roleFilter={sdDriveFilter}
          onChangeRoleFilter={onChangeSdDriveFilter}
          loading={sdDrivesLoading}
          onRefresh={onRefreshSdDrives}
          onSelectDrive={(drive) => {
            const resolved = resolvePrimaryDriveTarget(drive, true);
            setEditedDiskTarget(resolved.diskTarget);
            setDiskTargetError(null);
            if (resolved.mountPoint) {
              onChangeVerifyDir(resolved.mountPoint);
            }
          }}
        />
      )}
      <div className="flex flex-col sm:flex-row gap-2">
        <div className="flex-1 space-y-1">
          <Input
            value={diskTarget}
            aria-label={t('unbrick.sdKit.inspectDiskLabel')}
            aria-invalid={Boolean(diskTargetError)}
            aria-describedby={diskTargetError ? 'sd-disk-target-err' : undefined}
            onChange={(e) => {
              setEditedDiskTarget(e.target.value);
              if (diskTargetError) setDiskTargetError(null);
            }}
            placeholder={t('unbrick.sdKit.diskTargetPlaceholder')}
          />
          {diskTargetError && (
            <p id="sd-disk-target-err" className="text-[11px] text-rose-400 font-mono">
              {diskTargetError}
            </p>
          )}
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={diskLoading}
          onClick={() => void handleInspectDiskClick()}
        >
          {diskLoading ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
          ) : (
            <HardDrive className="w-3.5 h-3.5 mr-1.5 text-emerald-400" />
          )}
          <span>{t('unbrick.sdKit.inspectDiskBtn')}</span>
        </Button>
      </div>

      {diskError && (
        <Alert variant="destructive">
          <AlertDescription>{diskError}</AlertDescription>
        </Alert>
      )}

      {diskResult && <SdDiskReportCard diskResult={diskResult} />}
    </div>
  );
};
