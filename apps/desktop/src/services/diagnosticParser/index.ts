/**
 * @file services/diagnosticParser/index.ts
 * @brief Shared structured diagnostic parser used across USB CDC, Android USB-OTG, and BLE transports.
 */

import { PICO_FLOATING_PIN_VOLTS_THRESHOLD } from '@/constants';
import type { PicoDiagnosticResult } from '@/models/pico';
import { extractTelemetryFindings } from './findingsRules';
import {
  parseCapturedBytes,
  parseDumpRows,
  parseFlashMb,
  parseP7501DumpBytes,
  parsePincheckRows,
  parseTransitionRows,
  parseVddVolts,
  parseWireProtocolSummary,
} from './lineParsers';

export { extractTelemetryFindings, parseP7501DumpBytes };

function resolvePincheckVdd(rows: ReturnType<typeof parsePincheckRows>['rows']): number | null {
  const vddRow = rows.find(
    r => r.pin_label.includes('SWUART') || r.pin_label.includes('GP26') || r.pin_label.includes('VDD'),
  );
  const pinPdParsed = vddRow ? Number.parseFloat(vddRow.pull_down) : NaN;
  const pinHizParsed = vddRow ? Number.parseFloat(vddRow.hiz) : NaN;
  if (Number.isFinite(pinPdParsed) && pinPdParsed < PICO_FLOATING_PIN_VOLTS_THRESHOLD) return pinPdParsed;
  if (Number.isFinite(pinHizParsed)) return pinHizParsed;
  if (Number.isFinite(pinPdParsed)) return pinPdParsed;
  return null;
}

/**
 * Shared structured diagnostic parser used by Android BLE / USB-OTG and USB CDC transports.
 */
export function parseDiagnosticOutput(
  commandId: string,
  rawCommand: string,
  rawOutput: string,
  elapsedMs: number
): PicoDiagnosticResult {
  const norm = commandId.trim().toUpperCase();
  const normLower = norm.toLowerCase();
  const lower = rawOutput.toLowerCase();
  const linesTrimmed = lower.split(/\r?\n/).map(l => l.trim());
  const cmdOkPrefix = `${normLower}: ok`;
  const cmdErrPrefix = `${normLower}: error`;
  const bracketErrPrefix = `[${normLower}] error:`;

  const hasExplicitOk = linesTrimmed.some(
    line =>
      line.startsWith(cmdOkPrefix) ||
      (normLower === 'scan' && line === 'scan: completed') ||
      (normLower === 'ping' && (line === 'pong' || line.startsWith('pong '))) ||
      ((normLower === 'status' || normLower === 'vdd') && line.startsWith('vdd:')) ||
      (normLower === 'wdt' && (line.startsWith('wdt armed') || line.startsWith('wdt disarmed'))),
  );
  const hasPicoFatal = linesTrimmed.some(
    line =>
      line.startsWith(cmdErrPrefix) ||
      line.startsWith(bracketErrPrefix) ||
      line.startsWith('[safety interlock]') ||
      line.startsWith('[handshake] failed') ||
      line.startsWith('unknown command:'),
  );

  const wire = parseWireProtocolSummary(rawOutput);
  const legacyFailed =
    rawOutput.trim().length === 0 ||
    hasPicoFatal ||
    (!hasExplicitOk &&
      (lower.includes('failed') ||
        lower.includes('error') ||
        lower.includes('abort') ||
        lower.includes('safety interlock')));
  const failed =
    wire.resStatus !== null
      ? !(wire.resStatus.toUpperCase() === 'OK' && (wire.resCode === null || wire.resCode === '0' || wire.resCode.toUpperCase() === 'OK'))
      : legacyFailed;

  const pinData = norm === 'PINCHECK' ? parsePincheckRows(rawOutput) : { rows: [], riseTime: null };
  const transitionRows = parseTransitionRows(rawOutput);
  const dumpRows = norm === 'DUMP' ? parseDumpRows(rawOutput) : [];
  const meaningfulLines = rawOutput
    .split(/\r?\n/)
    .map(l => l.trim())
    .filter(l => l.length > 0 && !l.startsWith('PANADIGA>') && !l.startsWith('P7501>'));
  const displayLines = meaningfulLines.filter(l => !l.startsWith('@P7501_RES '));

  const vddVolts = parseVddVolts(rawOutput) ?? resolvePincheckVdd(pinData.rows) ?? wire.identVddVolts;

  return {
    command_id: norm,
    raw_command: rawCommand,
    success: !failed,
    elapsed_ms: elapsedMs,
    summary:
      displayLines[displayLines.length - 1] ??
      meaningfulLines[meaningfulLines.length - 1] ??
      `Sem resposta no barramento para '${rawCommand}'`,
    raw_output: rawOutput,
    vdd_volts: vddVolts,
    detected_flash_mb: parseFlashMb(rawOutput) ?? wire.identFlashMb,
    xreset_rise_us: pinData.riseTime,
    pincheck_rows: pinData.rows,
    transition_rows: transitionRows,
    dump_rows: dumpRows,
    captured_bytes: parseCapturedBytes(rawOutput),
    findings: extractTelemetryFindings(rawOutput),
    protocol_code: wire.resCode,
    protocol_elapsed_us: wire.resElapsedUs,
    ident_model: wire.identModel,
    ident_flash: wire.identFlash,
    ident_loader: wire.identLoader,
    progress_steps: wire.progressSteps,
    wire_events: wire.wireEvents,
  };
}
