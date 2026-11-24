import { describe, it, expect } from 'vitest';
import {
  buildCanonicalHardwareSsotSpec,
  CANONICAL_HARDWARE_SSOT_SPEC,
  PICO_MAX_VDD_CEILING_VOLTS,
  PICO_MIN_VDD_THRESHOLD_VOLTS,
  PICO_RP2350_CDC_PID,
} from '@/constants';
import hardwareCatalog from '../../../../crates/panadiga-core/src/ssot/hardware_catalog.json';
import { parseDiagnosticOutput, parseP7501DumpBytes } from './diagnosticParser';
import { simulateMockDiagnostic } from '@/test/mocks/mockDiagnostics';

describe('diagnosticParser & Hardware SSOT Consistency', () => {
  it('matches firmware SSOT constants and canonical P7501 UART pinout', () => {
    expect(PICO_MIN_VDD_THRESHOLD_VOLTS).toBe(2.5);
    expect(PICO_MAX_VDD_CEILING_VOLTS).toBe(3.45);
    expect(PICO_RP2350_CDC_PID).toBe(0x000f);
    expect(CANONICAL_HARDWARE_SSOT_SPEC.pinout).toHaveLength(6);
    expect(CANONICAL_HARDWARE_SSOT_SPEC.pinout[3].manual_pin).toBe(4);
    expect(CANONICAL_HARDWARE_SSOT_SPEC.pinout[4].signal_name).toContain('NSW5V');
    expect(CANONICAL_HARDWARE_SSOT_SPEC.pico_commands.length).toBeGreaterThanOrEqual(34);
  });

  it('parses PINCHECK matrix rows and XRESET rise time accurately', () => {
    const res = simulateMockDiagnostic('PINCHECK');
    expect(res.success).toBe(true);
    expect(res.pincheck_rows).toHaveLength(5);
    expect(res.xreset_rise_us).toBe('40 us');
    expect(res.vdd_volts).toBeCloseTo(3.28, 2);

    const liveHardwareOut = [
      '=== P7501 6-PIN ELECTRICAL MATRIX (PINCHECK) ===',
      'GP2 (Pin6 RXD   ) |  1   |     1     |    1    | STRONG HIGH (External Pull-Up / 3.3V Active)',
      'GP3 (Pin5 NSW5V ) |  1   |     1     |    1    | NSW5V HI-Z SENSE (Pulls Disabled Safe Rail)',
      'GP4 (Pin3 TXD   ) |  1   |     1     |    1    | STRONG HIGH (External Pull-Up / 3.3V Active)',
      'GP5 (Pin2 XRESET) |  1   |     1     |    1    | STRONG HIGH (Open-Drain Control)',
      'GP26(Pin1 SWUART) | 3.24V|   2.70V   |  3.26V  | P7501 SW_UART / Target Sense',
      'XRESET Release Rise Time (Hi-Z)     : 10 us',
      'PINCHECK: OK',
    ].join('\n');
    const liveParsed = parseDiagnosticOutput('PINCHECK', 'PINCHECK', liveHardwareOut, 393);
    expect(liveParsed.vdd_volts).toBeCloseTo(3.24, 2);
    expect(liveParsed.xreset_rise_us).toBe('10 us');

    const floatingOut = [
      'GP26(Pin1 SWUART) | 3.21V|   0.00V   |  3.28V  | P7501 SW_UART / Target Sense',
      'PINCHECK: OK',
    ].join('\n');
    const floatingParsed = parseDiagnosticOutput('PINCHECK', 'PINCHECK', floatingOut, 50);
    expect(floatingParsed.vdd_volts).toBe(0);
  });

  it('parses DUMP 32-bit words and little-endian ASCII representation', () => {
    const res = simulateMockDiagnostic('DUMP', '0x807F0000 16');
    expect(res.success).toBe(true);
    expect(res.dump_rows.length).toBeGreaterThanOrEqual(4);
    expect(res.dump_rows[0].address).toBe('0x807F0000');
    expect(res.dump_rows[0].words).toHaveLength(4);
  });

  it('marks FAILED or empty responses as unsuccessful', () => {
    const errRes = parseDiagnosticOutput('IDENTIFY', 'IDENTIFY', 'IDENTIFY: FAILED (Target offline)', 120);
    expect(errRes.success).toBe(false);
    expect(errRes.summary).toContain('FAILED');
  });

  it('extracts little-endian raw byte payloads from DUMP output for live NVRAM ingestion', () => {
    const sampleDump = [
      '0x807F0000: 44434241 78563412 FFFFFFFF 00000000',
      '0x807F0010: CAFEBABE DEADBEEF 01020304 0A0B0C0D',
    ].join('\n');

    const extracted = parseP7501DumpBytes(sampleDump);
    expect(extracted).not.toBeNull();
    expect(extracted.baseAddress).toBe(0x807f0000);
    expect(extracted.bytes.length).toBe(32);
    // Word 0x44434241 in little-endian -> 0x41 ('A'), 0x42 ('B'), 0x43 ('C'), 0x44 ('D')
    expect(extracted.bytes.slice(0, 4)).toEqual([0x41, 0x42, 0x43, 0x44]);
    // Word 0x78563412 -> 0x12, 0x34, 0x56, 0x78
    expect(extracted.bytes.slice(4, 8)).toEqual([0x12, 0x34, 0x56, 0x78]);
  });

  it('keeps explicit OK command results successful even when target boot log prints failed/ERROR', () => {
    const raw = [
      '=== UARTSTREAM PIO1 (115200 bps, 120 bytes, 8000 ms) ===',
      'BGO driver init failed[1].',
      '[SDCD] Set SD Bus Width fail ERROR:-91006',
      'UARTSTREAM: OK',
    ].join('\n');
    const parsed = parseDiagnosticOutput('UARTSTREAM', 'UARTSTREAM 8000 JIG:350', raw, 8050);
    expect(parsed.success).toBe(true);

    const errParsed = parseDiagnosticOutput(
      'IDENTIFY',
      'IDENTIFY',
      'IDENTIFY: FAILED (VDD=0.00V)',
      300,
    );
    expect(errParsed.success).toBe(false);
  });

  it('extracts structured findings for BGO halt, 0x83A7/80040904 laser APC, and SD Status -91006', () => {
    const raw = [
      'BGO driver init failed[1].',
      'ATAPI sense 80040904 / 83 A7 (Laser 0.5mW failed)',
      '[SDCD] SD Status ERROR -91006',
      'UARTSTREAM: OK',
    ].join('\n');
    const parsed = parseDiagnosticOutput('UARTSTREAM', 'UARTSTREAM 8000 JIG:350', raw, 8050);
    const codes = (parsed.findings ?? []).map((f) => f.code);
    expect(codes).toEqual([
      'BGO_INIT_HALT',
      'LASER_APC_05MW_83A7',
      'SD_STATUS_MINUS_91006',
    ]);
    const details = (parsed.findings ?? []).map((f) => f.detail).join(' ');
    expect(details).toContain('FUN_4400836f');
    expect(details).toContain('FUN_400db60c');
    expect(details).toContain('Ram88000194');
    expect(details).toContain('-0x1637e');
  });

  it('extracts WDT active status and Stage-1 SD funnel findings (SD_HW_OK, SD_DIAG_LOAD_FAIL, ODD_NO_TRAY_80045300)', () => {
    const wdt = simulateMockDiagnostic('WDT');
    expect(wdt.success).toBe(true);
    expect((wdt.findings ?? []).map((f) => f.code)).toContain('WDT_ACTIVE');

    const funnel = parseDiagnosticOutput(
      'DIAGWATCH',
      'DIAGWATCH 6000 JIG NONE',
      '... SD init end\nSD diag load failed[0].\n** can not close tray **\nDIAGWATCH: OK',
      6000,
    );
    expect(funnel.success).toBe(true);
    expect((funnel.findings ?? []).map((f) => f.code)).toEqual([
      'SD_HW_OK',
      'SD_DIAG_LOAD_FAIL',
      'ODD_NO_TRAY_80045300',
    ]);
  });

  it('derives both pt-BR and en-US SSOT specs cleanly from shared hardware_catalog.json', () => {
    const ptSpec = CANONICAL_HARDWARE_SSOT_SPEC;
    const enSpec = buildCanonicalHardwareSsotSpec('en-US');
    const catalogIds = hardwareCatalog.commands.map((c) => c.id);

    expect(ptSpec.pico_commands.map((c) => c.id)).toEqual(catalogIds);
    expect(enSpec.pico_commands.map((c) => c.id)).toEqual(catalogIds);
    expect(ptSpec.flash_geometries.map((g) => g.nvram_base_hex)).toEqual([
      '0x801F0000',
      '0x803F0000',
      '0x807F0000',
      '0x80FE0000',
    ]);

    const allCmds = [...ptSpec.pico_commands, ...enSpec.pico_commands];
    allCmds.forEach(cmd => {
      expect(cmd.syntax).not.toContain('{{');
      cmd.arg_options.forEach(opt => expect(opt.value).not.toContain('{{'));
    });
    allCmds
      .filter(cmd => cmd.default_arg !== null && cmd.default_arg !== undefined)
      .forEach(cmd => expect(cmd.default_arg).not.toContain('{{'));

    const dump = ptSpec.pico_commands.find((c) => c.id === 'DUMP');
    const drv = ptSpec.pico_commands.find((c) => c.id === 'DRVPANA');
    expect(dump?.default_arg).toBe('0x807F0000 16');
    expect(dump?.arg_placeholder).toBe('0x807F0000 16');
    expect(drv?.arg_options[1]?.value).toBe('PASS 0x4d415453 0x48495441');
  });

  it('treats @P7501_RES as authoritative and extracts @P7501_IDENT, @PROG, and @EVT frames', () => {
    const okRes = parseDiagnosticOutput(
      'IDENTIFY',
      'IDENTIFY',
      '@P7501_IDENT vdd_mv=3295 nsw5v=1 loader="Loader ver0.61" flash=S29JL064H model=DMR-EH55\n@P7501_RES cmd=IDENTIFY status=OK code=0 elapsed_us=1800',
      35,
    );
    expect(okRes.success).toBe(true);
    expect(okRes.ident_model).toBe('DMR-EH55');
    expect(okRes.ident_flash).toBe('S29JL064H');
    expect(okRes.ident_loader).toBe('Loader ver0.61');
    expect(okRes.detected_flash_mb).toBe(8);
    expect(okRes.vdd_volts).toBeCloseTo(3.295, 3);
    expect(okRes.protocol_code).toBe('0');
    expect(okRes.protocol_elapsed_us).toBe(1800);

    const errRes = parseDiagnosticOutput(
      'JOB',
      'JOB NVRAM_CLEAN CONFIRM',
      '@PROG step=2/3 msg=WRITE_P2_VAULT_SLOT_1\n@EVT kind=M0KEEPALIVE state=AUTO_OFF reason=TARGET_UNPOWERED\n@P7501_RES cmd=JOB status=ERR code=E05 elapsed_us=412',
      50,
    );
    expect(errRes.success).toBe(false);
    expect(errRes.protocol_code).toBe('E05');
    expect(errRes.progress_steps).toEqual(['2/3 WRITE_P2_VAULT_SLOT_1']);
    expect(errRes.wire_events).toEqual(['M0KEEPALIVE (state=AUTO_OFF reason=TARGET_UNPOWERED)']);
  });

  it('surfaces SSOT-synced FAT_FAIL, PANAMON_ACTIVE, and HUNG_U99 findings', () => {
    const raw = [
      'FAT Mount failed(-10010)',
      'Main hung-up time changed 255[s].',
      'PANAMON1>',
      'DIAGWATCH: OK',
    ].join('\n');
    const parsed = parseDiagnosticOutput('DIAGWATCH', 'DIAGWATCH 6000 JIG NONE', raw, 6000);
    const codes = (parsed.findings ?? []).map(f => f.code);
    expect(codes).toContain('FAT_FAIL');
    expect(codes).toContain('PANAMON_ACTIVE');
    expect(codes).toContain('HUNG_U99');
  });

  it('decodes @P7501_BOOT_CLASS and @P7501_TBUS_AUDIT telemetry frames', () => {
    const raw = [
      '@P7501_BOOT_CLASS verdict=CLEAN_MAIN_OK build=3060NP stage2=1 avout_ok=1 vmute0=1 ntsc=1 rsv_ok=1 hdd_fail=0 bgo_err=0 fpc_err=0 fpc_to=0 iic_err=0 odd_err=0 op7f_cnt=0 op7f=NONE',
      '@P7501_TBUS_AUDIT frames=6 br_rejected=1 nak7f=1 ops=A9',
      'BOOTTRACE: OK',
      '@P7501_RES cmd=BOOTTRACE status=OK code=0 elapsed_us=12000',
    ].join('\n');
    const parsed = parseDiagnosticOutput('BOOTTRACE', 'BOOTTRACE 90000 NORMAL', raw, 12000);
    expect(parsed.success).toBe(true);
    expect(parsed.boot_class?.verdict).toBe('CLEAN_MAIN_OK');
    expect(parsed.boot_class?.build).toBe('3060NP');
    expect(parsed.boot_class?.avout_ok).toBe(true);
    expect(parsed.boot_class?.op7f).toEqual([]);
    const codes = (parsed.findings ?? []).map(f => f.code);
    expect(codes).toContain('BOOT_CLASS_CLEAN_MAIN_OK');
    expect(codes).toContain('TBUS_AUDIT');
    expect(parsed.wire_events).toContain('BOOT_CLASS (verdict=CLEAN_MAIN_OK build=3060NP avout_ok=1 fpc_err=0 bgo_err=0)');
  });

  it('captures thin @P7501_FRAME, @P7501_HOTPLUG, @P7501_LIVE_MAP, and @P7501_DIAG_DEEP envelopes', () => {
    const raw = [
      '@P7501_FRAME seq=1 ts_us=500 type=MMIO len=20 crc32=1234ABCD reg=AIOF_VMOD val=0x8020',
      '@P7501_HOTPLUG event=ONLINE_AWAKE vdd_mv=3260 nsw5v=1 recoveries=1',
      '@P7501_LIVE_MAP mode=FULL bus=PANAMON1 frames=12 crc_err=0',
      '@P7501_DIAG_DEEP hw=M6930LB market=PL cav=N/PAL avsel=8/1 gui=6 avin_iic=0:0x0@3756',
      '@P7501_GPS_ENV build=3060NP hw=M6930LB cal=0x03 affine=1 delta=0',
      '@P7501_ROUTE dom=VIDEO target=0x88000000 payload=video_mmio_dump guard=ALLOW next=PAYLOAD_RUN',
      '@P7501_XRAY max_stage=S10_VFD_VV fault_stage=NONE stages_mask=0x07FF events=12 errors=0',
      '@P7501_SURG op=LOG entries=2 cursor=2 undo=2 redo=0',
      'LIVE_MAP: OK',
      '@P7501_RES cmd=LIVE_MAP status=OK code=0 elapsed_us=3100',
    ].join('\n');
    const parsed = parseDiagnosticOutput('LIVE_MAP', 'LIVE_MAP FULL 3000', raw, 3100);
    expect(parsed.success).toBe(true);
    expect(parsed.wire_events).toEqual([
      'FRAME:MMIO (seq=1 ts_us=500 type=MMIO len=20 crc32=1234ABCD reg=AIOF_VMOD val=0x8020)',
      'HOTPLUG (event=ONLINE_AWAKE vdd_mv=3260 nsw5v=1 recoveries=1)',
      'LIVE_MAP (mode=FULL bus=PANAMON1 frames=12 crc_err=0)',
      'DIAG_DEEP (hw=M6930LB market=PL cav=N/PAL avsel=8/1 gui=6 avin_iic=0:0x0@3756)',
      'GPS_ENV (build=3060NP hw=M6930LB cal=0x03 affine=1 delta=0)',
      'ROUTE (dom=VIDEO target=0x88000000 payload=video_mmio_dump guard=ALLOW next=PAYLOAD_RUN)',
      'XRAY (max_stage=S10_VFD_VV fault_stage=NONE stages_mask=0x07FF events=12 errors=0)',
      'SURG (op=LOG entries=2 cursor=2 undo=2 redo=0)',
    ]);
  });
});
