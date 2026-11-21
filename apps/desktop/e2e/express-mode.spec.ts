import { test, expect } from '@playwright/test';
import { installMockIpcBridge } from './fixtures/mockIpcBridge';

test.describe('Panadiga Express Mode (3 Missions x 3 Clicks & JIT Safety) E2E', () => {
  test('launches in Express Mode by default without blocking on ComplianceDialog even when unaccepted', async ({
    page,
  }) => {
    await installMockIpcBridge(page);
    await page.addInitScript(() => {
      window.localStorage.removeItem('panadiga.workbench.compliance_ack_v1');
      window.localStorage.removeItem('panadiga.workbench.mode_v1');
    });

    await page.goto('/');
    await expect(page.getByTestId('express-shell')).toBeVisible();
    await expect(page.getByTestId('compliance-dialog')).toHaveCount(0);
    await expect(page.getByTestId('express-mission-sd')).toBeVisible();

    // Switching to Engineering while unaccepted must trigger ComplianceDialog gate
    await page.getByTestId('express-to-engineering-btn').click();
    await expect(page.getByTestId('engineering-shell')).toBeVisible();
    const complianceDialog = page.getByTestId('compliance-dialog');
    await expect(complianceDialog).toBeVisible();
    await page.getByTestId('compliance-ack-checkbox').click();
    await page.getByTestId('compliance-accept-btn').click();
    await expect(complianceDialog).toHaveCount(0);
  });

  test('Mission 1: builds FAT16B (0x06) SD card, validates SDXC warning + CONFIRM gate, shows RecorderDisplayGuide, and deep-links to Engineering Unbrick', async ({
    page,
  }) => {
    await installMockIpcBridge(page);
    await page.goto('/');

    await expect(page.getByTestId('express-shell')).toBeVisible();
    await expect(page.getByText('FAT16B (0x06) • 32 MiB • LBA 63')).toBeVisible();

    await page.getByTestId('express-sd-mode-physical').click();
    await page.getByTestId('express-sd-drive-select').selectOption('\\\\.\\PhysicalDrive4');
    await expect(page.getByText(/SDXC >= 64 GiB Compatibility Warning/i)).toBeVisible();

    const flashBtn = page.getByTestId('express-sd-flash-btn');
    await expect(flashBtn).toBeDisabled();

    await page.getByTestId('express-sd-confirm-input').fill('CONFIRM-PhysicalDrive4');
    await expect(flashBtn).toBeEnabled();
    await flashBtn.click();

    await expect(page.getByTestId('express-sd-success')).toBeVisible();

    const vfdGuide = page.getByTestId('recorder-display-guide');
    await expect(vfdGuide).toBeVisible();
    await expect(vfdGuide).toContainText('SD');
    await expect(vfdGuide).toContainText('READ');
    await expect(vfdGuide).toContainText('W 01..W 99');
    await expect(vfdGuide).toContainText('COMPLETE');
    await expect(vfdGuide).toContainText('F99');
    await expect(vfdGuide).toContainText('SW-OFF');

    await expect(page.getByTestId('express-sd-tech-content')).toHaveCount(0);
    await page.getByTestId('express-sd-tech-toggle').click();
    await expect(page.getByTestId('express-sd-tech-content')).toContainText('0x06');

    await page.getByTestId('express-sd-tech-open-engineering').click();
    await expect(page.getByTestId('engineering-shell')).toBeVisible();
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'unbrick');
  });

  test('Missions 2 & 3: enforces JIT PicoConnectionSafetySheet once per session for PANAMON1 U99 clean and NOR 8 MiB rescue', async ({
    page,
  }) => {
    await installMockIpcBridge(page);
    await page.goto('/');

    await page.getByTestId('express-tab-u99').click();
    await expect(page.getByTestId('express-mission-u99')).toBeVisible();

    await page.getByTestId('express-u99-run-btn').click();
    const safetySheet = page.getByTestId('pico-connection-safety-sheet');
    await expect(safetySheet).toBeVisible();
    await expect(safetySheet).toContainText('3.3V LVTTL');
    await expect(safetySheet).toContainText('NSW5V');
    await expect(safetySheet).toContainText('Open-Drain');
    await expect(safetySheet).toContainText('Never Power Pico from P7501');

    const confirmBtn = page.getByTestId('pico-safety-sheet-confirm-btn');
    await expect(confirmBtn).toBeDisabled();
    await page.getByTestId('pico-safety-sheet-checkbox').click();
    await expect(confirmBtn).toBeEnabled();
    await confirmBtn.click();

    await expect(page.getByTestId('express-u99-success')).toContainText('SA134..SA141');

    // Second electrical P7501 action in the same session runs directly without re-opening safety sheet
    await page.getByTestId('express-tab-nor').click();
    await expect(page.getByTestId('express-mission-nor')).toBeVisible();

    await page.getByTestId('express-nor-source-pico').click();
    await page.getByTestId('express-nor-inspect-btn').click();
    await expect(page.getByTestId('pico-connection-safety-sheet')).toHaveCount(0);
    await expect(page.getByText('1. Bootloader IPL (SA0..SA10)')).toBeVisible();
    await expect(page.getByText('2. BGO FTL Table (SA26..SA70)')).toBeVisible();
    await expect(page.getByText('3. NVRAM U99 (SA134..SA141)')).toBeVisible();

    await page.getByTestId('express-nor-repair-btn').click();
    await expect(page.getByTestId('express-nor-success')).toBeVisible();
  });

  test('Mission 4 Strict 6-Step Vault: resolves relational JSON chip spec, segregates raw vs frm, builds ISO 9660 1x, and gates composed HDD write via Mn103Vm', async ({
    page,
  }) => {
    await installMockIpcBridge(page);
    await page.goto('/');

    await expect(page.getByTestId('strict-workflow-stepper')).toBeVisible();
    await page.getByTestId('express-tab-vault').click();
    await expect(page.getByTestId('express-mission-vault')).toBeVisible();
    await expect(page.getByTestId('express-vault-resolved-spec')).toContainText('S29JL064H70TFI00');

    const writeBtn = page.getByTestId('express-vault-write-btn');
    await expect(writeBtn).toBeDisabled();

    await page.getByTestId('express-vault-init-btn').click();
    await expect(page.getByTestId('strict-workflow-stepper')).toContainText('sess_dmr_eh55');

    await page.getByTestId('express-vault-ingest-input').fill('ic5001_nor_8mb.bin');
    await page.getByTestId('express-vault-ingest-btn').click();
    await expect(page.getByTestId('express-vault-bucket-raw')).toContainText('ic5001_nor_8mb.bin');

    await page.getByTestId('express-vault-ingest-input').fill('PANA_DVD.FRM');
    await page.getByTestId('express-vault-ingest-btn').click();
    await expect(page.getByTestId('express-vault-bucket-frm')).toContainText('PANA_DVD.FRM');

    await page.getByTestId('express-dvd-create-btn').click();
    await expect(page.getByTestId('express-dvd-success')).toContainText('1x_or_drive_minimum_dao');

    await page.getByTestId('express-vault-run-gate-btn').click();
    await expect(page.getByTestId('express-vault-gate-status')).toContainText('PASS');

    await page.getByTestId('express-vault-hdd-confirm-input').fill('CONFIRM-PhysicalDrive2');
    await expect(writeBtn).toBeEnabled();
    await writeBtn.click();
    await expect(page.getByTestId('express-vault-write-success')).toBeVisible();
  });
});
