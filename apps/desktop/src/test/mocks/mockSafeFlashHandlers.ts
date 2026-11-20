/**
 * Isolated mock handlers for Safe Target Disk discovery & CoW 64 KiB pre-flash EH55 flashing.
 */

import { BUS_TYPE_NVME_ID, HDD_CONFIRM_TOKEN_PREFIX } from '@/constants';
import type { Eh55HddFlashReport, SafeTargetDiskInfo } from '@/models';
import { readStringProp } from '@/utils/typeGuards';
import { publishMockEvent, readMockRequest } from './mockEventBus';

const BUS_TYPE_SATA_ID = 11;
const DISK0_SIZE_GB = 500;
const DISK1_SIZE_GB = 1000;
const DISK2_SIZE_GB = 250;
const GIB_BYTES = 1_073_741_824;
const PRIMARY_BYTES = 104_857_600;
const MIRROR_BYTES = 512;
const MIRROR_LBA = 57_344;

const MOCK_SAFE_DISKS: readonly SafeTargetDiskInfo[] = [
  {
    device_path: '\\\\.\\PhysicalDrive0',
    display_name: 'SYSTEM OS DISK0',
    size_bytes: DISK0_SIZE_GB * GIB_BYTES,
    size_gb: DISK0_SIZE_GB,
    bus_type: 'SATA',
    bus_type_id: BUS_TYPE_SATA_ID,
    is_nvme_blocked: false,
    is_os_blocked: true,
    safe_to_flash: false,
    lockout_reason: 'OS_BOOT_DISK_LOCKED (PhysicalDrive0)',
    mirror_main_lba: null,
  },
  {
    device_path: '\\\\.\\PhysicalDrive1',
    display_name: 'SAMSUNG NVMe MZVLB1T0',
    size_bytes: DISK1_SIZE_GB * GIB_BYTES,
    size_gb: DISK1_SIZE_GB,
    bus_type: 'NVMe',
    bus_type_id: BUS_TYPE_NVME_ID,
    is_nvme_blocked: true,
    is_os_blocked: false,
    safe_to_flash: false,
    lockout_reason: 'NVME_BUS_17_LOCKED',
    mirror_main_lba: null,
  },
  {
    device_path: '\\\\.\\PhysicalDrive2',
    display_name: 'ST3250820ACE SATA',
    size_bytes: DISK2_SIZE_GB * GIB_BYTES,
    size_gb: DISK2_SIZE_GB,
    bus_type: 'SATA',
    bus_type_id: BUS_TYPE_SATA_ID,
    is_nvme_blocked: false,
    is_os_blocked: false,
    safe_to_flash: true,
    lockout_reason: null,
    mirror_main_lba: MIRROR_LBA,
  },
];

export function handleMockSafeFlashCommand(
  cmd: string,
  args?: Record<string, unknown>,
): unknown | undefined {
  switch (cmd) {
    case 'inspect_safe_target_disks_command':
      return MOCK_SAFE_DISKS;

    case 'flash_eh55_hdd_safe_command': {
      const req = readMockRequest(args);
      const devicePath = readStringProp(req, 'devicePath') ?? '\\\\.\\PhysicalDrive2';
      const expectedToken = `${HDD_CONFIRM_TOKEN_PREFIX}${devicePath.replace(/^\\\\.\\/, '')}`;
      const token = readStringProp(req, 'confirmDiskModelToken') ?? '';
      if (token !== expectedToken) {
        throw new Error(`Invalid HDD flash confirm token: expected ${expectedToken}`);
      }
      const backupDir = readStringProp(req, 'backupDir') ?? 'unbrick_out/hdd_pre_flash_backup';
      publishMockEvent('hdd://flash-progress', {
        devicePath,
        stage: 'verify_sha256',
        bytesDone: PRIMARY_BYTES,
        bytesTotal: PRIMARY_BYTES,
        detail: 'Read-back SHA-256 verification succeeded (Primary + Mirror)',
      });
      const report: Eh55HddFlashReport = {
        device_path: devicePath,
        disk_bytes: DISK2_SIZE_GB * GIB_BYTES,
        bus_type_id: BUS_TYPE_SATA_ID,
        mirror_main_lba: MIRROR_LBA,
        backup_bin_path: `${backupDir}/pre_flash_64k_backup.bin`,
        backup_manifest_path: `${backupDir}/pre_flash_backup_sha256.json`,
        backup_64k_sha256: '102030405060708090a0b0c0d0e0f001102030405060708090a0b0c0d0e0f001',
        primary_bytes_written: PRIMARY_BYTES,
        primary_sha256: 'aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899',
        mirror_bytes_written: MIRROR_BYTES,
        mirror_sha256: 'ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100',
        readback_verified: true,
      };
      return report;
    }

    default:
      return undefined;
  }
}
