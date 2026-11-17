import { describe, it, expect } from 'vitest';
import {
  filterDrivesByRoleFilter,
  resolvePrimaryDriveTarget,
} from './useStorageDrives';
import type { PhysicalDriveInfo } from '@/models';

const MOCK_DRIVES: PhysicalDriveInfo[] = [
  {
    device_path: '/dev/nvme0n1',
    display_name: 'WD SN770 (NVMe, 1000.2 GB)',
    size_bytes: 1_000_204_886_016,
    size_gb: 1000.2,
    is_system_drive: true,
    safe_to_write: false,
    drive_role: 'system_os',
    is_mounted: true,
    mount_points: ['/'],
  },
  {
    device_path: '/dev/sda',
    display_name: 'Hitachi HDT722525DLAT80 (USB, 250.1 GB)',
    size_bytes: 250_059_350_016,
    size_gb: 250.1,
    is_system_drive: false,
    safe_to_write: true,
    drive_role: 'hard_disk',
    bus_type: 'USB',
    is_mounted: true,
    mount_points: ['/media/daniel/DIGA_HDD'],
    partitions: [
      {
        device_path: '/dev/sda1',
        label: 'DIGA_HDD',
        fs_type: 'ext4',
        mount_point: '/media/daniel/DIGA_HDD',
        size_bytes: 250_000_000_000,
      },
    ],
  },
  {
    device_path: '/dev/mmcblk0',
    display_name: 'SD2GB (MMC/SD, 2.0 GB)',
    size_bytes: 2_002_780_160,
    size_gb: 2.0,
    is_system_drive: false,
    safe_to_write: true,
    drive_role: 'sd_card',
    bus_type: 'MMC/SD',
    removable: true,
    is_mounted: true,
    mount_points: ['/run/media/daniel/PANASONIC'],
    partitions: [
      {
        device_path: '/dev/mmcblk0p1',
        label: 'PANASONIC',
        fs_type: 'vfat',
        mount_point: '/run/media/daniel/PANASONIC',
        size_bytes: 2_000_000_000,
      },
    ],
  },
];

describe('useStorageDrives pure role filtering and target resolution', () => {
  it('filters drives cleanly by hard_disk, sd_card, mounted, and all', () => {
    expect(filterDrivesByRoleFilter(MOCK_DRIVES, 'hard_disk').map(d => d.device_path)).toEqual([
      '/dev/sda',
    ]);
    expect(filterDrivesByRoleFilter(MOCK_DRIVES, 'sd_card').map(d => d.device_path)).toEqual([
      '/dev/mmcblk0',
    ]);
    expect(filterDrivesByRoleFilter(MOCK_DRIVES, 'mounted')).toHaveLength(3);
    expect(filterDrivesByRoleFilter(MOCK_DRIVES, 'all')).toHaveLength(3);
  });

  it('prefers mount point or partition for SD inspection while returning raw block path for HDD', () => {
    expect(resolvePrimaryDriveTarget(MOCK_DRIVES[2], true)).toEqual({
      diskTarget: '/run/media/daniel/PANASONIC',
      mountPoint: '/run/media/daniel/PANASONIC',
    });
    expect(resolvePrimaryDriveTarget(MOCK_DRIVES[1], false)).toEqual({
      diskTarget: '/dev/sda',
      mountPoint: '/media/daniel/DIGA_HDD',
    });
  });
});
