/**
 * @file services/diagnosticParser/lineParsers.ts
 * @brief Low-level pure line and word parsers for P7501 PINCHECK, transition counters, VDD, and hex dump payloads.
 */

import { FLASH_CANDIDATE_SIZES_MB } from '@/constants';
import type { BusTransitionRow, DumpWordRow, PincheckRow } from '@/models/pico';

const MIN_PINCHECK_COLUMNS = 5;
const PIN_COL_STATE_INDEX = 4;
const PIN_COL_PULL_UP_INDEX = 3;
const PIN_COL_PULL_DOWN_INDEX = 2;
const HEX_RADIX = 16;
const BYTE_MASK = 0xff;
const SHIFT_BYTE_1 = 8;
const SHIFT_BYTE_2 = 16;
const SHIFT_BYTE_3 = 24;
const FULL_DUMP_ADDR_DIGITS = 8;
const COMPACT_DUMP_ADDR_DIGITS = 4;
const ASCII_PRINTABLE_MIN = 0x20;
const ASCII_PRINTABLE_MAX = 0x7e;
const VDD_MIN_VALID = 0.01;

export function parsePincheckRows(rawOutput: string): { rows: PincheckRow[]; riseTime: string | null } {
  const lines = rawOutput.split(/\r?\n/).map(line => line.trim());

  const rows: PincheckRow[] = lines.flatMap(trimmed => {
    if (
      !trimmed.includes('|') ||
      (!trimmed.startsWith('GP') && !trimmed.includes('Pin')) ||
      trimmed.startsWith('Pin ') ||
      trimmed.includes('---+')
    ) {
      return [];
    }
    const parts = trimmed.split('|').map(s => s.trim());
    if (parts.length < MIN_PINCHECK_COLUMNS) {
      return [];
    }
    return [
      {
        pin_label: parts[0],
        hiz: parts[1],
        pull_down: parts[PIN_COL_PULL_DOWN_INDEX],
        pull_up: parts[PIN_COL_PULL_UP_INDEX],
        state: parts[PIN_COL_STATE_INDEX],
      },
    ];
  });

  const riseLine = lines.find(trimmed => trimmed.includes('XRESET Release Rise Time'));
  const colonIdx = riseLine ? riseLine.indexOf(':') : -1;
  const riseTime = riseLine && colonIdx >= 0 ? riseLine.slice(colonIdx + 1).trim() : null;

  return { rows, riseTime };
}

export function parseTransitionRows(rawOutput: string): BusTransitionRow[] {
  return rawOutput
    .split(/\r?\n/)
    .map(line => line.trim())
    .flatMap(trimmed => {
      if (!trimmed.includes('Final=') || !trimmed.includes('Transitions=') || !trimmed.includes(':')) {
        return [];
      }
      const colonIdx = trimmed.indexOf(':');
      const left = trimmed.slice(0, colonIdx).trim();
      const right = trimmed.slice(colonIdx + 1);
      const finalMatch = right.match(/Final=(\d+)/);
      const transMatch = right.match(/Transitions=(\d+)/);
      const leftParts = left.split(/\s+/);
      return [
        {
          signal: leftParts[0] || 'SIG',
          gpio: (leftParts[1] || '').replace(/[()]/g, ''),
          final_level: finalMatch ? Number.parseInt(finalMatch[1], 10) : null,
          transitions: transMatch ? Number.parseInt(transMatch[1], 10) : 0,
        },
      ];
    });
}

interface ParsedDumpLine {
  readonly addrHex: string;
  readonly parsedAddr: number;
  readonly words: string[];
  readonly bytes: number[];
}

function parseDumpLine(rawLine: string, minAddrHexDigits: number): ParsedDumpLine | null {
  const trimmed = rawLine.trim();
  const colonIdx = trimmed.indexOf(':');
  if (colonIdx <= 0) return null;

  const addrRaw = trimmed.slice(0, colonIdx).trim().replace(/^0x/i, '');
  const addrRegex = new RegExp(`^[0-9a-fA-F]{${minAddrHexDigits},8}$`);
  if (!addrRegex.test(addrRaw)) return null;

  const parsedAddr = Number.parseInt(addrRaw, HEX_RADIX);
  if (Number.isNaN(parsedAddr)) return null;

  const valsPart = trimmed.slice(colonIdx + 1).trim();
  const decodedTokens = valsPart
    .split(/\s+/)
    .map(tok => tok.replace(/^0x/i, ''))
    .filter(cleanTok => /^[0-9a-fA-F]{8}$/.test(cleanTok))
    .map(cleanTok => {
      const value = Number.parseInt(cleanTok, HEX_RADIX) >>> 0;
      return {
        word: `0x${cleanTok.toUpperCase()}`,
        leBytes: [
          value & BYTE_MASK,
          (value >>> SHIFT_BYTE_1) & BYTE_MASK,
          (value >>> SHIFT_BYTE_2) & BYTE_MASK,
          (value >>> SHIFT_BYTE_3) & BYTE_MASK,
        ],
      };
    });

  return {
    addrHex: addrRaw.toUpperCase(),
    parsedAddr,
    words: decodedTokens.map(token => token.word),
    bytes: decodedTokens.flatMap(token => token.leBytes),
  };
}

export function parseDumpRows(rawOutput: string): DumpWordRow[] {
  return rawOutput
    .split(/\r?\n/)
    .flatMap(line => {
      const parsed = parseDumpLine(line, FULL_DUMP_ADDR_DIGITS);
      if (!parsed || parsed.words.length === 0) return [];
      const ascii = parsed.bytes
        .map(b =>
          b >= ASCII_PRINTABLE_MIN && b <= ASCII_PRINTABLE_MAX ? String.fromCharCode(b) : '.',
        )
        .join('');
      return [
        {
          address: `0x${parsed.addrHex}`,
          words: parsed.words,
          ascii,
        },
      ];
    });
}

export function parseVddVolts(rawOutput: string): number | null {
  const match = rawOutput.match(/VDD[=:\s]+(\d+(?:\.\d+)?)\s*V/i);
  if (!match) return null;
  const v = Number.parseFloat(match[1]);
  return Number.isFinite(v) && v > VDD_MIN_VALID ? v : null;
}

export function parseCapturedBytes(rawOutput: string): number | null {
  const match = rawOutput.match(/(\d+)\s+bytes/i);
  return match ? Number.parseInt(match[1], 10) : null;
}

export function parseFlashMb(rawOutput: string): number | null {
  return FLASH_CANDIDATE_SIZES_MB.find(mb => rawOutput.includes(`${mb} MB`)) ?? null;
}

/**
 * Decodes `ADDR: w0 w1 w2 w3` lines returned by the Pico `DUMP` command into a Little-Endian byte array.
 */
export function parseP7501DumpBytes(rawOutput: string): { baseAddress: number | null; bytes: number[] } {
  const parsedLines = rawOutput
    .split(/\r?\n/)
    .map(rawLine => parseDumpLine(rawLine, COMPACT_DUMP_ADDR_DIGITS))
    .filter((item): item is ParsedDumpLine => item !== null);

  return {
    baseAddress: parsedLines[0]?.parsedAddr ?? null,
    bytes: parsedLines.flatMap(item => item.bytes),
  };
}
