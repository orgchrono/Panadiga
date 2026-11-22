import { test, expect } from '@playwright/test';
import { installMockIpcBridge } from './fixtures/mockIpcBridge';
import { enterEngineeringMode } from './fixtures/workbenchHelpers';

test.describe('HDD MEIHDFS MBR SH Inspection, Partition Extract & 250GB Capacity Patch E2E', () => {
  test.beforeEach(async ({ page }) => {
    await installMockIpcBridge(page);
    await page.goto('/');
    await enterEngineeringMode(page);
    await page.click('button[data-testid="nav-hdd"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'hdd');
  });

  test('inspects SH MBR & MEIHFS records, extracts partitions, and verifies safe flash lockout', async ({ page }) => {
    await expect(page.getByText(/SH \(System Storage Header\)|0x02000000/).first()).toBeVisible();

    await page.getByTestId('hdd-tab-meihdfs').click();
    await expect(page.locator('main')).toContainText(/REC|MEIHFS|0x/i);

    await page.getByRole('button', { name: /Extract|Extrair/i }).first().click();
    await expect(page.getByText(/partition_1_am34_os\.bin/).first()).toBeVisible();

    await page.getByTestId('hdd-tab-safe-flash').click();
    await expect(page.getByTestId('hdd-safe-disk-0')).toBeDisabled();
  });

  test('switches to capacity patcher in cap mode, previews 250GB RTSD #3 (0x07466528), and applies patch', async ({ page }) => {
    await page.getByTestId('hdd-tab-capacity').click();
    await page.getByRole('button', { name: /Explicit Cap|Teto Explícito|RTSD Preset/i }).first().click();
    await page.getByRole('button', { name: /250 GB/ }).first().click();
    await expect(page.getByText(/0x07466528/).first()).toBeVisible();
    await expect(page.getByText(/488,215,712|488\.215\.712/).first()).toBeVisible();

    await page.getByRole('button', { name: /Apply Dynamic Capacity Patch|Aplicar Patch Dinâmico de Capacidade/i }).last().click();
    await expect(page.getByText(/hdd_partition_firmware_0x02000000_PATCHED_DYNAMIC\.bin|a1b2c3d4e5f6/i).first()).toBeVisible();
  });
});
