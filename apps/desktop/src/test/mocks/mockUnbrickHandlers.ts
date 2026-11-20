/**
 * Mock IPC handlers for the Pre-MAIN DIGA Unbrick Workbench, SD Kit Builder, & TBC Passthrough.
 */

import type {
  FrmModuleExtractionReport,
  NorDumperStubResult,
  SdDiskInspectionReport,
  SdUnbrickKitReport,
} from '@/models';
import { handleMockMatshitaCommand } from './mockMatshitaHandlers';
import {
  MOCK_MAIN_PASSTHROUGH,
  MOCK_RSP_PLAN,
  MOCK_SD_VERIFICATION,
  MOCK_UNBRICK_WORKSPACE,
} from './mockUnbrickData';

export function handleMockUnbrickCommand(
  cmd: string,
  args?: Record<string, unknown>
): unknown | undefined {
  switch (cmd) {
    case 'inspect_unbrick_workbench':
      return MOCK_UNBRICK_WORKSPACE;

    case 'extract_frm_modules_command': {
      const outDir = (args?.outDir as string) || (args?.out_dir as string) || 'unbrick_extracted';
      const rep: FrmModuleExtractionReport = {
        source_frm_path: (args?.frmPath as string) || 'firmware_and_images/PANA_DVD.FRM',
        container_format: 'PANA_DVD_48B_FEISTEL',
        output_dir: outDir,
        modules: [
          {
            name: 'DRV S211',
            offset_hex: '0x00001800',
            encrypted_size: 720_896,
            decrypted_path: `${outDir}/drv_s211_dec.bin`,
            decrypted_sha256: 'aa11'.repeat(16),
            unpacked_path: `${outDir}/drv_s211_unpacked.bin`,
            unpacked_size: 1_441_792,
            dst_addr_hex: '0x90202000',
            checksum_valid: true,
          },
          {
            name: 'PROG061',
            offset_hex: '0x000B1800',
            encrypted_size: 262_144,
            decrypted_path: `${outDir}/prog061_dec.bin`,
            decrypted_sha256: 'c7da'.repeat(16),
            unpacked_path: null,
            unpacked_size: null,
            dst_addr_hex: null,
            checksum_valid: null,
          },
        ],
      };
      return rep;
    }

    case 'prepare_sd_unbrick_kit_command': {
      const req = (args?.request as Record<string, unknown>) || {};
      const outDir = (req.output_dir as string) || 'firmware_and_images/sd_unbrick_kit';
      const bgoMode = (req.bgo_mode as string) || 'bypass';
      const passthroughMode = (req.main_passthrough_mode as string) || 'av_passthrough';
      const loaderProfile = (req.loader_profile as string) || 'ver061';
      const includeSd = req.include_pana_sd !== false;
      const unlockDyn = req.unlock_hdd_dynamic !== false;
      const rep: SdUnbrickKitReport = {
        output_dir: outDir,
        prog061_source: 'firmware_and_images/PANA_DVD.FRM',
        main_source: includeSd ? 'firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin' : null,
        drv_source: includeSd ? 'drv_s211_dec.bin' : null,
        bgo_mode_applied: bgoMode,
        loader_profile_applied: loaderProfile,
        main_passthrough_applied: includeSd ? passthroughMode : null,
        hdd_capacity_applied: includeSd && unlockDyn ? 'dynamic' : null,
        digadiag_path: `${outDir}/DIGADIAG.FRM`,
        digadiag_sha256: 'deadbeef'.repeat(8),
        pana_sd_path: includeSd ? `${outDir}/PANA__SD.FRM` : null,
        pana_sd_sha256: includeSd ? 'cafebabe'.repeat(8) : null,
        sha256sums_path: `${outDir}/SHA256SUMS.txt`,
        verification: { ...MOCK_SD_VERIFICATION, sd_root_dir: outDir, pana_sd_present: includeSd },
      };
      return rep;
    }

    case 'verify_sd_unbrick_media_command': {
      const sdDir = (args?.sdRootDir as string) || (args?.sd_root_dir as string) || 'firmware_and_images/sd_unbrick_kit';
      return { ...MOCK_SD_VERIFICATION, sd_root_dir: sdDir };
    }

    case 'inspect_sd_disk_command': {
      const target = String(args?.target ?? 'E:');
      const rep: SdDiskInspectionReport = {
        target,
        partition_scheme: 'MBR',
        mbr_partitions: [
          {
            slot: 0,
            boot_flag: 0x00,
            partition_type: 0x06,
            type_name: 'FAT16 (>=32MB)',
            lba_start: 2048,
            sector_count: 3_909_632,
          },
        ],
        active_partition_lba: 2048,
        bpb: {
          oem_name: 'MSWIN4.1',
          bytes_per_sector: 512,
          sectors_per_cluster: 64,
          cluster_size_bytes: 32_768,
          reserved_sectors: 1,
          num_fats: 2,
          root_entry_count: 512,
          total_sectors: 3_909_632,
          media_descriptor: 0xf8,
          sectors_per_fat: 240,
          hidden_sectors: 2048,
          fat_type_label: 'FAT16',
          volume_label: 'NO NAME',
          data_clusters: 61_080,
          computed_fat_kind: 'FAT16',
        },
        digadiag_entry: {
          name_8_3: 'DIGADIAG.FRM',
          attr: 0x20,
          hidden: false,
          system: false,
          read_only: false,
          first_cluster: 2,
          file_size: 131_072,
          has_preceding_lfn: false,
          chain_clusters: [2, 3, 4, 5],
          chain_contiguous: true,
          chain_complete: true,
        },
        pana_sd_entry: {
          name_8_3: 'PANA__SD.FRM',
          attr: 0x20,
          hidden: false,
          system: false,
          read_only: false,
          first_cluster: 6,
          file_size: 7_183_272,
          has_preceding_lfn: false,
          chain_clusters: [6, 7],
          chain_contiguous: true,
          chain_complete: true,
        },
        root_files_sample: ['DIGADIAG.FRM', 'PANA__SD.FRM'],
        digadiag_payload_verified: true,
        overall_status: 'PASS',
        checks: [
          {
            id: 'fat16_layout',
            severity: 'pass',
            title: 'MBR / BPB FAT16 SDSC OK',
            detail: 'FAT16 (cluster 32 KiB), DIGADIAG.FRM 131072 B contíguo e completo.',
          },
        ],
      };
      return rep;
    }

    case 'patch_main_passthrough_command': {
      const action = ((args?.action as string) || 'inspect').toLowerCase();
      if (action === 'rsp_plan') {
        const cb = args?.callbackTargetVa as string | undefined;
        return {
          ...MOCK_RSP_PLAN,
          callback_write_frame: cb ? `$M515B2E40,4:${cb.replace(/^0x/i, '').toLowerCase()}#00` : null,
        };
      }
      return {
        ...MOCK_MAIN_PASSTHROUGH,
        detected_mode: action === 'patch' ? 'AV_PASSTHROUGH' : MOCK_MAIN_PASSTHROUGH.detected_mode,
        output_path: action === 'patch' ? ((args?.outPath as string) || 'EH55_MAIN_PASSTHROUGH.bin') : null,
        applied_mode: action === 'patch' ? ((args?.mode as string) || 'av_passthrough') : null,
      };
    }

    case 'build_nor_dumper_stub_command': {
      const wrap = Boolean(args?.wrapAsDigadiag ?? args?.wrap_as_digadiag);
      const rep: NorDumperStubResult = {
        start_addr: (args?.startAddr as string) || '0x84000000',
        total_len_bytes: 8_388_608,
        block_size: 4096,
        load_addr: (args?.loadAddr as string) || (wrap ? '0x50400000' : '0x51800000'),
        raw_stub_bytes: 168,
        wrapped_digadiag: wrap,
        output_bytes: wrap ? 131_072 : 168,
        written_path: (args?.outPath as string) || null,
        stub_hex: '44500050400000...',
      };
      return rep;
    }

    default:
      return handleMockMatshitaCommand(cmd, args);
  }
}
