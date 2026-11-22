import { test, expect } from '@playwright/test';
import { installMockIpcBridge } from './fixtures/mockIpcBridge';
import { enterEngineeringMode } from './fixtures/workbenchHelpers';

test.describe('EH55 250GB SD Kit, Standalone TBC & Matshita Emulator E2E', () => {
  test.beforeEach(async ({ page }) => {
    await installMockIpcBridge(page);
    await page.goto('/');
    await enterEngineeringMode(page);
    await page.click('button[data-testid="nav-unbrick"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'unbrick');
  });

  test('selects regional stock/250GB presets, builds kit with explicit MAIN path, and plans PAL-M TBC', async ({ page }) => {
    const capSelect = page.getByTestId('sd-hdd-capacity-select');
    await expect(capSelect).toBeVisible();
    await expect(capSelect.locator('option[value="200"]')).toContainText(/200 GB.*US\/BR.*0x05D1DDBA/);
    await expect(capSelect.locator('option[value="160"]')).toContainText(/160 GB.*EU.*0x04A7D7C8/);

    await capSelect.selectOption('400');
    await expect(capSelect).toHaveValue('400');
    await capSelect.selectOption('dynamic');
    await expect(capSelect).toHaveValue('dynamic');
    await capSelect.selectOption('250');
    await expect(capSelect).toHaveValue('250');

    await page
      .getByTestId('sd-kit-main-path')
      .fill('firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin');
    await page.getByTestId('sd-kit-submit-btn').click();

    await expect(page.getByText(/DIGADIAG\.FRM/).first()).toBeVisible();
    await expect(page.getByText(/PANA__SD\.FRM/).first()).toBeVisible();

    await page.getByTestId('tbc-standalone-preset-btn').click();
    await page.getByTestId('tbc-tv-standard').selectOption('pal_m');
    await page.getByTestId('tbc-rsp-plan-btn').click();
    await expect(page.getByText(/MAGIC 0e0402/).first()).toBeVisible();
  });

  test('switches to Matshita R6-MV4 tab and simulates unlock_disc_frm boot sequence', async ({ page }) => {
    await page.getByTestId('unbrick-tab-matshita').click();
    const card = page.getByTestId('matshita-emulator-card');
    await expect(card).toBeVisible();

    await page.getByTestId('matshita-run-boot-btn').click();
    await expect(card.getByText(/MATSHITA/i).first()).toBeVisible();
  });
});
