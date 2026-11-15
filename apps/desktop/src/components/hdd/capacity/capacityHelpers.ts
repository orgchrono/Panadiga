/**
 * @file components/hdd/capacity/capacityHelpers.ts
 * @brief Pure RTSD preset lookup and LBA48 / 2KB block conversion helpers for HDD capacity patching.
 */

export type PatchMode = 'dynamic' | 'cap' | 'factory_200gb';

export const QUICK_CAPACITIES_GB = [250, 300, 320, 400, 500, 1000, 2000] as const;

export const OFFICIAL_RTSD_BLOCKS_BY_GB: Record<number, number> = {
  400: 0x0ba43b74,
  300: 0x08baec97,
  250: 0x07466528,
  200: 0x05d21dba,
  160: 0x04a7d7c8,
  137: 0x03fc7ac3,
  120: 0x037dd1d6,
  100: 0x02e8cedd,
  80: 0x0253cbe4,
};

export function previewBlocks2Kb(gb: number): number {
  if (OFFICIAL_RTSD_BLOCKS_BY_GB[gb]) {
    return OFFICIAL_RTSD_BLOCKS_BY_GB[gb];
  }
  const safeGb = Math.max(20, Math.min(2200, Math.floor(gb || 250)));
  return Math.min(0x7fffffff, Math.floor((safeGb * 1_000_000_000) / 2048));
}

export function computePreviewBlocks(
  mode: PatchMode,
  targetGb: number,
  limitInDynamicMode: boolean
): number {
  if (mode === 'factory_200gb') return 0x05d21dba;
  if (mode === 'dynamic' && !limitInDynamicMode) return 0x3fffffff;
  return previewBlocks2Kb(targetGb);
}

export function resolveEffectiveTargetGb(
  mode: PatchMode,
  targetGb: number,
  limitInDynamicMode: boolean
): number | null {
  if (mode === 'factory_200gb') return null;
  if (mode === 'cap' || limitInDynamicMode) {
    return Math.max(20, Math.floor(targetGb || 250));
  }
  return null;
}
