/**
 * Stateless pure mock IPC handlers for Segregated Forensic Vaults, Mandatory Mn103Vm Pre-Write Gate,
 * Streaming Composed MEIHFS HDD Flash, ISO 9660 Level 1 (1x) DVD Creator, and Relational JSON Hardware Profiles.
 */

import type {
  ComposedHddFlashReport,
  DvdCreatorReport,
  NorFingerprintReport,
  OpticalBurnerDriveInfo,
  ProfileAuthorizationOutcome,
  RegistryValidationReport,
  VaultArtifactRecord,
  VaultBucket,
} from '@/models';
import { readBooleanProp, readStringProp } from '@/utils/typeGuards';
import { publishMockEvent, readMockRequest } from './mockEventBus';
import {
  MOCK_BIN_BYTES,
  MOCK_CREATED_TS,
  MOCK_DIGADIAG_RAW_BYTES,
  MOCK_DVD_1X_KBPS,
  MOCK_DVD_4X_KBPS,
  MOCK_FRM_BYTES,
  MOCK_HDD_DISK_BYTES,
  MOCK_HDD_MIRROR_LBA,
  MOCK_HDD_SECTORS,
  MOCK_ISO_BYTES,
  MOCK_ISO_EXTENT_LBA,
  MOCK_ISO_FILE_SECTORS,
  MOCK_ISO_SECTORS,
  MOCK_ISO_SECTOR_SIZE,
  MOCK_MAIN_WRITTEN_BYTES,
  MOCK_PERCENT_100,
  MOCK_PROBE_BPS,
  MOCK_PROFILE_BOARD_COUNT,
  MOCK_PROFILE_CHECKS_COUNT,
  MOCK_PROFILE_EEPROM_COUNT,
  MOCK_PROFILE_FLASH_COUNT,
  MOCK_PROFILE_LAYOUT_COUNT,
  MOCK_PROFILE_MODEL_COUNT,
  MOCK_PROFILE_SOC_CORE_COUNT,
  MOCK_PROFILE_SOC_COUNT,
  MOCK_PROFILE_TRAIN_COUNT,
  MOCK_SECTOR_64K,
  MOCK_SHA_A,
  MOCK_SHA_B,
  MOCK_SHA_C,
  buildEh55ResolvedProfile,
  buildMockPrewriteGate,
  buildMockSession,
  extractBasename,
  parseMockBucket,
  parseMockStep,
  resolveLoaderProfile,
} from './mockVaultFixtures';

