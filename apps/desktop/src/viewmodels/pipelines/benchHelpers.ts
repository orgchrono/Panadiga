/**
 * @file viewmodels/pipelines/benchHelpers.ts
 * @brief Pure helpers for Spansion 142-sector BGO FTL mapping, F2 integrity (40/820), and DAVIS MMIO decoding.
 */

import {
  DAVIS_MMIO_SPECS,
  HEX32_PAD_WIDTH,
  SPANSION_BGO_EXPECTED_P2_COUNT,
  SPANSION_BGO_EXPECTED_P2_SUM,
  SPANSION_BGO_L2P_SA_PRIMARY,
  SPANSION_BGO_L2P_SA_SECONDARY,
  SPANSION_IPL_LAST_SA,
  SPANSION_NVRAM_FIRST_SA,
} from '@/constants';
import { translate } from '@/i18n/useTranslation';
import type {
  DavisChannelId,
  DavisProbeReport,
  DavisTaskfileSnapshot,
  Eh55BgoAuditReport,
  FlashSectorInfo,
  HardwareRegisterSnapshot,
} from '@/models';
import { formatSpansionHexAddress, mapSpansion142Sectors } from '@/utils/spansionTopology';

const HEX_BYTE_PAD = 2;
const FLOATING_IDE_STATUS_FF = 0xff;
const DEAD_IDE_STATUS_ZERO = 0x00;

export interface F2IntegritySummary {
  readonly count: number;
  readonly sum: number;
  readonly expectedCount: number;
  readonly expectedSum: number;
  readonly valid: boolean;
}

export function formatHex(value: number, width: number = HEX_BYTE_PAD): string {
  return `0x${value.toString(16).toUpperCase().padStart(width, '0')}`;
}

export function computeF2Integrity(presentIds: readonly number[]): F2IntegritySummary {
  const uniqueValid = presentIds.filter(
    (id, index, arr) => id >= 1 && id <= SPANSION_BGO_EXPECTED_P2_COUNT && arr.indexOf(id) === index,
  );
  const sum = uniqueValid.reduce((acc, id) => acc + id, 0);
  return {
    count: uniqueValid.length,
    sum,
    expectedCount: SPANSION_BGO_EXPECTED_P2_COUNT,
    expectedSum: SPANSION_BGO_EXPECTED_P2_SUM,
    valid:
      uniqueValid.length === SPANSION_BGO_EXPECTED_P2_COUNT &&
      sum === SPANSION_BGO_EXPECTED_P2_SUM,
  };
}

export function buildBgo142SectorMap(audit: Eh55BgoAuditReport | null): FlashSectorInfo[] {
  const bySa = audit ? new Map(audit.sectors.map(sec => [sec.sa, sec])) : null;
  const roNote = translate('pipelines.bgoBench.sa0Sa3LockedBadge');
  return mapSpansion142Sectors((desc, baseType) => {
    if (desc.sa <= SPANSION_IPL_LAST_SA) {
      return {
        sectorType: 'bootloader',
        isCorrupted: false,
        activeBytes: desc.sizeBytes,
        status: 'immutable',
        note: `${roNote} • ${formatSpansionHexAddress(desc.uncachedVa)}`,
      };
    }
    if (desc.sa >= SPANSION_NVRAM_FIRST_SA) {
      return {
        sectorType: 'nvram_microsector',
        isCorrupted: false,
        activeBytes: desc.sizeBytes,
        status: 'clean',
        note: `${desc.writePolicy} • ${formatSpansionHexAddress(desc.uncachedVa)}`,
      };
    }
    const bgoSec = bySa?.get(desc.sa);
    if (!bgoSec) {
      return {
        sectorType: baseType,
        isCorrupted: false,
        activeBytes: desc.sizeBytes,
        status: 'clean',
        note: `${desc.writePolicy} • ${formatSpansionHexAddress(desc.uncachedVa)}`,
      };
    }
    const isCorrupt = bgoSec.kind === 'Corrupt' || !bgoSec.zero_marker_ok;
    const isL2p =
      desc.sa === SPANSION_BGO_L2P_SA_PRIMARY || desc.sa === SPANSION_BGO_L2P_SA_SECONDARY;
    return {
      sectorType: 'bgo_region',
      isCorrupted: isCorrupt,
      activeBytes: bgoSec.kind === 'Spare' ? 0 : desc.sizeBytes,
      status: isCorrupt ? 'corrupted' : 'factory',
      note: `${bgoSec.kind} • id=${bgoSec.sector_id}${isL2p ? ' [L2P]' : ''}`,
    };
  });
}

