import { describe, expect, it } from 'vitest';
import {
  buildInitialNvramProgress,
  buildSdFlashRequest,
  EXPRESS_PROGRESS_DONE_DEFAULT,
  type ExpressSdDriveTarget,
  filterSelectableExpressModels,
  formatExpressError,
  pickDefaultSafeSdPath,
  validateSdFlashSelection,
} from './expressMissionLogic';

const makeDrive = (overrides: Partial<ExpressSdDriveTarget>): ExpressSdDriveTarget => ({
  device_path: '/dev/sdb',
  safe_sd_target: true,
  expected_confirm_token: 'CONFIRM_SDB',
  ...overrides,
});

describe('expressMissionLogic', () => {
  it('filters AUTO placeholder from selectable catalog and picks first safe SD path', () => {
    const models = [
      { model_id: 'AUTO' },
      { model_id: 'DMR-EH55' },
      { model_id: 'DMR-ES35V' },
    ] as const;
    expect(filterSelectableExpressModels(models).map(m => m.model_id)).toEqual([
      'DMR-EH55',
      'DMR-ES35V',
    ]);

    const drives: readonly ExpressSdDriveTarget[] = [
      makeDrive({ device_path: '/dev/nvme0', safe_sd_target: false }),
      makeDrive({ device_path: '/dev/sdc', safe_sd_target: true }),
    ];
    expect(pickDefaultSafeSdPath(drives)).toBe('/dev/sdc');
    expect(pickDefaultSafeSdPath([])).toBeNull();
  });

  it('validates physical SD selection tokens and builds requests cleanly', () => {
    const drive = makeDrive({ device_path: '\\\\.\\PhysicalDrive3', expected_confirm_token: 'CONFIRM_3' });
    expect(validateSdFlashSelection('image', null, '')).toBeNull();
    expect(validateSdFlashSelection('physical', null, '')).toBeTruthy();
    expect(validateSdFlashSelection('physical', drive, 'WRONG')).toBeTruthy();
    expect(validateSdFlashSelection('physical', drive, ' CONFIRM_3 ')).toBeNull();

    const req = buildSdFlashRequest({
      sdTargetMode: 'physical',
      selectedSdDrive: drive,
      sdConfirmToken: ' CONFIRM_3 ',
      selectedModel: 'DMR-EH55',
      includePanaSd: true,
    });
    expect(req.devicePath).toBe('\\\\.\\PhysicalDrive3');
    expect(req.confirmToken).toBe('CONFIRM_3');
    expect(req.loaderProfile).toBe('ver061');
    expect(req.includePanaSd).toBe(true);
  });

  it('formats errors and initial NVRAM rescue progress deterministically', () => {
    expect(formatExpressError(new Error('boom'))).toBe('boom');
    expect(formatExpressError('plain failure')).toBe('plain failure');
    expect(buildInitialNvramProgress('COM10')).toEqual({
      port: 'COM10',
      stage: 'connect_bridge',
      current: 0,
      total: EXPRESS_PROGRESS_DONE_DEFAULT,
      detail: 'COM10',
    });
  });
});
