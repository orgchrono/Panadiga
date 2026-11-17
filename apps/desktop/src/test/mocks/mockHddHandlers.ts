/**
 * Mock IPC handlers for HDD MEIHDFS inspection, extraction, and physical drive operations.
 */

import { PartitionExtractionManifest } from '@/models';
import {
  mockHddAnalysisReport,
  mockMeihdfsAllocationReport,
} from './mockData';

export function handleMockHddCommand(
  cmd: string,
  args?: Record<string, unknown>
): unknown | undefined {
  switch (cmd) {
    case 'inspect_hdd':
      return mockHddAnalysisReport;

    case 'decode_meihdfs_allocations':
      return mockMeihdfsAllocationReport;

    case 'extract_partitions': {
      const manifest: PartitionExtractionManifest = {
        timestamp: new Date().toISOString(),
        sourceImage: 'firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin',
        outputDirectory: 'firmware_and_images/extracted_partitions/',
        manifestSha256Sums:
          '499e1d8820cbf018a77d1219eeea103980bc235e182390aef92348a011234bc9 *partition_1_am34_os.bin\n' +
          '7721839048102938471920394857102938471920394857102938471920394857 *partition_2_fonts_microcode.bin',
        extractedPartitions: [
          {
            id: 1,
            name: 'Partition 1 (AM34 Main Firmware)',
            filename: 'partition_1_am34_os.bin',
            offsetHex: '0x02000000',
            sizeBytes: 6173264,
            sha256: '499e1d8820cbf018a77d1219eeea103980bc235e182390aef92348a011234bc9',
            md5: '72dae89b21f3089daef3810237aa01e2',
            entropy: 7.9942,
          },
          {
            id: 2,
            name: 'Partition 2 (OSD Fonts & Microcode)',
            filename: 'partition_2_fonts_microcode.bin',
            offsetHex: '0x04000000',
            sizeBytes: 720896,
            sha256: '7721839048102938471920394857102938471920394857102938471920394857',
            md5: '3c8919f2010938472910384719283746',
            entropy: 7.8214,
          },
        ],
      };
      return manifest;
    }

    case 'list_physical_drives':
    case 'list_hdd_drives':
    case 'list_sd_card_drives': {
      const mockAll = [
        {
          device_path: '\\\\.\\PhysicalDrive0',
          display_name: 'Physical Drive 0 (Host Windows NVMe, 512.0 GB)',
          size_bytes: 512110190592,
          size_gb: 512.0,
          is_system_drive: true,
          safe_to_write: false,
          lockout_reason: 'LOCKED: Windows Host Operating System Boot Drive (PhysicalDrive0)',
          drive_role: 'system_os',
          bus_type: 'NVMe',
          removable: false,
          is_mounted: true,
          mount_points: ['C:\\'],
          partitions: [
            {
              device_path: '\\\\.\\C:',
              label: 'OS',
              fs_type: 'NTFS',
              mount_point: 'C:\\',
              size_bytes: 512110190592,
            },
          ],
        },
        {
          device_path: '\\\\.\\PhysicalDrive1',
          display_name: 'Physical Drive 1 (PATA/IDE Panasonic DIGA Target, 80.0 GB)',
          size_bytes: 80026361856,
          size_gb: 80.0,
          is_system_drive: false,
          safe_to_write: true,
          lockout_reason: null,
          drive_role: 'hard_disk',
          bus_type: 'USB',
          removable: false,
          is_mounted: false,
          mount_points: [],
          partitions: [],
        },
        {
          device_path: '\\\\.\\PhysicalDrive2',
          display_name: 'Generic SD/MMC Card Reader (USB, 2.0 GB)',
          size_bytes: 2002780160,
          size_gb: 2.0,
          is_system_drive: false,
          safe_to_write: true,
          lockout_reason: null,
          drive_role: 'sd_card',
          bus_type: 'MMC/SD',
          removable: true,
          is_mounted: true,
          mount_points: ['E:\\'],
          partitions: [
            {
              device_path: '\\\\.\\E:',
              label: 'DIGA_SD',
              fs_type: 'FAT',
              mount_point: 'E:\\',
              size_bytes: 2002780160,
            },
          ],
        },
      ];
      if (cmd === 'list_hdd_drives' || args?.role === 'hdd') {
        return mockAll.filter((d) => d.drive_role === 'hard_disk');
      }
      if (cmd === 'list_sd_card_drives' || args?.role === 'sd') {
        return mockAll.filter((d) => d.drive_role === 'sd_card');
      }
      return mockAll;
    }

    case 'unmount_physical_drive':
      return [`${String(args?.devicePath ?? '\\\\.\\PhysicalDrive2')} (unmounted)`];

    case 'format_hdd_meihdfs':
    case 'write_hdd_image':
    case 'restore_hdd_mirrors':
      return {
        device_path: (args?.devicePath as string) || '\\\\.\\PhysicalDrive1',
        operation: cmd,
        bytes_written: 104857600,
        sectors_written: (args?.totalSectors as number) || 204800,
        success: true,
        message: 'Operação física de gravação MEIHDFS executada com sucesso.',
      };

    case 'inspect_hdd_capacity_patch':
      return {
        input_path:
          (args?.inputPath as string) ||
          'firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin',
        container_format: 'EXTRHEAD_CONTAINER',
        build_name: 'MAIN3410PQ',
        raw_main_size: 13003052,
        raw_main_sha256: 'c3275d39549b68f48db16bc5fd043c6f8a373d4ba416826de09be5c55dc991b1',
        patch_site_file_offset: 5099964,
        patch_site_va_hex: '0x574DD1FC',
        max_product_lba_var_va_hex: '0x5311B9AC',
        physical_blocks_var_va_hex: '0x5311BC80',
        clamp_branch_opcode_hex: '0xC6',
        is_dynamic_unclamped: false,
        slot_400gb_blocks_2kb: 0x0ba43b74,
        slot_400gb_hex: '0x0BA43B74',
        slot_400gb_gb: 400.0,
        slot_200gb_blocks_2kb: 0x05d21dba,
        slot_200gb_hex: '0x05D21DBA',
        slot_200gb_gb: 200.0,
        slot_fallback_blocks_2kb: 0x04a6d7c8,
        slot_fallback_hex: '0x04A6D7C8',
        slot_fallback_gb: 159.91,
        effective_default_limit_gb: 200.0,
        rtsd_presets: [
          { menu_index: 1, label: '400GB HDD (RTSD)', capacity_gb: 400, blocks_2kb: 0x0ba3fb74, blocks_2kb_hex: '0x0BA3FB74', sectors_512b: 781184464 },
          { menu_index: 2, label: '300GB HDD (RTSD)', capacity_gb: 300, blocks_2kb: 0x08baec97, blocks_2kb_hex: '0x08BAEC97', sectors_512b: 585871964 },
          { menu_index: 3, label: '250GB HDD (RTSD)', capacity_gb: 250, blocks_2kb: 0x07466528, blocks_2kb_hex: '0x07466528', sectors_512b: 488215712 },
          { menu_index: 4, label: '200GB HDD (RTSD)', capacity_gb: 200, blocks_2kb: 0x05d1ddba, blocks_2kb_hex: '0x05D1DDBA', sectors_512b: 390559464 },
          { menu_index: 5, label: '160GB HDD (RTSD)', capacity_gb: 160, blocks_2kb: 0x04a7d7c8, blocks_2kb_hex: '0x04A7D7C8', sectors_512b: 312434464 },
          { menu_index: 6, label: '137GB HDD (RTSD)', capacity_gb: 137, blocks_2kb: 0x03fc7ac3, blocks_2kb_hex: '0x03FC7AC3', sectors_512b: 267512588 },
          { menu_index: 7, label: '120GB HDD (RTSD)', capacity_gb: 120, blocks_2kb: 0x037dd1d6, blocks_2kb_hex: '0x037DD1D6', sectors_512b: 234309464 },
          { menu_index: 8, label: '100GB HDD (RTSD)', capacity_gb: 100, blocks_2kb: 0x02e8cedd, blocks_2kb_hex: '0x02E8CEDD', sectors_512b: 195246964 },
          { menu_index: 9, label: ' 80GB HDD (RTSD)', capacity_gb: 80, blocks_2kb: 0x0253cbe4, blocks_2kb_hex: '0x0253CBE4', sectors_512b: 156184464 },
        ],
      };

    case 'apply_hdd_capacity_patch': {
      const mode = (args?.mode as string) || 'dynamic';
      const targetGb = (args?.targetGb as number | undefined) ?? (mode === 'cap' ? 250 : undefined);
      const isDynamic = mode === 'dynamic';
      const appliedBlocks =
        mode === 'factory_200gb'
          ? 0x05d21dba
          : targetGb === 250
            ? 0x07466528
            : targetGb
              ? Math.floor((targetGb * 1_000_000_000) / 2048)
              : 0x3fffffff;
      const before = handleMockHddCommand('inspect_hdd_capacity_patch', args) as Record<string, unknown>;
      return {
        input_path: before.input_path,
        output_path:
          (args?.outputPath as string) ||
          'firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000_PATCHED_DYNAMIC.bin',
        container_format: 'EXTRHEAD_CONTAINER',
        mode,
        target_gb: targetGb ?? null,
        applied_blocks_2kb: appliedBlocks,
        applied_blocks_2kb_hex: `0x${appliedBlocks.toString(16).toUpperCase().padStart(8, '0')}`,
        applied_sectors_512b: appliedBlocks * 4,
        mbr_superblocks_updated: false,
        repacked_container_size: 6173296,
        passthrough_mode:
          (args?.passthroughMode as string) || (args?.passthrough_mode as string) || null,
        original_sha256: '6ec63de059c00b287a0e8a1a4102a8007bbccf3210a552bfb45118a03b85c898',
        patched_sha256: 'a1b2c3d4e5f60718293a4b5c6d7e8f90112233445566778899aabbccddeeff00',
        inspection_before: before,
        inspection_after: {
          ...before,
          clamp_branch_opcode_hex: isDynamic ? '0xCA' : '0xC6',
          is_dynamic_unclamped: isDynamic,
          slot_200gb_blocks_2kb: appliedBlocks,
          slot_200gb_hex: `0x${appliedBlocks.toString(16).toUpperCase().padStart(8, '0')}`,
          slot_200gb_gb: targetGb ?? 2199.02,
          effective_default_limit_gb: targetGb ?? 2199.02,
        },
      };
    }

    default:
      return undefined;
  }
}
