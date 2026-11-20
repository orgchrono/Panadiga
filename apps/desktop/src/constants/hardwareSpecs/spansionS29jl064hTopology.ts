/**
 * @file constants/hardwareSpecs/spansionS29jl064hTopology.ts
 * @brief Exact 142-sector Spansion S29JL064H70TFI00 (IC5001, 8 MiB) asymmetric sector topology & BGO FTL constants.
 */

export type SpansionSectorBandId =
  | 'boot_ipl_8k'
  | 'boot_ipl_64k'
  | 'upper_nor_code'
  | 'bgo_ftl'
  | 'upper_nor_app'
  | 'nvram_eemul';

export type SpansionWritePolicy =
  | 'E05_READ_ONLY'
  | 'UPPER_NOR_GUARDED'
  | 'BGO_UNLOCK_GUARDED'
  | 'NVRAM_AUTHORIZED';

export interface SpansionSectorBandSpec {
  readonly id: SpansionSectorBandId;
  readonly startSa: number;
  readonly count: number;
  readonly baseOffset: number;
  readonly sectorBytes: number;
  readonly writePolicy: SpansionWritePolicy;
}

export interface SpansionSectorDescriptor {
  readonly sa: number;
  readonly name: string;
  readonly startOffset: number;
  readonly endOffset: number;
  readonly busVa: number;
  readonly uncachedVa: number;
  readonly sizeBytes: number;
  readonly band: SpansionSectorBandId;
  readonly writePolicy: SpansionWritePolicy;
}

export const SPANSION_S29JL064H_SECTOR_COUNT = 142;
export const SPANSION_8KB_SECTOR_BYTES = 8 * 1024;
export const SPANSION_64KB_SECTOR_BYTES = 64 * 1024;
export const SPANSION_S29JL064H_SIZE_BYTES = 8 * 1024 * 1024;
export const AM34_FLASH_BUS_BASE = 0x8000_0000;
export const AM34_FLASH_UNCACHED_BASE = 0x8400_0000;

export const SPANSION_IPL_LAST_SA = 10;
export const SPANSION_IPL_END_OFFSET = 0x0004_0000;
export const SPANSION_BGO_FIRST_SA = 26;
export const SPANSION_BGO_LAST_SA = 70;
export const SPANSION_BGO_SECTOR_COUNT = 45;
export const SPANSION_BGO_BASE_OFFSET = 0x0013_0000;
export const SPANSION_BGO_BUS_VA = 0x8413_0000;
export const SPANSION_BGO_SIZE_BYTES = 0x002d_0000;
export const SPANSION_BGO_EXPECTED_P2_COUNT = 40;
export const SPANSION_BGO_EXPECTED_P2_SUM = 820;
export const SPANSION_BGO_L2P_SA_PRIMARY = 29;
export const SPANSION_BGO_L2P_SA_SECONDARY = 41;
export const SPANSION_BGO_FAT_FIRST_PID = 0x0f01;
export const SPANSION_BGO_FAT_LAST_PID = 0x0f24;
export const SPANSION_NVRAM_FIRST_SA = 134;
export const SPANSION_NVRAM_LAST_SA = 141;

export const SPANSION_142_SECTOR_BANDS: readonly SpansionSectorBandSpec[] = Object.freeze([
  {
    id: 'boot_ipl_8k',
    startSa: 0,
    count: 8,
    baseOffset: 0x0000_0000,
    sectorBytes: SPANSION_8KB_SECTOR_BYTES,
    writePolicy: 'E05_READ_ONLY',
  },
  {
    id: 'boot_ipl_64k',
    startSa: 8,
    count: 3,
    baseOffset: 0x0001_0000,
    sectorBytes: SPANSION_64KB_SECTOR_BYTES,
    writePolicy: 'E05_READ_ONLY',
  },
  {
    id: 'upper_nor_code',
    startSa: 11,
    count: 15,
    baseOffset: 0x0004_0000,
    sectorBytes: SPANSION_64KB_SECTOR_BYTES,
    writePolicy: 'UPPER_NOR_GUARDED',
  },
  {
    id: 'bgo_ftl',
    startSa: 26,
    count: 45,
    baseOffset: 0x0013_0000,
    sectorBytes: SPANSION_64KB_SECTOR_BYTES,
    writePolicy: 'BGO_UNLOCK_GUARDED',
  },
  {
    id: 'upper_nor_app',
    startSa: 71,
    count: 63,
    baseOffset: 0x0040_0000,
    sectorBytes: SPANSION_64KB_SECTOR_BYTES,
    writePolicy: 'UPPER_NOR_GUARDED',
  },
  {
    id: 'nvram_eemul',
    startSa: 134,
    count: 8,
    baseOffset: 0x007f_0000,
    sectorBytes: SPANSION_8KB_SECTOR_BYTES,
    writePolicy: 'NVRAM_AUTHORIZED',
  },
]);

export function buildSpansion142SectorTopology(): readonly SpansionSectorDescriptor[] {
  return Object.freeze(
    SPANSION_142_SECTOR_BANDS.flatMap((band) =>
      Array.from({ length: band.count }, (_, idx) => {
        const sa = band.startSa + idx;
        const startOffset = band.baseOffset + idx * band.sectorBytes;
        return Object.freeze({
          sa,
          name: `SA${sa}`,
          startOffset,
          endOffset: startOffset + band.sectorBytes - 1,
          busVa: AM34_FLASH_BUS_BASE + startOffset,
          uncachedVa: AM34_FLASH_UNCACHED_BASE + startOffset,
          sizeBytes: band.sectorBytes,
          band: band.id,
          writePolicy: band.writePolicy,
        });
      }),
    ),
  );
}

export const SPANSION_142_SECTOR_TOPOLOGY: readonly SpansionSectorDescriptor[] =
  buildSpansion142SectorTopology();
