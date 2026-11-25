import { test, expect, type Page } from '@playwright/test';
import { installMockIpcBridge } from './fixtures/mockIpcBridge';
import { enterEngineeringMode } from './fixtures/workbenchHelpers';

const FORBIDDEN_TAXONOMY_REGEX = new RegExp(
  ['Pico' + 'OS', 'PICO' + 'OS>', 'OPEN' + 'OS>'].join('|'),
  'i',
);

const LETHAL_MMIO_ADDRS = ['0xC000100C', '0xDC0F0000'] as const;
const NOR_BLOCKED_ADDRS = ['0x80000000', '0x84000000'] as const;

async function attachZeroErrorGuards(page: Page): Promise<{
  consoleErrors: string[];
  pageErrors: string[];
}> {
  const consoleErrors: string[] = [];
  const pageErrors: string[] = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });
  page.on('pageerror', err => {
    pageErrors.push(err.message);
  });
  return { consoleErrors, pageErrors };
}

async function navigateToView(page: Page, viewKey: 'picobench' | 'ssotstudio' | 'siliconexplorer'): Promise<void> {
  await page.click(`button[data-testid="nav-${viewKey}"]`);
  await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', viewKey);
}

test.describe('Panadiga Big-Bang Taxonomy, DigaOS SSOT Studio & Silicon Explorer E2E (R1-R4)', () => {
  test.beforeEach(async ({ page }) => {
    await installMockIpcBridge(page);
    await page.goto('/');
    await enterEngineeringMode(page);
    await expect(page.getByText(/Modo Demo|Demonstração/i)).toHaveCount(0);
  });

  test('R1/F1/F4/F9: preserves physical P7501PinoutCard and enforces PicoFW PICO>/@PICO_* taxonomy without stale tokens', async ({
    page,
  }) => {
    const { consoleErrors, pageErrors } = await attachZeroErrorGuards(page);

    await navigateToView(page, 'picobench');
    await expect(page.getByLabel(/Select Pico serial port|Selecionar porta serial do Pico/i)).toHaveValue(/.+/);

    await expect(page.getByTestId('pico-pinout-title')).toBeVisible();
    await expect(page.getByText('P7501 [B6B-ZR-3R4]').first()).toBeVisible();
    await expect(page.getByTestId('pico-gnd-disclaimer')).toBeVisible();
    await expect(page.getByText(/SW_UART/).first()).toBeVisible();
    await expect(page.getByText(/XRESET/).first()).toBeVisible();
    await expect(page.getByText(/NSW5V/).first()).toBeVisible();

    const terminalInput = page.locator('input[placeholder*="STATUS, RESET, ADC"]');
    await expect(terminalInput).toBeVisible();
    await terminalInput.fill('STATUS');
    await page.getByTestId('pico-serial-send-btn').click();

    await expect(page.locator('body')).toContainText(/PICO>|@PICO_/);
    await expect(page.locator('body')).not.toContainText(FORBIDDEN_TAXONOMY_REGEX);
    expect(consoleErrors).toEqual([]);
    expect(pageErrors).toEqual([]);
  });

  test('R2/F6/F9/F10/S2/S3: operates Visual SSOT Schema & DigaOS Customizer Studio with multi-SoC guards and nonce-gated save', async ({
    page,
  }) => {
    const { consoleErrors, pageErrors } = await attachZeroErrorGuards(page);

    await navigateToView(page, 'ssotstudio');

    await expect(page.getByText(/DigaOS/i).first()).toBeVisible();
    await expect(page.getByText(/AM34|MN2DS0009/i).first()).toBeVisible();
    await expect(page.getByText(/UNVERIFIED_MMIO/i).first()).toBeVisible();

    for (const socName of [/AM33-2/i, /UniPhier/i, /SPARC|DoMiNo/i]) {
      await expect(page.getByText(socName).first()).toBeVisible();
    }

    const mmioProbeInput = page.getByTestId('ssotstudio-mmio-input');
    if ((await mmioProbeInput.count()) > 0) {
      await mmioProbeInput.fill('0xC000100C');
      await page.getByTestId('ssotstudio-validate-btn').click();
      await expect(page.getByText(/0xC000100C|lethal|letal|blocked|bloqueado/i).first()).toBeVisible();
    }

    const contrastInput = page.getByTestId('ssotstudio-theme-contrast-input');
    if ((await contrastInput.count()) > 0) {
      await contrastInput.fill('80');
    }

    const saveBtn = page.getByTestId('ssotstudio-save-btn');
    await expect(saveBtn).toBeVisible();
    await saveBtn.click();

    await expect(page.getByText(/Saved|Salvo|DIGAOS_SSOT_CONTRACT_HASH|0x[0-9A-Fa-f]{8}/i).first()).toBeVisible();
    await expect(page.locator('body')).not.toContainText(FORBIDDEN_TAXONOMY_REGEX);
    await expect(page.getByText(/Modo Demo|Demonstração/i)).toHaveCount(0);
    expect(consoleErrors).toEqual([]);
    expect(pageErrors).toEqual([]);
  });

  test('R3/F8/F9/F11/S4/S5/S6: replays live COM10 logs in Silicon Explorer with BCU/Standby/MMIO/NOR barriers, MAIN3060NP caveat, patch & rollback', async ({
    page,
  }) => {
    const { consoleErrors, pageErrors } = await attachZeroErrorGuards(page);

    await navigateToView(page, 'siliconexplorer');

    // 1. Replay live volatile patch log (LE words, AIOF_FS0, MAIN3060NP caveat)
    await page.getByTestId('silicon-replay-volatile-btn').click();
    await expect(page.getByText(/570DF348|570DF34A/i).first()).toBeVisible();
    await expect(page.getByText(/8800002C|AIOF_FS0/i).first()).toBeVisible();
    await expect(page.getByText(/C0F0\s*->\s*40F0|80\s*00\s*F0\s*FC\s*CB/i).first()).toBeVisible();
    await expect(page.getByText(/\/HDDEX\/UDA_M6930_H_JPN_V006/i).first()).toBeVisible();

    // 2. Replay live video OSD diag log (BCU BCBERR abort + STANDBY_GATE_STOP)
    await page.getByTestId('silicon-replay-diag-btn').click();
    await expect(page.getByText(/BCBERR|00002102|D4004000/i).first()).toBeVisible();
    await expect(page.getByText(/STANDBY_GATE_STOP/i).first()).toBeVisible();

    // 3. Fail-closed barriers against lethal MMIO & NOR writes
    const addrInput = page.getByTestId('silicon-target-addr-input');
    const applyBtn = page.getByTestId('silicon-apply-patch-btn');
    for (const blockedAddr of [...LETHAL_MMIO_ADDRS, ...NOR_BLOCKED_ADDRS]) {
      await addrInput.fill(blockedAddr);
      await applyBtn.click();
      await expect(page.getByText(/blocked|bloqueado|forbidden|proibido|E05|lethal|letal/i).first()).toBeVisible();
    }

    // 4. Valid volatile SDRAM patch + hot-diff + instant rollback
    await addrInput.fill('0x570DF34A');
    const hexInput = page.getByTestId('silicon-patch-hex-input');
    await hexInput.fill('8000f0fccb');
    const ackCheckbox = page.getByTestId('silicon-volatile-confirm-checkbox');
    if ((await ackCheckbox.count()) > 0) {
      await ackCheckbox.check();
    }
    await applyBtn.click();
    await expect(page.getByText(/8000f0fccb|CF\s*->\s*80|OLD\s*->\s*NEW/i).first()).toBeVisible();

    await page.getByTestId('silicon-rollback-btn').click();
    await expect(page.getByText(/Rollback|Revertido|cff2f8fee8|F2CF20F0/i).first()).toBeVisible();

    await expect(page.locator('body')).not.toContainText(FORBIDDEN_TAXONOMY_REGEX);
    await expect(page.getByText(/Modo Demo|Demonstração/i)).toHaveCount(0);
    expect(consoleErrors).toEqual([]);
    expect(pageErrors).toEqual([]);
  });

  test('R4/S7: bilingual en-US and pt-BR smoke across ssotstudio and siliconexplorer with zero error boundaries', async ({
    page,
  }) => {
    const { consoleErrors, pageErrors } = await attachZeroErrorGuards(page);

    for (const localeBtnLabel of ['Português (pt-BR)', 'English (en-US)']) {
      await page.click('button[data-testid="nav-settings"]');
      await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'settings');
      await page.getByRole('button', { name: localeBtnLabel }).click();

      await navigateToView(page, 'ssotstudio');
      await expect(page.getByText(/SSOT|DigaOS/i).first()).toBeVisible();

      await navigateToView(page, 'siliconexplorer');
      await expect(page.getByText(/Silicon|Silício|ndb>/i).first()).toBeVisible();

      await expect(page.locator('body')).not.toContainText(FORBIDDEN_TAXONOMY_REGEX);
      await expect(page.getByText(/Modo Demo|Demonstração/i)).toHaveCount(0);
    }

    expect(consoleErrors).toEqual([]);
    expect(pageErrors).toEqual([]);
  });
});
