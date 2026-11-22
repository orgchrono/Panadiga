/**
 * @file constants/hardwareSpecs/catalogTokens.ts
 * @brief Typed hardware/RE address tokens & template substitution for the shared SSOT catalog.
 *
 * Mirrors `crates/panadiga-core/src/ssot/spec/constants.rs` and `hardware_am34.rs` so that
 * `hardware_catalog.json` never embeds raw magic addresses, baud rates, or passcode literals.
 */

import { SSOT_AM34_BUS_CONSTANTS } from '@/models/generated/ssot';

import {
  AM34_FLASH_BASE_ADDR,
  AM34_NVRAM_BASE_ADDR,
  AM34_RAM_LOADER_ENTRY_ADDR,
  PICO_DEFAULT_BAUD_RATE,
  SPANSION_FLASH_SIZE_BYTES,
  SPANSION_NVRAM_SIZE_BYTES,
} from './physicalConstants';

export const AM34_FLASH_UNCACHED_BASE_ADDR = SSOT_AM34_BUS_CONSTANTS.flashUncachedBaseAddr;
export const AM34_NVRAM_UNCACHED_BASE_ADDR = SSOT_AM34_BUS_CONSTANTS.nvramUncachedBaseAddr;
export const FLASH_16MB_NVRAM_SIZE_BYTES = 128 * 1024;
export const FLASH_16MB_THRESHOLD_MB = 16;
export const MIB_BYTES = 1024 * 1024;

export const DEV_SERIAL_STATE_BASE_ADDR = 0x515b2e00;
export const DEV_SERIAL_MEM_PROBE_OFFSET = 0x38;

export const DRV_PASS_HEX_TOKEN_0 = 0x4d415453; // "MATS"
export const DRV_PASS_HEX_TOKEN_1 = 0x48495441; // "HITA"

export const P7501_WDT_MAX_CMD_WINDOW_MS = SSOT_AM34_BUS_CONSTANTS.wdtMaxCmdWindowMs;
export const WDT_DUMPNOR_CAPTURE_WINDOW_MS = 180000;
export const NOR_DUMPER_DEFAULT_BLOCK_SIZE = 4096;
export const AM34_FLASH_BASE_HEX = SSOT_AM34_BUS_CONSTANTS.flashCachedBaseHex;
export const AM34_NVRAM_BASE_HEX = SSOT_AM34_BUS_CONSTANTS.nvramCachedBaseHex;
export const AM34_FLASH_UNCACHED_ADDR_HEX = SSOT_AM34_BUS_CONSTANTS.flashUncachedBaseHex;
export const AM34_FLASH_SIZE_HEX = '0x800000';
export const AM34_SDRAM_STUB_WRAP_ADDR = SSOT_AM34_BUS_CONSTANTS.digadiagStage15LoadAddrHex;
export const DIGADIAG_STAGE15_LOAD_ADDR_HEX = SSOT_AM34_BUS_CONSTANTS.digadiagStage15LoadAddrHex;
export const AM34_SDRAM_STUB_RAW_ADDR = SSOT_AM34_BUS_CONSTANTS.sdramStubRawAddrHex;

export function formatHexU32(value: number, upperCase = true): string {
  const padded = (value >>> 0).toString(16).padStart(8, '0');
  return upperCase ? padded.toUpperCase() : padded;
}

const CATALOG_TEMPLATE_TOKENS: ReadonlyArray<readonly [string, string]> = Object.freeze([
  ['{{AM34_NVRAM_BASE_HEX}}', `0x${formatHexU32(AM34_NVRAM_BASE_ADDR)}`],
  ['{{AM34_FLASH_BASE_HEX}}', `0x${formatHexU32(AM34_FLASH_BASE_ADDR)}`],
  ['{{AM34_SRAM_LOADER_BASE_HEX}}', `0x${formatHexU32(AM34_RAM_LOADER_ENTRY_ADDR)}`],
  ['{{AM34_FLASH_UNCACHED_ADDR}}', formatHexU32(AM34_FLASH_UNCACHED_BASE_ADDR)],
  ['{{AM34_FLASH_SIZE_HEX_NO_PREFIX}}', SPANSION_FLASH_SIZE_BYTES.toString(16).toUpperCase()],
  ['{{AM34_NVRAM_UNCACHED_ADDR}}', formatHexU32(AM34_NVRAM_UNCACHED_BASE_ADDR)],
  [
    '{{DEV_SERIAL_MEM_PROBE_ADDR}}',
    formatHexU32(DEV_SERIAL_STATE_BASE_ADDR + DEV_SERIAL_MEM_PROBE_OFFSET),
  ],
  ['{{DEFAULT_BAUD_RATE}}', String(PICO_DEFAULT_BAUD_RATE)],
  [
    '{{MATSHITA_PASS_UNLOCK_CMD}}',
    `PASS 0x${formatHexU32(DRV_PASS_HEX_TOKEN_0, false)} 0x${formatHexU32(DRV_PASS_HEX_TOKEN_1, false)}`,
  ],
  ['{{WDT_DUMPNOR_CAPTURE_WINDOW_MS}}', String(WDT_DUMPNOR_CAPTURE_WINDOW_MS)],
]);

/** Resolves all canonical `{{TOKEN}}` placeholders in a catalog template string. */
export function resolveCatalogTemplate(raw: string): string {
  if (!raw.includes('{{')) {
    return raw;
  }
  return CATALOG_TEMPLATE_TOKENS.reduce(
    (resolved, [needle, replacement]) =>
      resolved.includes(needle) ? resolved.split(needle).join(replacement) : resolved,
    raw,
  );
}

/** Derives byte size, top-boot NVRAM base address, and NVRAM sector size from MB aperture. */
export function computeFlashGeometryAddresses(sizeMb: number): {
  sizeBytes: number;
  nvramBaseHex: string;
  nvramSizeBytes: number;
} {
  const sizeBytes = sizeMb * MIB_BYTES;
  const nvramSizeBytes =
    sizeMb >= FLASH_16MB_THRESHOLD_MB ? FLASH_16MB_NVRAM_SIZE_BYTES : SPANSION_NVRAM_SIZE_BYTES;
  const nvramBaseAddr = AM34_FLASH_BASE_ADDR + sizeBytes - nvramSizeBytes;
  return {
    sizeBytes,
    nvramBaseHex: `0x${formatHexU32(nvramBaseAddr)}`,
    nvramSizeBytes,
  };
}
