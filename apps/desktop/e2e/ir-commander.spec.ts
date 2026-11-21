import { expect, test } from '@playwright/test';
import { installMockIpcBridge } from './fixtures/mockIpcBridge';
import { enterEngineeringMode } from './fixtures/workbenchHelpers';

test.describe('DIGA Commander Android-Only Isolation & Workflows E2E', () => {
  test('never exposes DIGA Commander navigation in default Desktop environment', async ({
    page,
  }) => {
    await installMockIpcBridge(page);
    await page.goto('/');
    await enterEngineeringMode(page);
    await expect(page.getByTestId('nav-dashboard')).toBeVisible();
    await expect(page.getByTestId('nav-ircommander')).toHaveCount(0);
  });

  test('renders DIGA Commander under Android APK runtime and drives Remote, Catalog, Service & Exynos HAL tabs', async ({
    page,
  }) => {
    await page.addInitScript(() => {
      (window as unknown as Record<string, unknown>).__PANADIGA_ANDROID_RUNTIME__ = true;
      (window as unknown as Record<string, unknown>).__PANADIGA_IR_BRIDGE__ = {
        isAvailable: () => true,
        getCapabilitiesJson: () =>
          JSON.stringify({
            hasEmitter: true,
            serviceFound: true,
            hasSecIrNode: true,
            hasFpgaNode: true,
            suAvailable: true,
            ranges: [{ min_hz: 36000, max_hz: 40000 }],
            device: {
              manufacturer: 'samsung',
              model: 'SM-T805M',
              hardware: 'universal5420',
              board: 'chagalllte',
              product: 'chagalllteub',
              device: 'chagalllte',
              android_release: '13',
              sdk_int: 33,
            },
          }),
        getSettingsJson: () =>
          JSON.stringify({ timingUnitOverride: 'AUTO', repeatCount: 1, customProfilesJson: '[]' }),
        saveSettings: () => JSON.stringify({ ok: true }),
        saveCustomProfilesJson: () => true,
        transmitPreparedJson: () =>
          JSON.stringify({ ok: true, backendUsed: 'Exynos sec_ir (PERIOD_SCALED)' }),
      };
    });

    await installMockIpcBridge(page);
    await page.goto('/');
    await enterEngineeringMode(page);

    await page.click('button[data-testid="nav-ircommander"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute(
      'data-view',
      'ircommander',
    );
    await expect(page.getByTestId('ir-commander-view')).toBeVisible();

    await page.getByTestId('ir-key-curated_diga_eh55_eur7659_0').click();
    await expect(page.getByText('WIRE: 02 20 B0 00 3D 8D')).toBeVisible();

    await page.getByTestId('ir-tab-catalog').click();
    await page.getByTestId('ir-catalog-search-input').fill('ES35V');
    await expect(page.getByTestId('ir-codeset-hit-curated_diga_combo_vhs_dvd')).toBeVisible();

    await page.getByTestId('ir-open-import-btn').click();
    await page.getByTestId('ir-import-textarea').fill('name: Power\ntype: parsed\nprotocol: Kaseikyo');
    await page.getByTestId('ir-import-confirm-btn').click();

    await page.getByTestId('ir-tab-service').click();
    await page.getByTestId('ir-run-macro-test_service_enter').click();

    await page.getByTestId('ir-tab-diagnostics').click();
    await expect(page.getByText('samsung SM-T805M')).toBeVisible();
  });
});