function classifyStatusSeverity(statusByte: number): HardwareRegisterSnapshot['severity'] {
  if (statusByte === FLOATING_IDE_STATUS_FF || statusByte === DEAD_IDE_STATUS_ZERO) return 'critical';
  if ((statusByte & DAVIS_MMIO_SPECS.statusBsyBit) !== 0) return 'warning';
  if ((statusByte & DAVIS_MMIO_SPECS.statusErrBit) !== 0) return 'warning';
  if ((statusByte & DAVIS_MMIO_SPECS.statusDrdyBit) !== 0) return 'ok';
  return 'info';
}

function buildTaskfileRows(
  prefix: string,
  baseAddrHex: string,
  snap: DavisTaskfileSnapshot,
): HardwareRegisterSnapshot[] {
  const bsy = (snap.status & DAVIS_MMIO_SPECS.statusBsyBit) !== 0 ? 1 : 0;
  const drdy = (snap.status & DAVIS_MMIO_SPECS.statusDrdyBit) !== 0 ? 1 : 0;
  const drq = (snap.status & DAVIS_MMIO_SPECS.statusDrqBit) !== 0 ? 1 : 0;
  const err = (snap.status & DAVIS_MMIO_SPECS.statusErrBit) !== 0 ? 1 : 0;
  return [
    {
      id: `${prefix}-status`,
      name: `${prefix}_STATUS`,
      addressHex: baseAddrHex,
      valueHex: formatHex(snap.status),
      widthBits: 8,
      statusLabel: `BSY=${bsy} DRDY=${drdy}`,
      severity: classifyStatusSeverity(snap.status),
      flagBreakdown: `DRQ=${drq} ERR=${err} ALT=${formatHex(snap.altstat)}`,
    },
    {
      id: `${prefix}-devsel`,
      name: `${prefix}_DRIVE_HEAD`,
      addressHex: baseAddrHex,
      valueHex: formatHex(snap.devsel),
      widthBits: 8,
      statusLabel: (snap.devsel & DAVIS_MMIO_SPECS.driveHeadSlaveBit) !== 0 ? 'SLAVE' : 'MASTER',
      severity: 'info',
      flagBreakdown: `ERR_REG=${formatHex(snap.err)} SECCNT=${formatHex(snap.seccnt)}`,
    },
    {
      id: `${prefix}-lba`,
      name: `${prefix}_LBA_SIG`,
      addressHex: baseAddrHex,
      valueHex: `${formatHex(snap.lbahi)}:${formatHex(snap.lbamid)}:${formatHex(snap.lbalo)}`,
      widthBits: 24,
      statusLabel:
        snap.lbamid === DAVIS_MMIO_SPECS.atapiLbaMid && snap.lbahi === DAVIS_MMIO_SPECS.atapiLbaHigh
          ? 'ATAPI'
          : 'ATA',
      severity: 'info',
      flagBreakdown: `SENTINEL=${formatHex(snap.sentinel, HEX32_PAD_WIDTH)}`,
    },
  ];
}

export function mapDavisReportToRegisters(
  report: DavisProbeReport | null,
  channel: DavisChannelId,
): HardwareRegisterSnapshot[] {
  if (!report) return [];
  const activeSnap = channel === 'ch0' ? report.ch0_taskfile : report.ch1_master;
  const baseAddr = channel === 'ch0' ? DAVIS_MMIO_SPECS.ch0StatusHex : DAVIS_MMIO_SPECS.ch1StatusHex;
  const sysRows: readonly HardwareRegisterSnapshot[] = [
    {
      id: 'sys-c0001008-before',
      name: 'SYS_0xC0001008_PRE',
      addressHex: DAVIS_MMIO_SPECS.sysStatusHex,
      valueHex: formatHex(report.c0001008_before, HEX32_PAD_WIDTH),
      widthBits: 32,
      statusLabel: report.classification.toUpperCase(),
      severity: 'info',
    },
    {
      id: 'sys-c0001008-after',
      name: 'SYS_0xC0001008_POST',
      addressHex: DAVIS_MMIO_SPECS.sysStatusHex,
      valueHex: formatHex(report.c0001008_after, HEX32_PAD_WIDTH),
      widthBits: 32,
      statusLabel: report.fe_released ? 'FE_RELEASED' : 'FE_HALTED',
      severity: report.fe_released ? 'ok' : 'warning',
      flagBreakdown: `FUN_cd4d_rc=${formatHex(report.fun_cd4d_rc, HEX32_PAD_WIDTH)}`,
    },
  ];
  const chRows = buildTaskfileRows(channel.toUpperCase(), baseAddr, activeSnap);
  const slaveRows =
    channel === 'ch1'
      ? buildTaskfileRows('CH1_SLAVE', DAVIS_MMIO_SPECS.ch1DriveHeadHex, report.ch1_slave)
      : [];
  return [...sysRows, ...chRows, ...slaveRows];
}

export function isDavisEh55Compatible(report: DavisProbeReport | null): boolean {
  const info = report?.identify_info;
  if (!info || !report) return false;
  return !info.is_atapi && info.mwdma2_supported && report.classification === 'ata_ready';
}
