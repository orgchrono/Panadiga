/**
 * @file constants/hardwareSpecs/ssotBlueprints.ts
 * @brief Data-driven physical pinout, break-in, flash geometry, and memory region blueprints.
 */

import ptBRJson from '@/locales/pt-BR/ssot.json';
import type { ColdBreakinMode } from '@/models';
import hardwareCatalog from '../../../../../crates/panadiga-core/src/ssot/hardware_catalog.json';
import { computeFlashGeometryAddresses } from './catalogTokens';

export type SsotLocaleDict = typeof ptBRJson.ssot;

export interface PinBlueprint {
  readonly key: string;
  readonly manual_pin: number;
  readonly physical_pin: number;
  readonly pico_gpio: string;
}

export interface BreakInBlueprint {
  readonly key: string;
  readonly mode: ColdBreakinMode;
  readonly badge_variant: string;
}

export interface FlashGeometryBlueprint {
  readonly key: string;
  readonly size_mb: number;
  readonly size_bytes: number;
  readonly nvram_base_hex: string;
  readonly nvram_size_bytes: number;
  readonly microsector_range: string;
}

export interface MemoryRegionBlueprint {
  readonly key: string;
  readonly start: string;
  readonly end: string;
  readonly size_kb: number;
}

function assertCatalogKeyExists(
  table: Readonly<Record<string, unknown>>,
  raw: string,
  section: string,
): string {
  if (!Object.prototype.hasOwnProperty.call(table, raw)) {
    throw new Error(`Missing locale entry ssot.${section}.${raw}`);
  }
  return raw;
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
      key: assertCatalogKeyExists(ptBRJson.ssot.pins, entry.key, 'pins'),
      manual_pin: entry.manual_pin,
      physical_pin: entry.physical_pin,
      pico_gpio: entry.pico_gpio,
    }),
  ),
);

export const BREAKIN_BLUEPRINTS: readonly BreakInBlueprint[] = Object.freeze(
  hardwareCatalog.breakin_modes.map(entry =>
    Object.freeze({
      key: assertCatalogKeyExists(ptBRJson.ssot.breakinModes, entry.key, 'breakinModes'),
      mode: parseColdBreakinMode(entry.mode),
      badge_variant: entry.badge_variant,
    }),
  ),
);

export const FLASH_GEOMETRY_BLUEPRINTS: readonly FlashGeometryBlueprint[] = Object.freeze(
  hardwareCatalog.flash_geometries.map(entry => {
    const geom = computeFlashGeometryAddresses(entry.size_mb);
    return Object.freeze({
      key: assertCatalogKeyExists(ptBRJson.ssot.flashGeometries, entry.key, 'flashGeometries'),
      size_mb: entry.size_mb,
      size_bytes: geom.sizeBytes,
      nvram_base_hex: geom.nvramBaseHex,
      nvram_size_bytes: geom.nvramSizeBytes,
      microsector_range: entry.microsector_range,
    });
  }),
);

export const MEMORY_REGION_BLUEPRINTS: readonly MemoryRegionBlueprint[] = Object.freeze(
  hardwareCatalog.memory_region_blueprints.map(entry =>
    Object.freeze({
      key: entry.key,
      start: entry.start,
      end: entry.end,
      size_kb: entry.size_kb,
    }),
  ),
);
