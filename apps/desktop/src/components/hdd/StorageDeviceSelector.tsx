import React from 'react';
import { FolderOpen, HardDrive, MemoryStick, RefreshCw, ShieldAlert, Unplug } from 'lucide-react';
import { StatusBadge } from '@/components/StatusBadge';
import type { TranslationKey } from '@/i18n/translations';
import { useTranslation } from '@/i18n/useTranslation';
import type { DriveRoleFilter, PhysicalDriveInfo } from '@/models';

export interface StorageDeviceSelectorProps {
  readonly drives: readonly PhysicalDriveInfo[];
  readonly selectedDevicePath?: string | null;
  readonly roleFilter: DriveRoleFilter;
  readonly onChangeRoleFilter: (filter: DriveRoleFilter) => void;
  readonly loading: boolean;
  readonly unmountingPath?: string | null;
  readonly onRefresh: () => void;
  readonly onSelectDrive: (drive: PhysicalDriveInfo) => void;
  readonly onUnmountDrive?: (devicePath: string) => void;
  readonly allowSelectSystemDrive?: boolean;
  readonly compact?: boolean;
  readonly title?: string;
}

const ROLE_FILTERS: ReadonlyArray<{
  readonly id: DriveRoleFilter;
  readonly labelKey: TranslationKey;
}> = [
  { id: 'hard_disk', labelKey: 'hdd.devices.filterHdd' },
  { id: 'sd_card', labelKey: 'hdd.devices.filterSd' },
  { id: 'mounted', labelKey: 'hdd.devices.filterMounted' },
  { id: 'all', labelKey: 'hdd.devices.filterAll' },
];

export const StorageDeviceSelector: React.FC<StorageDeviceSelectorProps> = ({
  drives,
  selectedDevicePath,
  roleFilter,
  onChangeRoleFilter,
  loading,
  unmountingPath,
  onRefresh,
  onSelectDrive,
  onUnmountDrive,
  allowSelectSystemDrive = false,
  compact = false,
  title,
}) => {
  const { t } = useTranslation();

  return (
    <div className="space-y-2.5">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <fieldset className="flex flex-wrap items-center gap-1.5 border-0 p-0 m-0">
          <legend className="sr-only">{t('hdd.devices.filterAria')}</legend>
          {title && (
            <span className="text-[11px] font-semibold uppercase text-slate-400 mr-1.5">
              {title}
            </span>
          )}
          {ROLE_FILTERS.length === 0
            ? null
            : ROLE_FILTERS.map(({ id, labelKey }) => (
                <button
                  key={id}
                  type="button"
                  aria-pressed={roleFilter === id}
                  onClick={() => onChangeRoleFilter(id)}
                  className={`px-2 py-1 rounded-lg text-[10px] font-bold border transition-colors ${
                    roleFilter === id
                      ? 'bg-cyan-500/20 text-cyan-300 border-cyan-500/50'
                      : 'bg-slate-950 text-slate-400 border-slate-800 hover:border-slate-700 hover:text-slate-200'
                  }`}
                >
                  {t(labelKey)}
                </button>
              ))}
        </fieldset>

        <button
          type="button"
          onClick={onRefresh}
          disabled={loading}
          className="px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-750 text-slate-300 text-[11px] font-semibold flex items-center gap-1.5 transition-colors disabled:opacity-50"
        >
          <RefreshCw className={`w-3 h-3 ${loading ? 'animate-spin' : ''}`} />
          <span>{t('hdd.writer.refreshBtn')}</span>
        </button>
      </div>

      {drives.length === 0 ? (
        <div className="p-3 rounded-xl bg-slate-950 border border-slate-800 text-xs text-slate-500 text-center">
          {t('hdd.devices.emptyForFilter')}
        </div>
      ) : (
        <div className={`grid gap-2.5 ${compact ? 'grid-cols-1' : 'grid-cols-1 sm:grid-cols-2'}`}>
          {drives.map((drive) => {
            const locked = drive.is_system_drive && !allowSelectSystemDrive;
            const selectable = !locked && (allowSelectSystemDrive || drive.safe_to_write);
            const isSelected = selectedDevicePath === drive.device_path;
            const mounts = drive.mount_points ?? [];
            const isSd = drive.drive_role === 'sd_card';

            return (
              <div
                key={drive.device_path}
                className={`rounded-xl border transition-all ${
                  locked
                    ? 'bg-rose-950/20 border-rose-900/60 opacity-65'
                    : isSelected
                      ? 'bg-cyan-500/10 border-cyan-500/50 shadow-lg'
                      : 'bg-slate-950 border-slate-800 hover:border-slate-700'
                }`}
              >
                <button
                  type="button"
                  disabled={!selectable}
                  aria-pressed={isSelected}
                  onClick={() => selectable && onSelectDrive(drive)}
                  className={`w-full text-left ${compact ? 'p-2.5' : 'p-3.5'} ${
                    selectable ? 'cursor-pointer' : 'cursor-not-allowed'
                  }`}
                >
                  <div className="flex items-center justify-between gap-2 mb-1.5">
                    <span className="font-bold text-xs text-slate-200 flex items-center gap-1.5 truncate">
                      {drive.is_system_drive ? (
                        <ShieldAlert className="w-3.5 h-3.5 text-rose-400 shrink-0" />
                      ) : isSd ? (
                        <MemoryStick className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                      ) : (
                        <HardDrive className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
                      )}
                      <span className="truncate">{drive.display_name}</span>
                    </span>
                    <div className="flex items-center gap-1 shrink-0">
                      {drive.bus_type && (
                        <span className="px-1.5 py-0.5 rounded bg-slate-900 border border-slate-700 text-[9px] text-slate-300 font-bold">
                          {drive.bus_type}
                        </span>
                      )}
                      {drive.is_system_drive ? (
                        <StatusBadge variant="error" label={t('hdd.writer.badgeLocked')} />
                      ) : isSd ? (
                        <StatusBadge variant="info" label={t('hdd.devices.badgeSd')} />
                      ) : (
                        <StatusBadge variant="success" label={t('hdd.writer.badgeSafe')} />
                      )}
                    </div>
                  </div>

                  <div className="flex flex-wrap items-center gap-2 text-[11px] font-mono">
                    <span className="text-cyan-400">{drive.device_path}</span>
                    {mounts.length > 0 && (
                      <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-emerald-500/10 border border-emerald-500/30 text-[10px] text-emerald-300">
                        <FolderOpen className="w-2.5 h-2.5" />
                        <span>{mounts.join(', ')}</span>
                      </span>
                    )}
                  </div>

                  {drive.lockout_reason && (
                    <span className="text-[10px] text-rose-400 block mt-1">
                      {drive.lockout_reason}
                    </span>
                  )}
                </button>

                {onUnmountDrive && drive.is_mounted && !drive.is_system_drive && (
                  <div className="px-3 pb-2.5 pt-0 flex justify-end">
                    <button
                      type="button"
                      disabled={unmountingPath === drive.device_path}
                      onClick={() => onUnmountDrive(drive.device_path)}
                      className="px-2 py-1 rounded-lg bg-amber-500/10 hover:bg-amber-500/20 border border-amber-500/30 text-[10px] font-bold text-amber-300 flex items-center gap-1 transition-colors disabled:opacity-50"
                    >
                      <Unplug className="w-3 h-3" />
                      <span>
                        {unmountingPath === drive.device_path
                          ? t('hdd.writer.unmountingBtn')
                          : t('hdd.writer.unmountBtn')}
                      </span>
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
