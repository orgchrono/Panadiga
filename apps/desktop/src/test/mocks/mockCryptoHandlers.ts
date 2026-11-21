/**
 * Mock IPC handlers for PANA_DVD.FRM cryptanalysis, Spansion NVRAM, and DIGA model catalog.
 */

import {
  mockCodebookReport,
  mockCryptanalysisReport,
  mockFrmTocDumpReport,
  mockNvramCleanReport,
} from './mockData';

export function handleMockCryptoCommand(
  cmd: string,
  args?: Record<string, unknown>,
): unknown | undefined {
  switch (cmd) {
    case 'clean_nvram':
    case 'analyze_flash_dump':
      return mockNvramCleanReport;
    case 'patch_nvram_command': {
      const inputPath = String(args?.inputPath ?? args?.input_path ?? '').trim();
      const outputPath = String(args?.outputPath ?? args?.output_path ?? '').trim();
      if (!inputPath || !outputPath || inputPath === outputPath) {
        throw new Error('Copy-on-Write obrigatório: defina um output_path diferente do input_path.');
      }
      const rawOffset = args?.nvramOffset ?? args?.nvram_offset ?? 0;
      if (typeof rawOffset !== 'number' || !Number.isInteger(rawOffset) || rawOffset < 0) {
        throw new Error('nvramOffset inválido: esperado número inteiro u32 não-negativo.');
      }
      const parsedOffset = rawOffset;
      const patchHex = String(args?.patchHex ?? args?.patch_hex ?? '').replace(/\s+/g, '');
      const patchBytes = Math.ceil(patchHex.length / 2);
      if (
        !Number.isFinite(parsedOffset) ||
        parsedOffset < 0 ||
        parsedOffset + patchBytes > 65536
      ) {
        throw new Error(
          'E05: Escrita bloqueada além dos 64 KiB de NVRAM (região IPL Spansion 0x80000000..0x807EFFFF somente leitura).',
        );
      }
      return {
        ...mockNvramCleanReport,
        input_file: inputPath,
        output_file: outputPath,
        bootloader_immutable: true,
      };
    }

    case 'analyze_frm':
      return mockCryptanalysisReport;

    case 'dump_frm_toc':
      return mockFrmTocDumpReport;

    case 'get_codebook':
      return mockCodebookReport;

    case 'list_frm_containers_command':
      return ['firmware_and_images/PANA_DVD.FRM'];

    case 'pack_frm_command':
      return {
        mode: 'sd',
        total_bytes: 67584,
        toc_region_size: 2048,
        modules: [
          {
            index: 0,
            name: 'DIAG01',
            module_offset: 2048,
            module_size: 65536,
            fvu_checksum_u32: 0x12345678,
            list_sum_u16: 0x0abc,
          },
        ],
      };

    case 'unpack_extrhead_command':
      return {
        file_path: 'firmware_and_images/DIGADIAG.FRM',
        header: {
          tag: 'FVU ',
          flag: 0,
          algo: 1,
          dst_len: 1024,
          dst_addr: 0x50400000,
          src_len: 512,
          footer_offset: 576,
          header_checksum: 0x12345678,
          adler_step: 16,
          build_name: 'DIAG01',
        },
        decompressed_bytes: 1024,
        calculated_checksum_hex: '0x12345678',
        header_checksum_hex: '0x12345678',
        checksum_valid: true,
        footer_present: true,
        written_output_path: null,
      };

    case 'get_diga_model_catalog':
      return [
        {
          model_id: 'AUTO',
          series: 'Auto-Detect (Structural Probe & Loader Banner Fingerprinting)',
          flash_size_mb: 8,
          flash_size_bytes: 8388608,
          nvram_base_hex: 'AUTO (Probe-Verified)',
          sector_geometry: 'Probe-Detected (Derived from Resolved Flash Chip)',
          soc_name: 'Matsushita AM33/AM34 / LSI DoMiNo',
        },
        {
          model_id: 'DMR-EH55',
          series: 'Série EH (HDD+DVD)',
          flash_size_mb: 8,
          flash_size_bytes: 8388608,
          nvram_base_hex: '0x847F0000',
          sector_geometry: 'S29JL064H70TFI00 [8x8KB + 126x64KB + 8x8KB (S29JL064H70TFI00, SA0..SA141)] — NVRAM SA134..SA141',
          soc_name: 'MN10300/AM34 (MN2DS0009)',
        },
        {
          model_id: 'DMR-ES35V',
          series: 'Série ES (DVD+VHS)',
          flash_size_mb: 8,
          flash_size_bytes: 8388608,
          nvram_base_hex: '0x847F0000',
          sector_geometry: 'S29JL064H70TFI00 [8x8KB + 126x64KB + 8x8KB (S29JL064H70TFI00, SA0..SA141)] — NVRAM SA134..SA141',
          soc_name: 'MN10300/AM34 (MN2DS00151DB)',
        },
        {
          model_id: 'DMR-ES10',
          series: 'Série ES (DVD-RAM)',
          flash_size_mb: 1,
          flash_size_bytes: 1048576,
          nvram_base_hex: 'READ_ONLY_OTP',
          sector_geometry: 'MR27V802 (1 MiB Read-Only OTP PROM, No NVRAM)',
          soc_name: 'MN10300/AM33-2 (MN2DS0012-H)',
        },
        {
          model_id: 'DMR-EZ485V',
          series: 'Série EZ (LSI DoMiNo DVD+VHS)',
          flash_size_mb: 4,
          flash_size_bytes: 4194304,
          nvram_base_hex: 'UNKNOWN',
          sector_geometry: 'MX29LV320BT/BB [8x8KB + 63x64KB (MX29LV320B, SA0..SA70)]',
          soc_name: 'SPARC V8 (LSI DoMiNo) (LSI DoMiNo ZEBRA64)',
        },
        {
          model_id: 'DMR-E85H',
          series: 'Série ExxH (HDD+DVD)',
          flash_size_mb: 0,
          flash_size_bytes: 0,
          nvram_base_hex: 'UNKNOWN',
          sector_geometry: 'UNKNOWN (Flash/Layout Unverified — Inspect Only)',
          soc_name: 'UNKNOWN',
        },
        {
          model_id: 'DMR-EX75',
          series: 'Série EX (HDD+DVB-T)',
          flash_size_mb: 0,
          flash_size_bytes: 0,
          nvram_base_hex: 'UNKNOWN',
          sector_geometry: 'UNKNOWN (Flash/Layout Unverified — Inspect Only)',
          soc_name: 'UNKNOWN',
        },
      ];

    default:
      return undefined;
  }
}
