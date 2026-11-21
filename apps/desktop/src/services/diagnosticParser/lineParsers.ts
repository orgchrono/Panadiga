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
const FULL_DUMP_ADDR_REGEX = /^[0-9a-fA-F]{8}$/;
const COMPACT_DUMP_ADDR_REGEX = /^[0-9a-fA-F]{4,8}$/;
const HEX_WORD_REGEX = /^[0-9a-fA-F]{8}$/;
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

function parseDumpLine(rawLine: string, addrRegex: RegExp): ParsedDumpLine | null {
  const trimmed = rawLine.trim();
  const colonIdx = trimmed.indexOf(':');
  if (colonIdx <= 0) return null;

  const addrRaw = trimmed.slice(0, colonIdx).trim().replace(/^0x/i, '');
  if (!addrRegex.test(addrRaw)) return null;

  const parsedAddr = Number.parseInt(addrRaw, HEX_RADIX);
  if (Number.isNaN(parsedAddr)) return null;

  const valsPart = trimmed.slice(colonIdx + 1).trim();
  const decodedTokens = valsPart
    .split(/\s+/)
    .map(tok => tok.replace(/^0x/i, ''))
    .filter(cleanTok => HEX_WORD_REGEX.test(cleanTok))
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
      const parsed = parseDumpLine(line, FULL_DUMP_ADDR_REGEX);
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
    .map(rawLine => parseDumpLine(rawLine, COMPACT_DUMP_ADDR_REGEX))
    .filter((item): item is ParsedDumpLine => item !== null);

  return {
    baseAddress: parsedLines[0]?.parsedAddr ?? null,
    bytes: parsedLines.flatMap(item => item.bytes),
  };
}

const FLASH_MB_16 = 16;
const FLASH_MB_8 = 8;
const FLASH_MB_4 = 4;
const FLASH_MB_2 = 2;
const FLASH_MB_1 = 1;
const DECIMAL_RADIX = 10;
const MILLIVOLTS_PER_VOLT = 1000;
const WIRE_GROUP_KEY = 1;
const WIRE_GROUP_RAW = 2;
const WIRE_GROUP_QUOTED = 3;

const FLASH_CHIP_SIZE_RULES: ReadonlyArray<readonly [ReadonlyArray<string>, number]> = Object.freeze([
  [['128N', '16MB'], FLASH_MB_16],
  [['064H', '8MB'], FLASH_MB_8],
  [['032', '320', '4MB'], FLASH_MB_4],
  [['016', '2MB'], FLASH_MB_2],
  [['27V802', '1MB'], FLASH_MB_1],
]);

export interface ParsedWireProtocolSummary {
  readonly resStatus: string | null;
  readonly resCode: string | null;
  readonly resElapsedUs: number | null;
  readonly identModel: string | null;
  readonly identFlash: string | null;
  readonly identLoader: string | null;
  readonly identVddVolts: number | null;
  readonly identFlashMb: number | null;
  readonly progressSteps: string[];
  readonly wireEvents: string[];
}

function parseWireKvPairs(rest: string): Readonly<Record<string, string>> {
  const tokenRegex = /([A-Za-z0-9_]+)=("([^"]*)"|\S+)/g;
  const entries = Array.from(rest.matchAll(tokenRegex)).flatMap((match): ReadonlyArray<readonly [string, string]> => {
    const key = match[WIRE_GROUP_KEY];
    if (!key) {
      return [];
    }
    const value = match[WIRE_GROUP_QUOTED] ?? match[WIRE_GROUP_RAW] ?? '';
    return [[key, value]];
  });
  return Object.fromEntries(entries);
}

function inferFlashMbFromChip(flashPart: string | undefined): number | null {
  const upper = (flashPart ?? '').toUpperCase();
  const matched = FLASH_CHIP_SIZE_RULES.find(([tokens]) => tokens.some(tok => upper.includes(tok)));
  return matched ? matched[1] : null;
}

function formatEvtLine(kv: Readonly<Record<string, string>>): ReadonlyArray<string> {
  const { kind, ...rest } = kv;
  if (!kind) {
    return [];
  }
  const attrs = Object.entries(rest)
    .map(([k, v]) => `${k}=${v}`)
    .join(' ');
  return [attrs ? `${kind} (${attrs})` : kind];
}

/**
 * Extracts structured `@P7501_IDENT`, `@PROG`, `@EVT`, and `@P7501_RES` frames cleanly.
 */
export function parseWireProtocolSummary(rawOutput: string): ParsedWireProtocolSummary {
  const lines = rawOutput.split(/\r?\n/).map(line => line.trim());
  const resLines = lines.filter(line => line.startsWith('@P7501_RES '));
  const lastResLine = resLines.at(-1);
  const resKv = lastResLine ? parseWireKvPairs(lastResLine.slice('@P7501_RES '.length)) : {};
  const elapsed = resKv.elapsed_us ? Number.parseInt(resKv.elapsed_us, DECIMAL_RADIX) : NaN;

  const identLine = lines.find(line => line.startsWith('@P7501_IDENT '));
  const identKv = identLine ? parseWireKvPairs(identLine.slice('@P7501_IDENT '.length)) : {};
  const vddMv = identKv.vdd_mv ? Number.parseInt(identKv.vdd_mv, DECIMAL_RADIX) : NaN;

  const progressSteps = lines
    .filter(line => line.startsWith('@PROG '))
    .flatMap(line => {
      const kv = parseWireKvPairs(line.slice('@PROG '.length));
      return kv.step ? [`${kv.step}${kv.msg ? ` ${kv.msg}` : ''}`] : [];
    });

  const wireEvents = lines
    .filter(line => line.startsWith('@EVT '))
    .flatMap(line => formatEvtLine(parseWireKvPairs(line.slice('@EVT '.length))));

  return {
    resStatus: resKv.status ?? null,
    resCode: resKv.code ?? null,
    resElapsedUs: Number.isFinite(elapsed) ? elapsed : null,
    identModel: identKv.model ?? null,
    identFlash: identKv.flash ?? null,
    identLoader: identKv.loader ?? null,
    identVddVolts: Number.isFinite(vddMv) && vddMv > 0 ? vddMv / MILLIVOLTS_PER_VOLT : null,
    identFlashMb: inferFlashMbFromChip(identKv.flash),
    progressSteps,
    wireEvents,
  };
}
