import { test, expect } from '@playwright/test';
import { installMockIpcBridge } from './fixtures/mockIpcBridge';
import { enterEngineeringMode } from './fixtures/workbenchHelpers';

test.describe('Calibration & Standalone TBC/SD Hardening E2E', () => {
  test.beforeEach(async ({ page }) => {
    await installMockIpcBridge(page);
    await page.goto('/');
    await enterEngineeringMode(page);
    await page.click('button[data-testid="nav-unbrick"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'unbrick');
  });

  test('executes standalone 5-bypass TBC lab and hardens ver0.61 SD unbrick kit', async ({ page }) => {
    await expect(page.getByTestId('unbrick-tab-tbc-sd')).toBeVisible();
    await page.getByTestId('tbc-standalone-preset-btn').click();
    await page.getByTestId('tbc-rsp-plan-btn').click();
    await expect(page.getByText(/MAGIC 0e0402/).first()).toBeVisible();

    await page.getByTestId('tbc-confirm-input').fill('APPLY_MAIN_TBC_COW');
    await page.getByTestId('tbc-apply-btn').click();
    await expect(page.getByText(/MAIN3410PQ_STANDALONE_TBC\.bin/).first()).toBeVisible();

    await page.getByTestId('sd-audit-disk-btn').click();
    await expect(page.getByText(/FAT16/).first()).toBeVisible();

    await page.getByTestId('preflight-check-autoFormatD4D5').check();
    await page.getByTestId('preflight-check-sd91006Fat16').check();
    await page.getByTestId('preflight-check-p59001P7402').check();
    await page.getByTestId('sd-harden-build-btn').click();
    await expect(page.getByText(/DUAL PARITY OK|PARIDADE DUAL OK/i).first()).toBeVisible();
  });

  test('inspects PAL-M region calibration and generates CoW artifacts safely', async ({ page }) => {
    await page.getByTestId('unbrick-tab-calibration').click();
    await page.getByTestId('calibration-inspect-btn').click();
    await expect(page.getByText('AIOF_VMODESEL').first()).toBeVisible();
    await expect(page.getByText('0x8021').first()).toBeVisible();

    await page.getByTestId('preflight-check-distinctCow').check();
    await page.getByTestId('preflight-check-backupReady').check();
    await page.getByTestId('preflight-check-timerMcuAck').check();

    await page.getByTestId('calibration-confirm-input').fill('CALIBRATE_REGION_COW');
    await page.getByTestId('calibration-apply-btn').click();
    await expect(page.getByText(/ifodev2_calibrated\.bin/).first()).toBeVisible();
  });
});
