/**
 * Isolated mock handlers for Live Pico 142-Sector NOR Dump, BGO FTL Audit/Repair, and DAVIS IDE Probing.
 */

import {
  SPANSION_BGO_EXPECTED_P2_COUNT,
  SPANSION_BGO_FIRST_SA,
  SPANSION_BGO_LAST_SA,
  SPANSION_BGO_L2P_SA_PRIMARY,
  SPANSION_BGO_L2P_SA_SECONDARY,
  SPANSION_64KB_SECTOR_BYTES,
  UPPER_NOR_CONFIRM_TOKEN,
} from '@/constants';
import type {
  BgoRepairPlan,
  DavisTaskfileSnapshot,
  Eh55BgoAuditReport,
  Eh55BgoSectorReport,
  PicoEh55BgoRepairResponse,
  PicoEh55NorDumpResponse,
  PicoProbeDavisIdeResponse,
} from '@/models';
import { readBooleanProp, readStringProp } from '@/utils/typeGuards';
import { publishMockEvent, readMockRequest } from './mockEventBus';

const P1_ACTIVE_COUNT = 2;
const L2P_ACTIVE_PAGES = 1024;
const NFROM_COUNT = 12;
const BGO_WINDOW_START_VA = 0x801a_0000;
const BGO_WINDOW_END_VA = 0x8047_0000;
const NOR_BASE_PHYS = 0x8000_0000;
const NOR_DUMP_TOTAL_LEN = 0x0080_0000;
const BGO_TOTAL_LEN = 0x002d_0000;
const MOCK_POST_BGO_SHA = '11223344556677889900aabbccddeeff11223344556677889900aabbccddeeff';
const MOCK_NOR_SHA = 'ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100';
const MOCK_BACKUP_SHA = '99887766554433221100ffeeddccbbaa99887766554433221100ffeeddccbbaa';
const WRITE_BYTES_LEN = 64;
const SYS_BEFORE = 0x0000_000e;
const SYS_AFTER = 0x0000_0000;
const ST3250_LBA28 = 268_435_455;
const ST3250_LBA48 = 488_397_168;
const ST3250_BYTES = 250_059_350_016;
const ST3250_MIRROR_LBA = 57_344;
const WORD63_MWDMA2 = 0x0407;
const WORD88_UDMA5 = 0x203f;
const IDENTIFY_WORD0 = 0x0040;
const STATUS_ATA_READY = 0x50;
const STATUS_FLOATING_FF = 0xff;
const DEVSEL_MASTER = 0xa0;
const DEVSEL_SLAVE = 0xb0;
const ATAPI_MID = 0x14;
const ATAPI_HIGH = 0xeb;
const MOCK_PROGRESS_STEPS = 4;

function buildBgoAuditReport(): Eh55BgoAuditReport {
  const totalBgoSectors = SPANSION_BGO_LAST_SA - SPANSION_BGO_FIRST_SA + 1;
  const p2Ids = Array.from({ length: SPANSION_BGO_EXPECTED_P2_COUNT }, (_, idx) => idx + 1);
  const sectors: readonly Eh55BgoSectorReport[] = Array.from({ length: totalBgoSectors }, (_, idx) => {
    const sa = SPANSION_BGO_FIRST_SA + idx;
    const isP1 = sa === SPANSION_BGO_L2P_SA_PRIMARY || sa === SPANSION_BGO_L2P_SA_SECONDARY;
    const p2Id = idx - P1_ACTIVE_COUNT;
    const kind = isP1
      ? 'Partition1Active'
      : p2Id >= 1 && p2Id <= SPANSION_BGO_EXPECTED_P2_COUNT
        ? 'Partition2'
        : 'Spare';
    return {
      sa,
      flash_offset: sa * SPANSION_64KB_SECTOR_BYTES,
      phys_va: NOR_BASE_PHYS + sa * SPANSION_64KB_SECTOR_BYTES,
      sector_id: isP1 ? idx : p2Id >= 1 && p2Id <= SPANSION_BGO_EXPECTED_P2_COUNT ? p2Id : 0xffff,
      kind,
      migration_idx: null,
      migration_target: 0xffff,
      zero_marker_ok: true,
    };
  });

  return {
    healthy: true,
    total_sectors: totalBgoSectors,
    p1_active_count: P1_ACTIVE_COUNT,
    p1_migrating_count: 0,
    p2_present_ids: p2Ids,
    p2_missing_ids: [],
    spare_sas: [totalBgoSectors - 3 + SPANSION_BGO_FIRST_SA, SPANSION_BGO_LAST_SA],
    corrupt_sas: [],
    active_l2p_pages: L2P_ACTIVE_PAGES,
    l2p_tag_mismatches: 0,
    fat_mirror_mismatches: 0,
    nfrom_file_count: NFROM_COUNT,
    ifodev2_model: 'EH55',
    sectors,
  };
}

