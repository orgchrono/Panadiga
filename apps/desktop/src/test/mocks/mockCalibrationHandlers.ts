/**
 * Isolated mock handlers for Region Calibration (LBS), SD Kit Hardening, and TBC Bypass Lab pipelines.
 */

import { REGION_CALIBRATION_CONFIRM_TOKEN, TBC_BYPASS_CONFIRM_TOKEN } from '@/constants';
import type {
  ApplyMainTbcBypassesResponse,
  ApplyRegionCalibrationResponse,
  MainPassthroughMode,
  TbcApplianceMode,
} from '@/models';
import {
  isMainPassthroughMode,
  isTbcApplianceMode,
  isTvStandardSlug,
  readStringProp,
} from '@/utils/typeGuards';
import {
  buildCalibrationBundleReport,
  buildHardenKitResponse,
  MOCK_MAIN_BINARY_SIZE,
} from './mockCalibrationFixtures';
import { readMockRequest } from './mockEventBus';

const FCP_SITE_BYTES = 6;

export function handleMockCalibrationCommand(
  cmd: string,
  args?: Record<string, unknown>,
): unknown | undefined {
  const req = readMockRequest(args);
  switch (cmd) {
    case 'inspect_region_calibration_command':
      return buildCalibrationBundleReport(req);

    case 'apply_region_calibration_command': {
      const token = readStringProp(req, 'confirmToken') ?? '';
      if (token !== REGION_CALIBRATION_CONFIRM_TOKEN) {
        throw new Error(`Invalid calibration confirm token: expected ${REGION_CALIBRATION_CONFIRM_TOKEN}`);
      }
      const outDir = readStringProp(req, 'outputDir') ?? 'unbrick_out/calibrated';
      const audit = buildCalibrationBundleReport(req);
      const response: ApplyRegionCalibrationResponse = {
        outputDir: outDir,
        metadataAndDevOemOnly: true,
        devBinOutputPath: `${outDir}/ifodev2_calibrated.bin`,
        devBinReport: audit.dev_bin,
        mainOutputPath: `${outDir}/MAIN3410PQ_calibrated.bin`,
        mainReport: audit.main_region,
        frmOutputPath: null,
        frmReport: null,
        bundleAudit: audit,
      };
      return response;
    }

    case 'harden_sd_unbrick_kit_command':
      return buildHardenKitResponse(req);

    case 'apply_main_tbc_bypasses_command': {
      const token = readStringProp(req, 'confirmToken') ?? '';
      if (token !== TBC_BYPASS_CONFIRM_TOKEN) {
        throw new Error(`Invalid TBC bypass confirm token: expected ${TBC_BYPASS_CONFIRM_TOKEN}`);
      }
      const modeRaw = readStringProp(req, 'applianceMode') ?? 'digadiag_ram_tbc';
      const applianceMode: TbcApplianceMode = isTbcApplianceMode(modeRaw)
        ? modeRaw
        : 'digadiag_ram_tbc';
      const passRaw = readStringProp(req, 'passthroughMode') ?? 'av_passthrough';
      const passMode: MainPassthroughMode = isMainPassthroughMode(passRaw)
        ? passRaw
        : 'av_passthrough';
      const tvRaw = readStringProp(req, 'tvStandard') ?? 'ntsc';
      const cowOut = readStringProp(req, 'outputMainPath') ?? 'unbrick_out/MAIN3410PQ_STANDALONE_TBC.bin';

      const response: ApplyMainTbcBypassesResponse = {
        appliancePlan: {
          mode: applianceMode,
          tv_standard: isTvStandardSlug(tvRaw) ? tvRaw : 'ntsc',
          requires_experimental_flag: applianceMode === 'experimental_sd_ram_chainload',
          warnings: [],
          steps: ['1. Patch 5 standalone TBC/FPC/ODD/HDD bypasses (CoW)', '2. Keep NOR SA0..SA3 untouched'],
          mmio_ops: [],
          nor_sa0_to_sa3_untouched: true,
          verified_main_passthrough_sites: [
            {
              name: 'fpc_status_ctrl',
              va_hex: '0x5771ABFB',
              file_offset_hex: '0x71ABFB',
              length_bytes: FCP_SITE_BYTES,
              note: passMode,
            },
          ],
        },
        mainPatchReport: {
          file_size: MOCK_MAIN_BINARY_SIZE,
          expected_size_match: true,
          detected_mode: passMode.toUpperCase(),
          sites: [],
          output_path: cowOut,
          applied_mode: passMode.toUpperCase(),
        },
        cowOutputPath: cowOut,
      };
      return response;
    }

    default:
      return undefined;
  }
}
