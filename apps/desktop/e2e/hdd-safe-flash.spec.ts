import { test, expect } from '@playwright/test';
import { installMockIpcBridge } from './fixtures/mockIpcBridge';

test.describe('HDD 5GB-2TB Safe Target Flash E2E', () => {
  test.beforeEach(async ({ page }) => {
    await installMockIpcBridge(page);
    await page.goto('/');
    await page.click('button[data-testid="nav-hdd"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'hdd');
  });

  test('enforces Disk 0 & NVMe lockouts, computes 5 GB SAMSUNG SV0511D RTSD, and flashes PhysicalDrive2 with SHA-256 read-back', async ({ page }) => {
    await page.getByTestId('hdd-tab-safe-flash').click();

    await expect(page.getByTestId('hdd-safe-disk-0')).toBeDisabled();
    await expect(page.getByTestId('hdd-safe-disk-1')).toBeDisabled();
    await expect(page.getByTestId('hdd-safe-disk-2')).toBeEnabled();

    await page.getByTestId('hdd-safe-cap-5').click();
    await expect(page.getByTestId('hdd-safe-5gb-badge')).toBeVisible();
    await expect(page.getByText(/0x24C388/)).toBeVisible();

    await page.getByTestId('preflight-check-backup64k').check();
    await page.getByTestId('preflight-check-wipeLayout').check();
    await page.getByTestId('preflight-check-capacityRange').check();

    await page.getByTestId('hdd-safe-confirm-input').fill('CONFIRM-PhysicalDrive2');
    await page.getByTestId('hdd-safe-flash-btn').click();

    await expect(page.getByText(/READ-BACK SHA-256 VERIFIED|SHA-256 DE READ-BACK VERIFICADO/i).first()).toBeVisible();
    await expect(page.getByText(/pre_flash_64k_backup\.bin/).first()).toBeVisible();
  });
});
