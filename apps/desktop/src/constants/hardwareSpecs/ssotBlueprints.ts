/**
 * @file constants/hardwareSpecs/ssotBlueprints.ts
 * @brief Data-driven physical pinout, break-in, flash geometry, and memory region blueprints.
 */

import ptBRJson from '@/locales/pt-BR.json';
import type { ColdBreakinMode } from '@/models';
import hardwareCatalog from '../../../../../crates/panadiga-core/src/ssot/hardware_catalog.json';
import { computeFlashGeometryAddresses } from './catalogTokens';

export type SsotLocaleDict = typeof ptBRJson.ssot;
export type PinLocaleKey = keyof SsotLocaleDict['pins'];
export type ModeLocaleKey = keyof SsotLocaleDict['breakinModes'];
export type FlashLocaleKey = keyof SsotLocaleDict['flashGeometries'];
export type RegionLocaleKey = keyof SsotLocaleDict['memoryRegions'];

export interface PinBlueprint {
  readonly key: PinLocaleKey;
  readonly manual_pin: number;
  readonly physical_pin: number;
  readonly pico_gpio: string;
}

export interface BreakInBlueprint {
  readonly key: ModeLocaleKey;
  readonly mode: ColdBreakinMode;
  readonly badge_variant: string;
}

export interface FlashGeometryBlueprint {
  readonly key: FlashLocaleKey;
  readonly size_mb: number;
  readonly size_bytes: number;
  readonly nvram_base_hex: string;
  readonly nvram_size_bytes: number;
  readonly microsector_range: string;
}

export interface MemoryRegionBlueprint {
  readonly key: RegionLocaleKey;
  readonly start: string;
  readonly end: string;
  readonly size_kb: number;
}

function assertLocaleKey<K extends string>(
  table: Readonly<Record<string, unknown>>,
  raw: string,
  section: string,
): K {
  if (!Object.prototype.hasOwnProperty.call(table, raw)) {
    throw new Error(`Missing locale entry ssot.${section}.${raw}`);
  }
  return raw as K;
}

function parseColdBreakinMode(raw: number): ColdBreakinMode {
  if (raw === 0 || raw === 1 || raw === 2 || raw === 3) {
    return raw;
  }
  throw new Error(`Invalid ColdBreakinMode ${raw} in hardware_catalog.json`);
}

export const PINOUT_BLUEPRINTS: readonly PinBlueprint[] = Object.freeze(
  hardwareCatalog.pinout.map(entry =>
    Object.freeze({
      key: assertLocaleKey<PinLocaleKey>(ptBRJson.ssot.pins, entry.key, 'pins'),
      manual_pin: entry.manual_pin,
      physical_pin: entry.physical_pin,
      pico_gpio: entry.pico_gpio,
    }),
  ),
);

export const BREAKIN_BLUEPRINTS: readonly BreakInBlueprint[] = Object.freeze(
  hardwareCatalog.breakin_modes.map(entry =>
    Object.freeze({
      key: assertLocaleKey<ModeLocaleKey>(ptBRJson.ssot.breakinModes, entry.key, 'breakinModes'),
      mode: parseColdBreakinMode(entry.mode),
      badge_variant: entry.badge_variant,
    }),
  ),
);

export const FLASH_GEOMETRY_BLUEPRINTS: readonly FlashGeometryBlueprint[] = Object.freeze(
  hardwareCatalog.flash_geometries.map(entry => {
    const geom = computeFlashGeometryAddresses(entry.size_mb);
    return Object.freeze({
      key: assertLocaleKey<FlashLocaleKey>(
        ptBRJson.ssot.flashGeometries,
        entry.key,
        'flashGeometries',
      ),
      size_mb: entry.size_mb,
      size_bytes: geom.sizeBytes,
      nvram_base_hex: geom.nvramBaseHex,
      nvram_size_bytes: geom.nvramSizeBytes,
      microsector_range: entry.microsector_range,
    });
  }),
);

export const MEMORY_REGION_BLUEPRINTS: readonly MemoryRegionBlueprint[] = Object.freeze([
  Object.freeze({ key: 'sram', start: '0x00000000', end: '0x0000FFFF', size_kb: 64 }),
  Object.freeze({ key: 'mmio', start: '0xC0000000', end: '0xC000FFFF', size_kb: 64 }),
  Object.freeze({ key: 'flash_ipl', start: '0x80000000', end: '0x800FFFFF', size_kb: 1024 }),
  Object.freeze({ key: 'rtos_fw', start: '0x80100000', end: '0x807EFFFF', size_kb: 7104 }),
  Object.freeze({ key: 'flash_nvram', start: '0x807F0000', end: '0x807FFFFF', size_kb: 64 }),
  Object.freeze({ key: 'sdram', start: '0x40000000', end: '0x43FFFFFF', size_kb: 65536 }),
]);