function buildRepairPlan(audit: Eh55BgoAuditReport): BgoRepairPlan {
  return {
    bgo_window_start_va: BGO_WINDOW_START_VA,
    bgo_window_end_va: BGO_WINDOW_END_VA,
    target_sectors: [SPANSION_BGO_FIRST_SA, SPANSION_BGO_L2P_SA_SECONDARY],
    source_nor_sha256: MOCK_NOR_SHA,
    source_bgo_sha256: MOCK_BACKUP_SHA,
    expected_post_sha256: MOCK_POST_BGO_SHA,
    expected_post_nor_sha256: MOCK_NOR_SHA,
    missing_f2_ids: [],
    assigned_spare_sas: [],
    erased_sas: [],
    total_write_bytes: WRITE_BYTES_LEN,
    ops: [
      {
        op: 'write_bytes',
        sa: SPANSION_BGO_FIRST_SA,
        phys_va: BGO_WINDOW_START_VA,
        data: [0, 1, 2, 3],
        requires_erase_first: false,
      },
    ],
    post_audit: audit,
  };
}

function makeTaskfile(status: number, devsel: number, lbamid = 0, lbahi = 0): DavisTaskfileSnapshot {
  return {
    data: 0,
    err: 0,
    seccnt: 1,
    lbalo: 1,
    lbamid,
    lbahi,
    devsel,
    status,
    altstat: status,
    sentinel: 0,
  };
}

