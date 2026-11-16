/**
 * @file services/diagnosticParser/lineParsers.ts
 * @brief Low-level line and word parsers for P7501 PINCHECK, transition counters, VDD, and hex dump payloads.
 */

import type { BusTransitionRow, DumpWordRow, PincheckRow } from '@/models/pico';

export function parsePincheckRows(rawOutput: string): { rows: PincheckRow[]; riseTime: string | null } {
  const rows: PincheckRow[] = [];
  let riseTime: string | null = null;

  for (const line of rawOutput.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (
      trimmed.includes('|') &&
      (trimmed.startsWith('GP') || trimmed.includes('Pin')) &&
      !trimmed.startsWith('Pin ') &&
      !trimmed.includes('---+')
    ) {
      const parts = trimmed.split('|').map(s => s.trim());
      if (parts.length >= 5) {
        rows.push({
          pin_label: parts[0],
          hiz: parts[1],
          pull_down: parts[2],
          pull_up: parts[3],
          state: parts[4],
        });
      }
    } else if (trimmed.includes('XRESET Release Rise Time')) {
      const idx = trimmed.indexOf(':');
      if (idx >= 0) {
        riseTime = trimmed.slice(idx + 1).trim();
      }
    }
  }

  return { rows, riseTime };
}

export function parseTransitionRows(rawOutput: string): BusTransitionRow[] {
  const rows: BusTransitionRow[] = [];
  for (const line of rawOutput.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed.includes('Final=') && trimmed.includes('Transitions=') && trimmed.includes(':')) {
      const colonIdx = trimmed.indexOf(':');
      const left = trimmed.slice(0, colonIdx).trim();
      const right = trimmed.slice(colonIdx + 1);
      const finalMatch = right.match(/Final=(\d+)/);
      const transMatch = right.match(/Transitions=(\d+)/);
      const leftParts = left.split(/\s+/);
      rows.push({
        signal: leftParts[0] || 'SIG',
        gpio: (leftParts[1] || '').replace(/[()]/g, ''),
        final_level: finalMatch ? Number.parseInt(finalMatch[1], 10) : null,
        transitions: transMatch ? Number.parseInt(transMatch[1], 10) : 0,
      });
    }
  }
  return rows;
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

  const parsedAddr = Number.parseInt(addrRaw, 16);
  if (Number.isNaN(parsedAddr)) return null;

  const words: string[] = [];
  const bytes: number[] = [];
  const valsPart = trimmed.slice(colonIdx + 1).trim();
  for (const tok of valsPart.split(/\s+/)) {
    const cleanTok = tok.replace(/^0x/i, '');
    if (!/^[0-9a-fA-F]{8}$/.test(cleanTok)) continue;
    const value = Number.parseInt(cleanTok, 16) >>> 0;
    words.push(`0x${cleanTok.toUpperCase()}`);
    bytes.push(value & 0xff, (value >>> 8) & 0xff, (value >>> 16) & 0xff, (value >>> 24) & 0xff);
  }

  return { addrHex: addrRaw.toUpperCase(), parsedAddr, words, bytes };
}

export function parseDumpRows(rawOutput: string): DumpWordRow[] {
  const rows: DumpWordRow[] = [];
  for (const line of rawOutput.split(/\r?\n/)) {
    const parsed = parseDumpLine(line, 8);
    if (!parsed || parsed.words.length === 0) continue;
    const ascii = parsed.bytes.map(b => (b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : '.')).join('');
    rows.push({
      address: `0x${parsed.addrHex}`,
      words: parsed.words,
      ascii,
    });
  }
  return rows;
}

export function parseVddVolts(rawOutput: string): number | null {
  const match = rawOutput.match(/VDD[=:\s]+(\d+(?:\.\d+)?)\s*V/i);
  if (!match) return null;
  const v = Number.parseFloat(match[1]);
  return Number.isFinite(v) && v > 0.01 ? v : null;
}

export function parseCapturedBytes(rawOutput: string): number | null {
  const match = rawOutput.match(/(\d+)\s+bytes/i);
  return match ? Number.parseInt(match[1], 10) : null;
}

export function parseFlashMb(rawOutput: string): number | null {
  for (const mb of [16, 8, 4, 2]) {
    if (rawOutput.includes(`${mb} MB`)) return mb;
  }
  return null;
}

/**
 * Decodes `ADDR: w0 w1 w2 w3` lines returned by the Pico `DUMP` command into a Little-Endian byte array.
 */
export function parseP7501DumpBytes(rawOutput: string): { baseAddress: number | null; bytes: number[] } {
  const bytes: number[] = [];
  let baseAddress: number | null = null;

  for (const rawLine of rawOutput.split(/\r?\n/)) {
    const parsed = parseDumpLine(rawLine, 4);
    if (!parsed) continue;
    if (baseAddress === null) {
      baseAddress = parsed.parsedAddr;
    }
    bytes.push(...parsed.bytes);
  }

  return { baseAddress, bytes };
}
