import { describe, expect, it } from 'vitest';
import {
  buildSdKitRequestPayload,
  buildTbcActionParams,
  resolveHddCapacitySelection,
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
      mainPath: '   ',
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

    const withMainOverride = buildSdKitRequestPayload({
      outputDir: 'sd_kit_main3060',
      panaDvdPath: 'firmware_and_images/PANA_DVD.FRM',
      mainPath: ' firmware_and_images/MAIN3060NP.bin ',
      bgoMode: 'bypass',
      mainMode: 'av_passthrough',
      includePanaSd: true,
      unlockHddDynamic: true,
    });
    expect(withMainOverride.errorKey).toBeNull();
    expect(withMainOverride.request).toEqual({
      output_dir: 'sd_kit_main3060',
      pana_dvd_path: 'firmware_and_images/PANA_DVD.FRM',
      main_path: 'firmware_and_images/MAIN3060NP.bin',
      bgo_mode: 'bypass',
      main_passthrough_mode: 'av_passthrough',
      include_pana_sd: true,
      unlock_hdd_dynamic: true,
    });

    expect(resolveHddCapacitySelection('dynamic')).toEqual({
      unlockHddDynamic: true,
      hddTargetGb: null,
    });
    expect(resolveHddCapacitySelection('250')).toEqual({
      unlockHddDynamic: false,
      hddTargetGb: 250,
    });

    const fixed250 = buildSdKitRequestPayload({
      outputDir: 'sd_kit_250',
      panaDvdPath: 'PANA_DVD.FRM',
      bgoMode: 'uart_monitor',
      mainMode: 'av_passthrough',
      loaderProfile: 'ver061',
      includePanaSd: true,
      ...resolveHddCapacitySelection('250'),
    });
    expect(fixed250.errorKey).toBeNull();
    expect(fixed250.request).toEqual({
      output_dir: 'sd_kit_250',
      pana_dvd_path: 'PANA_DVD.FRM',
      bgo_mode: 'uart_monitor',
      main_passthrough_mode: 'av_passthrough',
      loader_profile: 'ver061',
      include_pana_sd: true,
      unlock_hdd_dynamic: false,
      hdd_target_gb: 250,
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
