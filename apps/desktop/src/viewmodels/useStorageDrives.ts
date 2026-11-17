import { useCallback, useEffect, useMemo, useState } from 'react';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import type { DriveRoleFilter, PhysicalDriveInfo } from '@/models';
import {
  listHddDrives,
  listPhysicalDrives,
  listSdCardDrives,
  unmountPhysicalDrive,
} from '@/services/storageCryptoService';
import {
  filterDrivesByRoleFilter,
  resolvePrimaryDriveTarget,
} from '@/utils/storageDrives';

export { filterDrivesByRoleFilter, resolvePrimaryDriveTarget };

/**
 * Reusable MVVM hook managing physical & mounted storage enumeration, role filtering, and safe unmounting.
 */
export function useStorageDrives(
  defaultFilter: DriveRoleFilter = 'all',
  autoFetch = true,
) {
  const { showToast } = useToast();
  const [drives, setDrives] = useState<PhysicalDriveInfo[]>([]);
  const [roleFilter, setRoleFilter] = useState<DriveRoleFilter>(defaultFilter);
  const [loading, setLoading] = useState<boolean>(false);
  const [unmountingPath, setUnmountingPath] = useState<string | null>(null);

  const refresh = useCallback(
    async (explicitRole?: DriveRoleFilter, signal?: AbortSignal): Promise<PhysicalDriveInfo[]> => {
      setLoading(true);
      try {
        const rawList =
          explicitRole === 'hard_disk'
            ? await listHddDrives()
            : explicitRole === 'sd_card'
              ? await listSdCardDrives()
              : await listPhysicalDrives();
        const list = Array.isArray(rawList) ? rawList : [];
        if (signal?.aborted) return [];
        setDrives(list);
        return list;
      } catch (err) {
        if (signal?.aborted) return [];
        showToast({
          type: 'error',
          title: translate('errors.hdd.listDisksError'),
          message: err instanceof Error ? err.message : String(err),
        });
        return [];
      } finally {
        if (!signal?.aborted) {
          setLoading(false);
        }
      }
    },
    [showToast],
  );

  const unmountDrive = useCallback(
    async (devicePath: string): Promise<boolean> => {
      setUnmountingPath(devicePath);
      try {
        const unmounted = await unmountPhysicalDrive(devicePath);
        showToast({
          type: 'success',
          title: translate('hdd.writer.unmountSuccessTitle'),
          message:
            unmounted.length > 0
              ? unmounted.join(', ')
              : translate('hdd.writer.unmountEmptyMsg', { device: devicePath }),
        });
        await refresh();
        return true;
      } catch (err) {
        showToast({
          type: 'error',
          title: translate('hdd.writer.unmountErrorTitle'),
          message: err instanceof Error ? err.message : String(err),
        });
        return false;
      } finally {
        setUnmountingPath(null);
      }
    },
    [refresh, showToast],
  );

  useEffect(() => {
    if (!autoFetch) return;
    const controller = new AbortController();
    void Promise.resolve().then(() => {
      if (!controller.signal.aborted) {
        void refresh(undefined, controller.signal);
      }
    });
    return () => {
      controller.abort();
    };
  }, [autoFetch, refresh]);

  const filteredDrives = useMemo(
    () => filterDrivesByRoleFilter(drives, roleFilter),
    [drives, roleFilter],
  );

  return {
    drives,
    filteredDrives,
    roleFilter,
    setRoleFilter,
    loading,
    unmountingPath,
    refresh,
    unmountDrive,
  };
}
