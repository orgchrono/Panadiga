import { test, expect } from '@playwright/test';
import { installMockIpcBridge } from './fixtures/mockIpcBridge';

test.describe('Pico BGO FTL 1->0 & DAVIS CH0/CH1 IDE Probe E2E', () => {
  test.beforeEach(async ({ page }) => {
    await installMockIpcBridge(page);
    await page.goto('/');
    await page.click('button[data-testid="nav-picobench"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'picobench');
  });

  test('executes 1-click NOR dump, F2 40/820 audit, dry-run/live repair, and DAVIS CH1 probe to HDD navigation', async ({ page }) => {
    await page.getByTestId('pico-section-bgo').click();
    await page.getByTestId('bgo-nor-dump-btn').click();
    await expect(page.getByText(/eh55_nor_bgo_cow\.bin/).first()).toBeVisible();

    await page.getByTestId('bgo-audit-btn').click();
    await expect(page.getByText(/40\/40.*820\/820/).first()).toBeVisible();

    await page.getByTestId('bgo-dry-run-btn').click();
    await expect(page.getByText(/PURE 1->0|1->0 PURO/i).first()).toBeVisible();

    await page.getByTestId('preflight-check-cowBackupSession').check();
    await page.getByTestId('preflight-check-upperNorUnlock').check();
    await page.getByTestId('preflight-check-noBootInterrupt').check();
    await page.getByTestId('bgo-confirm-input').fill('CONFIRM_UPPER_NOR');
    await page.getByTestId('bgo-live-repair-btn').click();
    await expect(page.getByText(/BOOTTRACE 15000 NOJIG -> BOOT NORMAL/).first()).toBeVisible();

    await page.getByTestId('pico-section-davis').click();
    await page.getByTestId('davis-probe-btn').click();
    await expect(page.getByText('ST3250820ACE').first()).toBeVisible();
    await expect(page.getByText(/0xC000100C GUARDED|0xC000100C PROTEGIDO/i).first()).toBeVisible();

    await page.getByTestId('davis-open-hdd-btn').click();
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'hdd');
  });
});
