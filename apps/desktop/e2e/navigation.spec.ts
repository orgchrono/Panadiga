import { test, expect } from '@playwright/test';
import { installMockIpcBridge } from './fixtures/mockIpcBridge';

test.describe('Panadiga Desktop Navigation & Layout E2E', () => {
  test.beforeEach(async ({ page }) => {
    await installMockIpcBridge(page);
    await page.goto('/');
    await expect(page.locator('h1')).toContainText('DIGA SUITE');
    await expect(page.getByText(/Modo Demo|Demonstração/i)).toHaveCount(0);
  });

  test('loads Dashboard View by default with system metrics', async ({ page }) => {
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'dashboard');
    await expect(page.getByText('Spansion NOR Flash').first()).toBeVisible();
    await expect(page.getByText(/Head Dump/).first()).toBeVisible();
  });

  test('navigates seamlessly across all workbench modules', async ({ page }) => {
    // 1. Flash & NVRAM Module
    await page.click('button[data-testid="nav-nvram"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'nvram');
    await expect(page.getByText(/Cirurgia|Surgical/).first()).toBeVisible();

    // 2. Armazenamento HD Module
    await page.click('button[data-testid="nav-hdd"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'hdd');

    // 3. Bancada PicoBench Module
    await page.click('button[data-testid="nav-picobench"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'picobench');

    // 4. Criptoanálise FRM Module
    await page.click('button[data-testid="nav-frmcrypto"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'frmcrypto');

    // 5. Manuais & Esquemas Module
    await page.click('button[data-testid="nav-manuals"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'manuals');

    // 6. Inspetor Hexadecimal Module
    await page.click('button[data-testid="nav-hexinspector"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'hexinspector');

    // 7. Centro Unbrick & TBC Passthrough Module
    await page.click('button[data-testid="nav-unbrick"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'unbrick');
    await expect(page.getByText('DIGADIAG.FRM vs PANA__SD.FRM').first()).toBeVisible();
  });

  test('builds SD recovery kit from Centro Unbrick', async ({ page }) => {
    await page.click('button[data-testid="nav-unbrick"]');
    await page.click('button[data-testid="sd-kit-submit-btn"]');
    await expect(page.getByText(/sd_card_eh55_kit\/DIGADIAG\.FRM/).first()).toBeVisible();
  });

  test('operates Guided Pipeline Bar, Basic/Advanced mode toggle, Draggable HUD, and Resizable/RangeDnD in HexInspector', async ({ page }) => {
    // 1. Open Draggable HUD Pinout
    await page.getByRole('button', { name: 'HUD Pinout' }).click();
    await expect(page.getByText('HUD P7501 (Arraste livre)')).toBeVisible();
    await page.getByRole('button', { name: 'Fechar HUD Flutuante' }).click();
    await expect(page.getByText('HUD P7501 (Arraste livre)')).toHaveCount(0);

    // 2. Navigate via Pipeline Bar to Hex Inspector and verify ResizableSplitPane + RangeDnD
    await page.click('button[data-testid="nav-hexinspector"]');
    await expect(page.getByRole('separator')).toBeVisible();
    await expect(page.getByText('Scrubber LBA / Offset Físico (RangeDnD)')).toBeVisible();

    // 3. Toggle Basic mode in NVRAM view and verify advanced patcher collapses cleanly
    await page.click('button:has-text("2. Cirurgia NVRAM U99")');
    await expect(page.getByText('Janela de Setores (RangeDnD)')).toBeVisible();
    await page.getByRole('button', { name: 'Básico (Guiado)' }).click();
    await expect(page.getByText(/Modo Básico ativo: o editor hexadecimal Copy-on-Write/i)).toBeVisible();
    await page.getByRole('button', { name: 'Avançado (Engenharia)' }).click();

    // 4. Open Settings & Accessibility view, toggle High Contrast & Colorblind mode, then restore defaults
    await page.click('button[data-testid="nav-settings"]');
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'settings');
    await page.getByRole('button', { name: /Alto Contraste/i }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'high-contrast');
    await page.getByRole('switch', { name: 'Ativar modo daltônico' }).click();
    await expect(page.locator('html')).toHaveAttribute('data-colorblind', 'true');
    await page.getByRole('button', { name: /Restaurar Layout & Padrões/i }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark-lab');
  });

  test('opens MobileSidebarDrawer cleanly on narrow 390x844 mobile viewport', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    const navTrigger = page.getByTestId('header-mobile-nav-btn');
    await expect(navTrigger).toBeVisible();
    await navTrigger.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.locator('[role="dialog"] button[data-testid="nav-settings"]').click();
    await expect(page.locator('[data-testid="app-header"]')).toHaveAttribute('data-view', 'settings');
  });
});
