import type { DriveRoleFilter, PhysicalDriveInfo } from '@/models';

/**
 * Pure filter classifying discovered drives by media role or mount state.
 */
export function filterDrivesByRoleFilter(
  drives: readonly PhysicalDriveInfo[],
  filter: DriveRoleFilter,
): PhysicalDriveInfo[] {
  const safeDrives = Array.isArray(drives) ? drives : [];
  if (filter === 'all') {
    return [...safeDrives];
  }
  if (filter === 'mounted') {
    return safeDrives.filter(
      d => Boolean(d.is_mounted) || (d.mount_points?.length ?? 0) > 0,
    );
  }
  return safeDrives.filter(d => d.drive_role === filter);
}

/**
 * Resolves the preferred disk/mount target string for a selected drive.
 */
export function resolvePrimaryDriveTarget(
  drive: PhysicalDriveInfo,
  preferMountOrPartition: boolean,
): { diskTarget: string; mountPoint: string | null } {
  const firstMount =
    drive.mount_points?.find(m => m.trim().length > 0) ??
    drive.partitions?.find(p => p.mount_point.trim().length > 0)?.mount_point ??
    null;
  const firstPartition = drive.partitions?.[0]?.device_path;
  if (preferMountOrPartition) {
    return {
      diskTarget: firstMount ?? firstPartition ?? drive.device_path,
      mountPoint: firstMount,
    };
  }
  return {
    diskTarget: drive.device_path,
    mountPoint: firstMount,
  };
}