export function handleMockBenchExtensionsCommand(
  cmd: string,
  args?: Record<string, unknown>,
): unknown | undefined {
  const req = readMockRequest(args);
  switch (cmd) {
    case 'pico_eh55_nor_dump_command': {
      const port = readStringProp(req, 'portName') ?? 'COM10';
      const slice = readStringProp(req, 'slice') ?? 'bgo';
      const outPath = readStringProp(req, 'outputPath') ?? 'unbrick_out/eh55_nor_bgo_cow.bin';
      const len = slice === 'full' ? NOR_DUMP_TOTAL_LEN : BGO_TOTAL_LEN;
      publishMockEvent('nor://dump-progress', {
        port,
        stage: 'PANADMP1 capture complete',
        current: MOCK_PROGRESS_STEPS,
        total: MOCK_PROGRESS_STEPS,
      });
      const response: PicoEh55NorDumpResponse = {
        outputPath: outPath,
        startVaHex: slice === 'full' ? '0x80000000' : '0x801A0000',
        totalLen: len,
        sha256: MOCK_NOR_SHA,
        slices: [
          {
            start_va: slice === 'full' ? NOR_BASE_PHYS : BGO_WINDOW_START_VA,
            len,
            sha256: MOCK_NOR_SHA,
            bridge_summary: null,
            checkpoint_path: null,
            resumed_from_checkpoint: false,
          },
        ],
      };
      return response;
    }

    case 'pico_eh55_bgo_audit_command':
      return buildBgoAuditReport();

    case 'pico_eh55_bgo_repair_command': {
      const dryRunOnly = readBooleanProp(req, 'dryRunOnly') ?? true;
      const audit = buildBgoAuditReport();
      const plan = buildRepairPlan(audit);
      if (dryRunOnly) {
        const dryRes: PicoEh55BgoRepairResponse = {
          dryRunOnly: true,
          sa0ToSa3LockedE05: true,
          plan,
          outPlanPath: 'unbrick_out/bgo_repair_plan.json',
          cowOutPath: null,
          liveReport: null,
        };
        return dryRes;
      }

      const token = readStringProp(req, 'confirmToken') ?? '';
      if (token !== UPPER_NOR_CONFIRM_TOKEN) {
        throw new Error(`Invalid Upper NOR unlock token: expected ${UPPER_NOR_CONFIRM_TOKEN}`);
      }
      const backupDir = readStringProp(req, 'backupDir') ?? 'unbrick_out/bgo_pre_repair_backup';
      publishMockEvent('bgo://repair-progress', {
        stage: 'complete',
        current: MOCK_PROGRESS_STEPS,
        total: MOCK_PROGRESS_STEPS,
        detail: 'Upper NOR repaired & locked (U LOCK)',
      });
      const liveRes: PicoEh55BgoRepairResponse = {
        dryRunOnly: false,
        sa0ToSa3LockedE05: true,
        plan,
        outPlanPath: 'unbrick_out/bgo_repair_plan.json',
        cowOutPath: null,
        liveReport: {
          backup_path: `${backupDir}/pre_bgo_backup.bin`,
          backup_sha256: MOCK_BACKUP_SHA,
          pre_bgo_sha256: MOCK_NOR_SHA,
          post_bgo_sha256: plan.expected_post_sha256,
          erased_sectors: 0,
          written_bytes: WRITE_BYTES_LEN,
          boot_trace_output: 'BOOTTRACE 15000 NOJIG -> BOOT NORMAL 0x9000_0000 OK',
          boot_normal_ok: true,
        },
      };
      return liveRes;
    }

    case 'pico_probe_davis_ide_command': {
      const port = readStringProp(req, 'portName') ?? 'COM10';
      const drive = readStringProp(req, 'drive') ?? 'master';
      const payload: PicoProbeDavisIdeResponse = {
        port,
        driveSelHex: drive === 'slave' ? '0xB0' : '0xA0',
        lethal0xc000100cBlocked: true,
        report: {
          c0001008_before: SYS_BEFORE,
          c0001008_after: SYS_AFTER,
          fe_released: true,
          ch0_taskfile: makeTaskfile(STATUS_ATA_READY, DEVSEL_MASTER, ATAPI_MID, ATAPI_HIGH),
          ch1_raw: makeTaskfile(STATUS_ATA_READY, DEVSEL_MASTER),
          ch1_master: makeTaskfile(STATUS_ATA_READY, DEVSEL_MASTER),
          ch1_slave: makeTaskfile(STATUS_FLOATING_FF, DEVSEL_SLAVE),
          ch1_2b_reset: null,
          ch1_2c_reset: null,
          fun_cd4d_rc: 0,
          ch1_struct_hex: '0x51801200',
          identify_sentinel: STATUS_ATA_READY,
          identify_post_altstat: STATUS_ATA_READY,
          identify_info: {
            word0: IDENTIFY_WORD0,
            is_atapi: false,
            model: 'ST3250820ACE',
            serial_number: '5QE12345',
            firmware_revision: '3.ADA',
            lba28_sectors: ST3250_LBA28,
            lba48_supported: true,
            lba48_sectors: ST3250_LBA48,
            effective_sectors: ST3250_LBA48,
            capacity_bytes: ST3250_BYTES,
            mirror_main_lba: ST3250_MIRROR_LBA,
            word63_mwdma: WORD63_MWDMA2,
            mwdma2_supported: true,
            mwdma_active_mode: 2,
            word88_udma: WORD88_UDMA5,
            udma_supported_mask: 0x3f,
            udma_active_mode: 5,
          },
          classification: 'ata_ready',
          step_logs: [
            '[GUARD] 0xC000100C lethal write permanently locked out',
            '[DAVIS] Released F/E halt bits on 0xC0001008 (0x0000000E -> 0x00000000)',
            '[ATA] CH1 0xEC IDENTIFY DEVICE OK: ST3250820ACE MWDMA2 supported',
          ],
        },
      };
      publishMockEvent('davis://probe-result', payload);
      return payload;
    }

    default:
      return undefined;
  }
}
