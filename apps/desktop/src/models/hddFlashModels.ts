/**
 * @file models/hddFlashModels.ts
 * @brief Safe target disk lockout and 5 GB..2 TB physical MEIHFS flash DTOs.
 */

export interface SafeTargetDiskInfo {
  readonly device_path: string;
  readonly display_name: string;
  readonly size_bytes: number;
  readonly size_gb: number;
  readonly bus_type: string;
  readonly bus_type_id: number;
  readonly is_nvme_blocked: boolean;
  readonly is_os_blocked: boolean;
  readonly safe_to_flash: boolean;
  readonly lockout_reason: string | null;
  readonly mirror_main_lba: number | null;
}

export type Eh55FlashStage =
  | 'preflight'
  | 'backup64k'
  | 'wipe_layout'
  | 'write100m'
  | 'write_mirror'
  | 'verify_sha256'
  | 'done';

export interface HddFlashProgressEvent {
  readonly devicePath: string;
  readonly stage: Eh55FlashStage;
  readonly bytesDone: number;
  readonly bytesTotal: number;
  readonly detail: string;
}

export interface FlashEh55HddSafeRequest {
  readonly devicePath: string;
  readonly confirmDiskModelToken: string;
  readonly imagePath?: string;
  readonly mirrorP1Path?: string;
  readonly backupDir: string;
  readonly expectedMinGb?: number;
  readonly expectedMaxGb?: number;
  readonly allowExtended4tb?: boolean;
  readonly nonceId?: string;
}

export interface Eh55HddFlashReport {
  readonly device_path: string;
  readonly disk_bytes: number;
  readonly bus_type_id: number;
  readonly mirror_main_lba: number;
  readonly backup_bin_path: string;
  readonly backup_manifest_path: string;
  readonly backup_64k_sha256: string;
  readonly primary_bytes_written: number;
  readonly primary_sha256: string;
  readonly mirror_bytes_written: number;
  readonly mirror_sha256: string | null;
  readonly readback_verified: boolean;
}
