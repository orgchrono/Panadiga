import { test, expect } from '@playwright/test';
import { installMockIpcBridge } from './fixtures/mockIpcBridge';
import { enterEngineeringMode } from './fixtures/workbenchHelpers';

test.describe('Settings P7501 GPIO Guard, Manuals Search & Hex Inspector E2E', () => {
  test.beforeEach(async ({ page }) => {
    await installMockIpcBridge(page);
    await page.goto('/');
    await enterEngineeringMode(page);
  });

  test('enforces GP0..GP5 P7501 reserved GPIO guard, saves valid GPIO 16, and resets defaults', async ({ page }) => {
    await page.click('button[data-testid="nav-settings"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'settings');

    const gpioInput = page.locator('#rt-ir-gpio');
    await expect(gpioInput).toBeVisible();

    await gpioInput.fill('3');
    await page.getByTestId('runtime-settings-save-btn').click();
    await expect(page.locator('[role="alert"]').first()).toBeVisible();

    await gpioInput.fill('16');
    await page.getByTestId('runtime-settings-save-btn').click();
    await expect(page.locator('output').first()).toBeVisible();

    await page.getByTestId('runtime-settings-reset-btn').click();
    await expect(gpioInput).toHaveValue('6');
  });

  test('searches service manuals & engineering docs panel and jumps hex inspector to Partition 1', async ({ page }) => {
    await page.click('button[data-testid="nav-manuals"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'manuals');

    await expect(page.getByTestId('engineering-docs-panel')).toBeVisible();
    await page.getByTestId('eng-docs-search-input').fill('Matshita');
    await expect(page.getByTestId('engineering-docs-panel')).toContainText(/Matshita/i);

    await page.getByRole('button', { name: 'U99' }).first().click();
    await expect(page.locator('main')).toContainText(/S29JL064H|U99/i);

    await page.click('button[data-testid="nav-hexinspector"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'hexinspector');

    const jumpInput = page.locator('input[placeholder]').first();
    await jumpInput.fill('0x02000000');
    await page.getByRole('button', { name: /Jump|Ir|Go/i }).first().click();
    await expect(page.locator('main')).toContainText(/0x02000000/);
    await expect(page.locator('main')).toContainText(/LBA 65,536|LBA 65\.536/);
  });
});