export function handleMockVaultCommand(
  cmd: string,
  args?: Record<string, unknown>,
): unknown | undefined {
  const req = readMockRequest(args);
  switch (cmd) {
    case 'vault_init_session_command': {
      const modelId = readStringProp(req, 'modelId') ?? 'DMR-EH55';
      const loaderProfile = resolveLoaderProfile(readStringProp(req, 'loaderProfile'));
      const sessionId =
        readStringProp(req, 'sessionId') ?? `sess_${modelId.toLowerCase().replace(/[^a-z0-9]+/g, '_')}`;
      return buildMockSession(sessionId, modelId, loaderProfile);
    }

    case 'vault_list_sessions_command':
      return [buildMockSession('sess_dmr_eh55_default', 'DMR-EH55', 'ver061')];

    case 'vault_get_session_command': {
      const sid = readStringProp(args ?? {}, 'sessionId') ?? 'sess_dmr_eh55_default';
      return buildMockSession(sid, 'DMR-EH55', 'ver061');
    }

    case 'vault_ingest_file_command': {
      const src = readStringProp(req, 'sourcePath') ?? 'ic5001.bin';
      const filename = extractBasename(src) || 'ic5001.bin';
      const isFrm = filename.toLowerCase().endsWith('.frm');
      const override = parseMockBucket(readStringProp(req, 'bucketOverride'));
      if (isFrm && override === 'raw') {
        throw new Error(`Vault segregation violation: .FRM file '${filename}' belongs in frm/, NEVER raw/`);
      }
      if (!isFrm && override === 'frm') {
        throw new Error(`Vault segregation violation: frm/ accepts ONLY .FRM files (got '${filename}')`);
      }
      const bucket: VaultBucket = override ?? (isFrm ? 'frm' : 'raw');
      const rec: VaultArtifactRecord = {
        id: `${bucket}:${filename}`,
        bucket,
        filename,
        relativePath: `${bucket}/${filename}`,
        sizeBytes: isFrm ? MOCK_FRM_BYTES : MOCK_BIN_BYTES,
        sha256: isFrm ? MOCK_SHA_B : MOCK_SHA_A,
        origin: readStringProp(req, 'origin') ?? 'desktop_manual_ingest',
        createdAtEpochS: MOCK_CREATED_TS,
        emuVerified: bucket === 'recompiled',
      };
      return rec;
    }

    case 'vault_mark_step_command': {
      const sid = readStringProp(args ?? {}, 'sessionId') ?? 'sess_dmr_eh55_default';
      const step = parseMockStep(readStringProp(args ?? {}, 'step'));
      return {
        ...buildMockSession(sid, 'DMR-EH55', 'ver061'),
        completedSteps: ['select_model', step],
        currentStep: step,
      };
    }

    case 'vault_run_prewrite_emu_gate_command': {
      const kind = readStringProp(req, 'targetKind') ?? 'cleanroom_digadiag';
      return buildMockPrewriteGate(kind, true);
    }

    case 'flash_composed_hdd_safe_command': {
      const devPath = readStringProp(req, 'devicePath') ?? '';
      if (!devPath || devPath.includes('PhysicalDrive0')) {
        throw new Error('FATAL SAFETY LOCKOUT: PhysicalDrive0 is permanently blocked.');
      }
      const token = (readStringProp(req, 'confirmDiskModelToken') ?? '').trim();
      const shortDev = devPath.replace(/^\\\\.\\/, '');
      const expected = `CONFIRM-${shortDev}`;
      if (token !== expected && token !== shortDev) {
        throw new Error(`FATAL SAFETY LOCKOUT: expected ${expected}`);
      }
      publishMockEvent('hdd://flash-progress', {
        devicePath: devPath,
        stage: 'done',
        bytesDone: MOCK_SECTOR_64K,
        bytesTotal: MOCK_SECTOR_64K,
        detail: 'MEIHFS streaming composition & SHA-256 readback verified',
      });
      const report: ComposedHddFlashReport = {
        devicePath: devPath,
        diskBytes: MOCK_HDD_DISK_BYTES,
        totalSectors: MOCK_HDD_SECTORS,
        busTypeId: 7,
        mirrorMainLba: MOCK_HDD_MIRROR_LBA,
        backupBinPath: 'backups/cow_64k_backup.bin',
        backupManifestPath: 'backups/cow_64k_manifest.json',
        backup64kSha256: MOCK_SHA_C,
        layoutSha256: MOCK_SHA_A,
        p1MainBytesWritten: MOCK_MAIN_WRITTEN_BYTES,
        p1MainSha256: MOCK_SHA_A,
        p2DrvBytesWritten: 0,
        p2DrvSha256: null,
        mirrorMainBytesWritten: MOCK_MAIN_WRITTEN_BYTES,
        mirrorMainSha256: MOCK_SHA_A,
        emuGate: buildMockPrewriteGate('composed_hdd_stream', true),
        readbackVerified: true,
      };
      return report;
    }

    case 'inspect_optical_burners_command': {
      const burners: readonly OpticalBurnerDriveInfo[] = [
        {
          deviceId: '/dev/sr0',
          displayName: 'HL-DT-ST DVDRAM GH24NSC0',
          osBackend: 'mock_imapi2',
          supportedWriteSpeedsKbps: [MOCK_DVD_1X_KBPS, MOCK_DVD_4X_KBPS],
          minWriteSpeedKbps: MOCK_DVD_1X_KBPS,
          minWriteSpeedLabel: '1385 KB/s (1x DVD)',
          safeOpticalTarget: true,
          expectedConfirmToken: 'CONFIRM-sr0',
        },
      ];
      return burners;
    }

    case 'create_dvd_recovery_disc_command': {
      const burnerId = readStringProp(req, 'burnerId');
      if (burnerId) {
        const expected = `CONFIRM-${burnerId.replace(/^\/dev\//, '')}`;
        if ((readStringProp(req, 'confirmToken') ?? '').trim() !== expected) {
          throw new Error(`DVD SAFETY LOCKOUT: confirm_token must equal '${expected}'`);
        }
      }
      const target = burnerId ?? (readStringProp(req, 'outputIsoPath') ?? 'backups/eh55_dvd_recovery_iso9660.iso');
      const model = readStringProp(req, 'model') ?? 'DMR-EH55';
      const volumeLabel = readStringProp(req, 'volumeLabel') ?? 'PANADIGA_REC';
      const cleanroomOnly = readBooleanProp(req, 'cleanroomOnly') ?? true;
      publishMockEvent('dvd://burn-progress', {
        target,
        stage: 'readback_verify',
        percent: MOCK_PERCENT_100,
        detail: 'ISO 9660 Level 1 (1x DAO) verified',
      });
      const rep: DvdCreatorReport = {
        target,
        targetKind: burnerId ? 'optical_disc' : 'iso_file',
        model,
        volumeLabel,
        cleanroomOnly,
        bytesWritten: MOCK_ISO_BYTES,
        isoSha256: MOCK_SHA_A,
        writeSpeedPolicy: '1x_or_drive_minimum_dao',
        burnCommandPreview: burnerId ? `mock --speed=1 --dev=${burnerId}` : null,
        files: [
          {
            name_8_3: 'DIGADIAG.FRM',
            extent_lba: MOCK_ISO_EXTENT_LBA,
            size_bytes: MOCK_DIGADIAG_RAW_BYTES,
            sector_count: MOCK_ISO_FILE_SECTORS,
            sha256: MOCK_SHA_A,
          },
        ],
        emuGate: buildMockPrewriteGate('cleanroom_digadiag', true),
        inspection: {
          valid_pvd: true,
          valid_terminator: true,
          volume_id: volumeLabel,
          logical_block_size: MOCK_ISO_SECTOR_SIZE,
          volume_space_sectors: MOCK_ISO_SECTORS,
          files: [
            {
              name_8_3: 'DIGADIAG.FRM',
              extent_lba: MOCK_ISO_EXTENT_LBA,
              size_bytes: MOCK_DIGADIAG_RAW_BYTES,
              sector_count: MOCK_ISO_FILE_SECTORS,
              sha256: MOCK_SHA_A,
            },
          ],
          overall_status: 'PASS',
        },
        nextSteps: [
          '1. Burn this verified ISO 9660 Level 1 .iso strictly in DAO mode at 1x (or lowest drive speed).',
          `2. Insert disc into ${model} to run recovery update.`,
        ],
      };
      return rep;
    }

    case 'list_hardware_profiles_command':
      return [buildEh55ResolvedProfile()];

    case 'get_resolved_model_profile_command':
      return buildEh55ResolvedProfile();

    case 'validate_hardware_profiles_command': {
      const val: RegistryValidationReport = {
        valid: true,
        socCoreCount: MOCK_PROFILE_SOC_CORE_COUNT,
        socCount: MOCK_PROFILE_SOC_COUNT,
        flashChipCount: MOCK_PROFILE_FLASH_COUNT,
        eepromChipCount: MOCK_PROFILE_EEPROM_COUNT,
        firmwareTrainCount: MOCK_PROFILE_TRAIN_COUNT,
        norLayoutCount: MOCK_PROFILE_LAYOUT_COUNT,
        mediaFormatCount: MOCK_PROFILE_LAYOUT_COUNT,
        boardCount: MOCK_PROFILE_BOARD_COUNT,
        modelCount: MOCK_PROFILE_MODEL_COUNT,
        checksRun: MOCK_PROFILE_CHECKS_COUNT,
        checksFailed: 0,
        safetyChecksPassed: MOCK_PROFILE_CHECKS_COUNT,
        errors: [],
      };
      return val;
    }

    case 'authorize_profile_capability_command': {
      const req = readMockRequest(args);
      const modelId =
        readStringProp(req, 'modelId') ?? readStringProp(req, 'model_id') ?? 'DMR-EH55';
      const rawCap = readStringProp(req, 'capability') ?? 'inspect_read_only';
      const capability =
        rawCap === 'nvram_sanitize_offline_cow' ||
        rawCap === 'nvram_write_live' ||
        rawCap === 'upper_nor_patch_offline_cow' ||
        rawCap === 'upper_nor_write_live' ||
        rawCap === 'official_frm_pass_through' ||
        rawCap === 'custom_recovery_media' ||
        rawCap === 'critical_ipl_or_lethal_mmio'
          ? rawCap
          : 'inspect_read_only';
      const blocked =
        modelId === 'DMR-EZ485V' ||
        modelId === 'DMR-ES10' ||
        capability === 'critical_ipl_or_lethal_mmio';
      const outcome: ProfileAuthorizationOutcome = blocked
        ? {
            allowed: false,
            authorization: null,
            denial: {
              modelId,
              capability,
              reason: `Mock policy denied ${capability} on ${modelId}`,
              labOverrideEligible: false,
            },
          }
        : {
            allowed: true,
            authorization: {
              modelId,
              capability,
              mode: 'verified_policy',
              effectiveTier: 'bench_verified',
              warnings: [],
            },
            denial: null,
          };
      return outcome;
    }

    case 'fingerprint_nor_dump_command': {
      const fp: NorFingerprintReport = {
        detectedModel: 'DMR-EH55',
        displayLabel: 'DMR-EH55',
        isSizeOnly: false,
        matchedProbe: 'loader_banner:Loader ver0.61@0x0',
        hasEemulF1Marker: true,
        dumpSizeBytes: MOCK_BIN_BYTES,
        candidates: [
          {
            modelId: 'DMR-EH55',
            matchedProbe: 'loader_banner:Loader ver0.61@0x0',
            confidenceBps: MOCK_PROBE_BPS,
          },
        ],
      };
      return fp;
    }

    default:
      return undefined;
  }
}
