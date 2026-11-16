/**
 * @file services/diagnosticParser/findingsRules.ts
 * @brief Telemetry pattern matchers surfaced in structured P7501 diagnostic results.
 */

import type { PicoDiagnosticResult } from '@/models/pico';

const TELEMETRY_FINDING_RULES: ReadonlyArray<{
  readonly code: string;
  readonly severity: 'info' | 'warning' | 'error';
  readonly pattern: RegExp;
  readonly summary: string;
  readonly detail: string;
}> = [
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
    pattern: /\.\.\.\s*SD init end/i,
    summary: 'Stage-1 SD hardware 4-bit init OK',
    detail:
      'BOOT.BIN initialized SD card interface cleanly (... SD init end) before attempting DIGADIAG.FRM load.',
  },
  {
    code: 'SD_DIAG_LOAD_FAIL',
    severity: 'error',
    pattern: /SD diag load failed/i,
    summary: 'Stage-1 SD DIAG load failed (`DIGADIAG.FRM`)',
    detail:
      'FUN_4400a97c could not read 128 KiB from DIGADIAG.FRM (Mount/Open OK -> fat_read 0x20000 failed or <128 KiB).',
  },
  {
    code: 'SD_UNCOMPRESS_FAIL',
    severity: 'error',
    pattern: /SDDIAG\s+UNCOMPRESS\s+FAIL|fat\s+uncompress\s+error/i,
    summary: 'Stage-1 DIGADIAG.FRM EXTRHEAD/zlib unpack failed',
    detail:
      'DIGADIAG.FRM was read into RAM but decompression or Adler-32/EXTRFOOT validation failed.',
  },
  {
    code: 'DIAG_EXEC',
    severity: 'info',
    pattern: /PANAMON1>|Start\s+SDDIAG/i,
    summary: 'Stage-1 DIGADIAG.FRM decompressed & jumped (EXEC)',
    detail:
      'Stage-1 loader transferred execution cleanly to the 128 KiB DIGADIAG.FRM payload in SDRAM.',
  },
  {
    code: 'ODD_NO_TRAY_80045300',
    severity: 'warning',
    pattern: /\*\* can not close tray \*\*|\b80045300\b/i,
    summary: 'ODD cannot close tray / no disc (80045300)',
    detail:
      'Matshita optical drive reported tray/media absence (80045300) during boot probe.',
  },
  {
    code: 'BGO_INIT_HALT',
    severity: 'warning',
    pattern: /BGO\s+driver\s+init\s+failed/i,
    summary: 'BGO driver init failed -> BOOT halted in NORMAL mode (RSP offline)',
    detail:
      'FUN_4400836f (bgo_driver_init) failed and routed BOOT.BIN into boot_panic_or_halt_7812 before fmu_sd_dispatcher(1), leaving MAIN / _dev_SERIAL (PANARSP) unstarted.',
  },
  {
    code: 'LASER_APC_05MW_83A7',
    severity: 'error',
    pattern: /\b80040904\b|\b0x83A7\b|\b83[\s:-]?A7\b/i,
    summary: 'Drive laser APC fault 0x83A7 / 80040904 (0.5mW check prior to spin-up)',
    detail:
      'FUN_400db60c observed Ram88000194 = 0x000103A7 (0.5mW APC fault) prior to spindle motor actuation.',
  },
  {
    code: 'SD_STATUS_MINUS_91006',
    severity: 'warning',
    pattern: /SD\s+Status\s+ERROR\s*-91006|Set\s+SD\s+Bus\s+Width\s+fail\s+ERROR:-91006|\b-0x1637e\b/i,
    summary: 'GetSD_Status ACMD13 rejected SD card (-91006 / -0x1637e)',
    detail:
      'GetSD_Status inspected ACMD13 byte 2 (SD_CARD_TYPE) and returned -91006 (-0x1637e), blocking SD card bus initialization.',
  },
];

export function extractTelemetryFindings(
  rawOutput: string
): NonNullable<PicoDiagnosticResult['findings']> {
  return TELEMETRY_FINDING_RULES.filter(rule => rule.pattern.test(rawOutput)).map(
    ({ code, severity, summary, detail }) => ({
      code,
      severity,
      summary,
      detail,
    })
  );
}
