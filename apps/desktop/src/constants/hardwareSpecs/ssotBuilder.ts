/**
 * @file constants/hardwareSpecs/ssotBuilder.ts
 * @brief Functional SSOT Spec Builder composing frozen hardware blueprints with i18next localization tables.
 */

import enUSJson from '@/locales/en-US/ssot.json';
import ptBRJson from '@/locales/pt-BR/ssot.json';
import type {
  BreakInModeSpec,
  FlashGeometrySpec,
  HardwareSsotSpec,
  MemoryRegionSpec,
  P7501PinMapping,
  PicoCommandArgOption,
  PicoCommandSpec,
} from '@/models';
import {
  HARDWARE_THRESHOLDS_SPEC,
  INDEXED_SERVICE_MANUALS_COUNT,
  PICO_USB_IDENTITY_SPEC,
} from './physicalConstants';
import { PICO_COMMAND_BLUEPRINTS, type PicoCommandLocaleEntry } from './picoCommandBlueprints';
import {
  BREAKIN_BLUEPRINTS,
  FLASH_GEOMETRY_BLUEPRINTS,
  MEMORY_REGION_BLUEPRINTS,
  PINOUT_BLUEPRINTS,
} from './ssotBlueprints';

const SSOT_LOCALE_TABLES = Object.freeze({
  'pt-BR': ptBRJson.ssot,
  'en-US': enUSJson.ssot,
});

export type SupportedLocale = keyof typeof SSOT_LOCALE_TABLES;

function lookupSectionEntry<T>(
  section: Readonly<Record<string, T>>,
  key: string,
  sectionName: string,
): T {
  const matched = Object.entries(section).find(([entryKey]) => entryKey === key);
  if (!matched) {
    throw new Error(`Missing locale entry ssot.${sectionName}.${key}`);
  }
  return matched[1];
}

/**
 * Pure builder composing frozen physical hardware constants with i18next localization tables.
 */
export function buildCanonicalHardwareSsotSpec(locale: SupportedLocale = 'pt-BR'): HardwareSsotSpec {
  const dict = SSOT_LOCALE_TABLES[locale] ?? SSOT_LOCALE_TABLES['pt-BR'];
  const commandDict: Readonly<Record<string, PicoCommandLocaleEntry>> = dict.commands;

  const pinout: P7501PinMapping[] = PINOUT_BLUEPRINTS.map(bp => {
    const text = lookupSectionEntry(dict.pins, bp.key, 'pins');
    const criticalNote = 'critical_note' in text && typeof text.critical_note === 'string'
      ? text.critical_note
      : '';
    return {
      manual_pin: bp.manual_pin,
      physical_pin: bp.physical_pin,
      pin_label: text.pin_label,
      pico_gpio: bp.pico_gpio,
      pico_pin: text.pico_pin,
      signal_name: text.signal_name,
      direction: text.direction,
      voltage: text.voltage,
      critical_note: criticalNote || null,
    };
  });

  const breakin_modes: BreakInModeSpec[] = BREAKIN_BLUEPRINTS.map(bp => {
    const text = lookupSectionEntry(dict.breakinModes, bp.key, 'breakinModes');
    return {
      mode: bp.mode,
      label: text.label,
      badge: text.badge,
      badge_variant: bp.badge_variant,
      description: text.description,
      signal_diagram: text.signal_diagram,
    };
  });

  const flash_geometries: FlashGeometrySpec[] = FLASH_GEOMETRY_BLUEPRINTS.map(bp => {
    const text = lookupSectionEntry(dict.flashGeometries, bp.key, 'flashGeometries');
    return {
      size_mb: bp.size_mb,
      size_bytes: bp.size_bytes,
      chip_name: text.chip_name,
      nvram_base_hex: bp.nvram_base_hex,
      nvram_size_bytes: bp.nvram_size_bytes,
      microsector_range: bp.microsector_range,
    };
  });

  const memory_regions: MemoryRegionSpec[] = MEMORY_REGION_BLUEPRINTS.map(bp => {
    const text = lookupSectionEntry(dict.memoryRegions, bp.key, 'memoryRegions');
    return {
      name: text.name,
      start: bp.start,
      end: bp.end,
      size_kb: bp.size_kb,
      perms: text.perms,
    };
  });

  const pico_commands: PicoCommandSpec[] = PICO_COMMAND_BLUEPRINTS.map(bp => {
    const text = lookupSectionEntry(commandDict, bp.id, 'commands');
    const options: PicoCommandArgOption[] = (bp.optionPairs ?? []).map(([optKey, value]) => ({
      label: text.options?.[optKey] ?? value,
      value,
    }));
    return {
      id: bp.id,
      category: bp.category,
      label: text.label,
      description: text.description,
      syntax: bp.syntax,
      arg_label: text.arg_label ?? null,
      arg_placeholder: bp.default_arg ?? text.arg_placeholder ?? null,
      default_arg: bp.default_arg,
      arg_options: options,
      dangerous: Boolean(bp.dangerous),
      timeout_ms: bp.timeout_ms,
    };
  });

  return Object.freeze({
    thresholds: HARDWARE_THRESHOLDS_SPEC,
    usb_ids: PICO_USB_IDENTITY_SPEC,
    indexed_manuals_count: INDEXED_SERVICE_MANUALS_COUNT,
    pinout,
    breakin_modes,
    flash_geometries,
    memory_regions,
    pico_commands,
  });
}

export const CANONICAL_HARDWARE_SSOT_SPEC: HardwareSsotSpec = buildCanonicalHardwareSsotSpec('pt-BR');
