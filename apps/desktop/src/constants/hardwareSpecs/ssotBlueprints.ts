/**
 * @file constants/hardwareSpecs/ssotBlueprints.ts
 * @brief Frozen physical pinout, break-in, flash geometry, and memory region blueprints.
 */

import type ptBRJson from '@/locales/pt-BR.json';
import type { ColdBreakinMode } from '@/models';
import {
  SPANSION_FLASH_SIZE_BYTES,
  SPANSION_NVRAM_SIZE_BYTES,
} from './physicalConstants';

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

export const PINOUT_BLUEPRINTS: readonly PinBlueprint[] = Object.freeze([
  Object.freeze({ key: 'pin1', manual_pin: 1, physical_pin: 1, pico_gpio: 'GP26 (ADC0 / Pin 31)' }),
  Object.freeze({ key: 'pin2', manual_pin: 2, physical_pin: 2, pico_gpio: 'GP5 (Pin 7)' }),
  Object.freeze({ key: 'pin3', manual_pin: 3, physical_pin: 3, pico_gpio: 'GP4 (Pin 6)' }),
  Object.freeze({ key: 'pin4', manual_pin: 4, physical_pin: 4, pico_gpio: 'GND (Pin 38)' }),
  Object.freeze({ key: 'pin5', manual_pin: 5, physical_pin: 5, pico_gpio: 'GP3 (Pin 5)' }),
  Object.freeze({ key: 'pin6', manual_pin: 6, physical_pin: 6, pico_gpio: 'GP2 (Pin 4)' }),
]);

export interface BreakInBlueprint {
  readonly key: ModeLocaleKey;
  readonly mode: ColdBreakinMode;
  readonly badge_variant: string;
}

export const BREAKIN_BLUEPRINTS: readonly BreakInBlueprint[] = Object.freeze([
  Object.freeze({ key: 'mode0', mode: 0, badge_variant: 'secondary' }),
  Object.freeze({ key: 'mode1', mode: 1, badge_variant: 'secondary' }),
  Object.freeze({ key: 'mode2', mode: 2, badge_variant: 'success' }),
  Object.freeze({ key: 'mode3', mode: 3, badge_variant: 'warning' }),
]);

export interface FlashGeometryBlueprint {
  readonly key: FlashLocaleKey;
  readonly size_mb: number;
  readonly size_bytes: number;
  readonly nvram_base_hex: string;
  readonly nvram_size_bytes: number;
  readonly microsector_range: string;
}

export const FLASH_GEOMETRY_BLUEPRINTS: readonly FlashGeometryBlueprint[] = Object.freeze([
  Object.freeze({
    key: 'mb2',
    size_mb: 2,
    size_bytes: 0x0020_0000,
    nvram_base_hex: '0x801F0000',
    nvram_size_bytes: SPANSION_NVRAM_SIZE_BYTES,
    microsector_range: 'SA38..SA45',
  }),
  Object.freeze({
    key: 'mb4',
    size_mb: 4,
    size_bytes: 0x0040_0000,
    nvram_base_hex: '0x803F0000',
    nvram_size_bytes: SPANSION_NVRAM_SIZE_BYTES,
    microsector_range: 'SA70..SA77',
  }),
  Object.freeze({
    key: 'mb8',
    size_mb: 8,
    size_bytes: SPANSION_FLASH_SIZE_BYTES,
    nvram_base_hex: '0x807F0000',
    nvram_size_bytes: SPANSION_NVRAM_SIZE_BYTES,
    microsector_range: 'SA134..SA141',
  }),
  Object.freeze({
    key: 'mb16',
    size_mb: 16,
    size_bytes: 0x0100_0000,
    nvram_base_hex: '0x80FE0000',
    nvram_size_bytes: 0x0002_0000,
    microsector_range: 'SA254..SA269',
  }),
]);

export interface MemoryRegionBlueprint {
  readonly key: RegionLocaleKey;
  readonly start: string;
  readonly end: string;
  readonly size_kb: number;
}

export const MEMORY_REGION_BLUEPRINTS: readonly MemoryRegionBlueprint[] = Object.freeze([
  Object.freeze({ key: 'sram', start: '0x00000000', end: '0x0000FFFF', size_kb: 64 }),
  Object.freeze({ key: 'mmio', start: '0xC0000000', end: '0xC000FFFF', size_kb: 64 }),
  Object.freeze({ key: 'flash_ipl', start: '0x80000000', end: '0x800FFFFF', size_kb: 1024 }),
  Object.freeze({ key: 'rtos_fw', start: '0x80100000', end: '0x807EFFFF', size_kb: 7104 }),
  Object.freeze({ key: 'flash_nvram', start: '0x807F0000', end: '0x807FFFFF', size_kb: 64 }),
  Object.freeze({ key: 'sdram', start: '0x40000000', end: '0x43FFFFFF', size_kb: 65536 }),
]);
