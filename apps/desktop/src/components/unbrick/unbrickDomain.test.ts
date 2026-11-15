import { describe, expect, it } from 'vitest';
import {
  buildSdKitRequestPayload,
  buildTbcActionParams,
  resolveSdOverallBadgeVariant,
  resolveSeverityBadgeVariant,
  resolveTbcModeBadgeVariant,
} from './unbrickDomain';

describe('unbrickDomain pure transformations', () => {
  it('validates and builds deterministic SD kit requests', () => {
    const empty = buildSdKitRequestPayload({
      outputDir: '   ',
      panaDvdPath: 'PANA_DVD.FRM',
      bgoMode: 'bypass',
      mainMode: 'av_passthrough',
      includePanaSd: true,
      unlockHddDynamic: true,
    });
    expect(empty.errorKey).toBe('unbrick.errors.outputDirRequired');
    expect(empty.request).toBeNull();

    const valid = buildSdKitRequestPayload({
      outputDir: ' sd_kit ',
      panaDvdPath: '   ',
      bgoMode: 'uart_monitor',
      mainMode: 'factory_original',
      includePanaSd: false,
      unlockHddDynamic: false,
    });
    expect(valid.errorKey).toBeNull();
    expect(valid.request).toEqual({
      output_dir: 'sd_kit',
      pana_dvd_path: undefined,
      bgo_mode: 'uart_monitor',
      main_passthrough_mode: 'factory_original',
      include_pana_sd: false,
      unlock_hdd_dynamic: false,
    });
  });

  it('validates inspect vs patch TBC action parameters and badge variants', () => {
    const inspectMissing = buildTbcActionParams({
      action: 'inspect',
      mainInput: '',
      mainOutput: '',
      mode: 'av_passthrough',
    });
    expect(inspectMissing.inErrorKey).toBe('unbrick.errors.mainPathRequired');
    expect(inspectMissing.params).toBeNull();

    const patchMissingOut = buildTbcActionParams({
      action: 'patch',
      mainInput: ' MAIN.bin ',
      mainOutput: '  ',
      mode: 'av_passthrough',
    });
    expect(patchMissingOut.outErrorKey).toBe('unbrick.errors.mainOutRequired');
    expect(patchMissingOut.params).toBeNull();

    const patchValid = buildTbcActionParams({
      action: 'patch',
      mainInput: ' MAIN.bin ',
      mainOutput: ' MAIN_PATCHED.bin ',
      mode: 'av_passthrough',
    });
    expect(patchValid.params).toEqual({
      action: 'patch',
      inputPath: 'MAIN.bin',
      outPath: 'MAIN_PATCHED.bin',
      mode: 'av_passthrough',
    });

    expect(resolveSeverityBadgeVariant('pass')).toBe('success');
    expect(resolveSeverityBadgeVariant('warn')).toBe('warning');
    expect(resolveSeverityBadgeVariant('fail')).toBe('destructive');
    expect(resolveSeverityBadgeVariant('other')).toBe('secondary');
    expect(resolveSdOverallBadgeVariant('INCOMPLETE_OR_INVALID')).toBe('destructive');
    expect(resolveSdOverallBadgeVariant('READY')).toBe('success');
    expect(resolveTbcModeBadgeVariant('AV_PASSTHROUGH')).toBe('success');
  });
});
