/**
 * @file utils/hddCapacity.ts
 * @brief Pure RTSD preset lookup and LBA48 / 2KB block conversion helpers for HDD capacity patching.
 */

import {
  DECIMAL_GB_BYTES,
  HDD_DEFAULT_CAPACITY_GB,
  HDD_FACTORY_200GB_BLOCKS,
  HDD_MAX_2KB_BLOCKS_CAP,
  HDD_MAX_CAPACITY_GB,
  HDD_MIN_CAPACITY_GB,
  HDD_UNCLAMPED_LBA48_BLOCKS,
  MEIHDFS_BLOCK_SIZE_BYTES,
  OFFICIAL_RTSD_BLOCKS_BY_GB,
} from '@/constants';
import type { CapacityPassthroughMode, PatchMode } from '@/models';

export type { CapacityPassthroughMode, PatchMode };

export function previewBlocks2Kb(gb: number): number {
  if (OFFICIAL_RTSD_BLOCKS_BY_GB[gb]) {
    return OFFICIAL_RTSD_BLOCKS_BY_GB[gb];
  }
  const safeGb = Math.max(
    HDD_MIN_CAPACITY_GB,
    Math.min(HDD_MAX_CAPACITY_GB, Math.floor(gb || HDD_DEFAULT_CAPACITY_GB)),
  );
  return Math.min(
    HDD_MAX_2KB_BLOCKS_CAP,
    Math.floor((safeGb * DECIMAL_GB_BYTES) / MEIHDFS_BLOCK_SIZE_BYTES),
  );
}

export function computePreviewBlocks(
  mode: PatchMode,
  targetGb: number,
  limitInDynamicMode: boolean,
): number {
  if (mode === 'factory_200gb') return HDD_FACTORY_200GB_BLOCKS;
  if (mode === 'dynamic' && !limitInDynamicMode) return HDD_UNCLAMPED_LBA48_BLOCKS;
  return previewBlocks2Kb(targetGb);
}

export function resolveEffectiveTargetGb(
  mode: PatchMode,
  targetGb: number,
  limitInDynamicMode: boolean,
): number | null {
  if (mode === 'factory_200gb') return null;
  if (mode === 'cap' || limitInDynamicMode) {
    return Math.max(HDD_MIN_CAPACITY_GB, Math.floor(targetGb || HDD_DEFAULT_CAPACITY_GB));
  }
  return null;
}
