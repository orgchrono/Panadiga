/**
 * @file constants/hddCapacity.ts
 * @brief Canonical RTSD preset block tables & LBA48 capacity bounds for Panasonic DIGA HDDs.
 */

export const QUICK_CAPACITIES_GB = [250, 300, 320, 400, 500, 1000, 2000] as const;
export const HDD_WRITER_QUICK_GBS = [250, 300, 400, 500, 1000, 2000] as const;
export const ONE_TERABYTE_GB = 1000;

export const OFFICIAL_RTSD_BLOCKS_BY_GB: Readonly<Record<number, number>> = Object.freeze({
  400: 0x0ba43b74,
  300: 0x08baec97,
  250: 0x07466528,
  200: 0x05d21dba,
  160: 0x04a7d7c8,
  137: 0x03fc7ac3,
  120: 0x037dd1d6,
  100: 0x02e8cedd,
  80: 0x0253cbe4,
});

export const HDD_MIN_CAPACITY_GB = 20;
export const HDD_MAX_CAPACITY_GB = 2200;
export const HDD_DEFAULT_CAPACITY_GB = 250;
export const HDD_FACTORY_200GB_GB = 200;
export const DECIMAL_GB_BYTES = 1_000_000_000;
export const HDD_FACTORY_200GB_BLOCKS = 0x05d21dba;
export const HDD_UNCLAMPED_LBA48_BLOCKS = 0x3fffffff;
export const HDD_MAX_2KB_BLOCKS_CAP = 0x7fffffff;
export const HDD_DEFAULT_250GB_SECTORS = 488_281_250;
