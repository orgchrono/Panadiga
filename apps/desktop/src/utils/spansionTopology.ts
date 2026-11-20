/**
 * @file utils/spansionTopology.ts
 * @brief Pure 142-sector Spansion S29JL064H (SA0..SA141) FlashSectorInfo mapper shared by NVRAM and BGO FTL views.
 */

import {
  HEX32_PAD_WIDTH,
  SPANSION_142_SECTOR_TOPOLOGY,
  SPANSION_BGO_FIRST_SA,
  SPANSION_BGO_LAST_SA,
  SPANSION_IPL_LAST_SA,
  SPANSION_NVRAM_FIRST_SA,
  type SpansionSectorDescriptor,
} from '@/constants';
import type { FlashSectorInfo, SectorType } from '@/models';

export interface SectorMapEntryContext {
  readonly sectorType: SectorType;
  readonly isCorrupted: boolean;
  readonly activeBytes: number;
  readonly status: FlashSectorInfo['status'];
  readonly note: string;
}

export function resolveSpansionSectorType(sa: number): SectorType {
  if (sa <= SPANSION_IPL_LAST_SA) return 'bootloader';
  if (sa >= SPANSION_BGO_FIRST_SA && sa <= SPANSION_BGO_LAST_SA) return 'bgo_region';
  if (sa >= SPANSION_NVRAM_FIRST_SA) return 'nvram_microsector';
  return 'os_firmware';
}

export function formatSpansionHexAddress(offset: number): string {
  return `0x${offset.toString(16).toUpperCase().padStart(HEX32_PAD_WIDTH, '0')}`;
}

export function mapSpansion142Sectors(
  resolveEntry: (desc: SpansionSectorDescriptor, sectorType: SectorType) => SectorMapEntryContext,
): FlashSectorInfo[] {
  return SPANSION_142_SECTOR_TOPOLOGY.map((desc) => {
    const baseType = resolveSpansionSectorType(desc.sa);
    const ctx = resolveEntry(desc, baseType);
    return {
      index: desc.sa,
      name: desc.name,
      start_offset: desc.startOffset,
      end_offset: desc.endOffset,
      size_bytes: desc.sizeBytes,
      sector_type: ctx.sectorType,
      write_policy: desc.writePolicy,
      is_corrupted_area: ctx.isCorrupted,
      status: ctx.status,
      active_bytes: ctx.activeBytes,
      notes: ctx.note,
    };
  });
}
