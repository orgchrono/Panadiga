/**
 * @file services/diagnosticParser/findingsRules.ts
 * @brief Telemetry pattern matchers synchronized with `SSOT_DIAG_SIGNATURE_RULES` from Rust SSOT.
 */

import { SSOT_DIAG_SIGNATURE_RULES } from '@/models/generated/ssot';
import type { PicoDiagnosticResult } from '@/models/pico';

interface TelemetryFindingRule {
  readonly code: string;
  readonly severity: 'info' | 'warning' | 'error';
  readonly eventSymbol?: string;
  readonly pattern?: RegExp;
  readonly summary: string;
  readonly detail: string;
}

const TELEMETRY_FINDING_RULES: ReadonlyArray<TelemetryFindingRule> = [
  {
    code: 'WDT_ACTIVE',
    severity: 'info',
    pattern: /\[WDT\]\s+wdt=ACTIVE/i,
    summary: 'Hardware Watchdog ACTIVE (4000ms, non-overridable)',
    detail:
      'RP2040/RP2350 hardware watchdog enforces automatic Hi-Z isolation and scratch recovery on stalls.',
  },
  {
    code: 'SD_HW_OK',
    severity: 'info',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_SD_HW_OK',
    summary: 'Stage-1 SD hardware 4-bit init OK',
    detail:
      'BOOT.BIN initialized SD card interface cleanly (... SD init end) before attempting DIGADIAG.FRM load.',
  },
  {
    code: 'FAT_FAIL',
    severity: 'error',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_FAT_FAIL',
    summary: 'Stage-1 FAT16 Mount/Open error on SD card (-10010/-10020/-10025)',
    detail:
      'BOOT.BIN failed to mount FAT16 or open DIGADIAG.FRM on the SD card (verify FAT16 SDSC formatting and root filename).',
  },
  {
    code: 'SD_DIAG_LOAD_FAIL',
    severity: 'error',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_SD_DIAG_LOAD_FAIL',
    summary: 'Stage-1 SD DIAG load failed (`DIGADIAG.FRM`)',
    detail:
      'FUN_4400a97c could not read 128 KiB from DIGADIAG.FRM (Mount/Open OK -> fat_read 0x20000 failed or <128 KiB).',
  },
  {
    code: 'SD_UNCOMPRESS_FAIL',
    severity: 'error',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_SD_UNCOMPRESS_FAIL',
    pattern: /SDDIAG\s+UNCOMPRESS\s+FAIL|fat\s+uncompress\s+error/i,
    summary: 'Stage-1 DIGADIAG.FRM EXTRHEAD/zlib unpack failed',
    detail:
      'DIGADIAG.FRM was read into RAM but decompression or Adler-32/EXTRFOOT validation failed.',
  },
  {
    code: 'DIAG_EXEC',
    severity: 'info',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_DIAG_EXEC',
    pattern: /Start\s+SDDIAG/i,
    summary: 'Stage-1 DIGADIAG.FRM decompressed & jumped (EXEC)',
    detail:
      'Stage-1 loader transferred execution cleanly to the 128 KiB DIGADIAG.FRM payload in SDRAM.',
  },
  {
    code: 'PANAMON_ACTIVE',
    severity: 'info',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_PANAMON',
    summary: 'PANAMON1> debug monitor active on SC0',
    detail:
      'Custom DIGADIAG.FRM monitor prompt PANAMON1> is online on P7501 SC0 UART and ready for memory/flash inspection.',
  },
  {
    code: 'ODD_NO_TRAY_80045300',
    severity: 'warning',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_ODD_TRAY_FAIL',
    summary: 'ODD cannot close tray / no disc (80045300)',
    detail:
      'Matshita optical drive reported tray/media absence (80045300) during boot probe.',
  },
  {
    code: 'BGO_INIT_HALT',
    severity: 'warning',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_BGO_FAIL',
    summary: 'BGO driver init failed -> BOOT halted in NORMAL mode (RSP offline)',
    detail:
      'FUN_4400836f (bgo_driver_init) failed and routed BOOT.BIN into boot_panic_or_halt_7812 before fmu_sd_dispatcher(1), leaving MAIN / _dev_SERIAL (PANARSP) unstarted.',
  },
  {
    code: 'HUNG_U99',
    severity: 'warning',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_HUNG_U99',
    summary: 'AM34 Main hung-up timer trap / U99 condition observed',
    detail:
      'Main firmware emitted hung-up time changed telemetry associated with U99 watchdog recovery stall.',
  },
  {
    code: 'LASER_APC_05MW_83A7',
    severity: 'error',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_CD_APC_ERR',
    pattern: /\b83[\s:-]?A7\b/i,
    summary: 'Drive laser APC fault 0x83A7 / 80040904 (0.5mW check prior to spin-up)',
    detail:
      'FUN_400db60c observed Ram88000194 = 0x000103A7 (0.5mW APC fault) prior to spindle motor actuation.',
  },
  {
    code: 'SD_STATUS_MINUS_91006',
    severity: 'warning',
    eventSymbol: 'P7501_SSOT_DIAG_EVT_SD_ERR',
    pattern: /\b-0x1637e\b/i,
    summary: 'GetSD_Status ACMD13 rejected SD card (-91006 / -0x1637e)',
    detail:
      'GetSD_Status inspected ACMD13 byte 2 (SD_CARD_TYPE) and returned -91006 (-0x1637e), blocking SD card bus initialization.',
  },
];

const SSOT_NEEDLES_BY_EVENT: ReadonlyMap<string, readonly string[]> = new Map(
  Array.from(new Set(SSOT_DIAG_SIGNATURE_RULES.map(rule => rule.eventSymbol))).map(symbol => [
    symbol,
    SSOT_DIAG_SIGNATURE_RULES.filter(rule => rule.eventSymbol === symbol).map(rule =>
      rule.needle.toLowerCase(),
    ),
  ]),
);

function matchesTelemetryRule(rawOutput: string, lowerOutput: string, rule: TelemetryFindingRule): boolean {
  if (rule.pattern?.test(rawOutput)) {
    return true;
  }
  if (!rule.eventSymbol) {
    return false;
  }
  const needles = SSOT_NEEDLES_BY_EVENT.get(rule.eventSymbol);
  return needles ? needles.some(needle => lowerOutput.includes(needle)) : false;
}

export function extractTelemetryFindings(
  rawOutput: string
): NonNullable<PicoDiagnosticResult['findings']> {
  const lowerOutput = rawOutput.toLowerCase();
  return TELEMETRY_FINDING_RULES.filter(rule => matchesTelemetryRule(rawOutput, lowerOutput, rule)).map(
    ({ code, severity, summary, detail }) => ({
      code,
      severity,
      summary,
      detail,
    })
  );
}
