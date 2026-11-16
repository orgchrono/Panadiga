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
    await expect(page.locator('header')).toContainText('Visão Geral do Gravador');
    await expect(page.getByText('Spansion NOR Flash').first()).toBeVisible();
    await expect(page.getByText(/Head Dump/).first()).toBeVisible();
  });

  test('navigates seamlessly across all workbench modules', async ({ page }) => {
    // 1. Flash & NVRAM Module
    await page.click('button:has-text("Flash & NVRAM")');
    await expect(page.locator('header')).toContainText('Flash Spansion S29JL064H');
    await expect(page.getByText(/Cirurgia|Surgical/).first()).toBeVisible();

    // 2. Armazenamento HD Module
    await page.click('button:has-text("Armazenamento HD")');
    await expect(page.locator('header')).toContainText('Armazenamento MEIHDFS');

    // 3. Bancada PicoBench Module
    await page.click('button:has-text("Bancada PicoBench")');
    await expect(page.locator('header')).toContainText('Bancada Raspberry Pi Pico');

    // 4. Criptoanálise FRM Module
    await page.click('button:has-text("Criptoanálise FRM")');
    await expect(page.locator('header')).toContainText('Criptoanálise Óptica');

    // 5. Manuais & Esquemas Module
    await page.click('button:has-text("Manuais & Esquemas")');
    await expect(page.locator('header')).toContainText('Manuais de Serviço');

    // 6. Inspetor Hexadecimal Module
    await page.click('button:has-text("Inspetor Hexadecimal")');
    await expect(page.locator('header')).toContainText('Inspetor Hexadecimal');

    // 7. Centro Unbrick & TBC Passthrough Module
    await page.click('button:has-text("Centro Unbrick")');
    await expect(page.locator('header')).toContainText('Centro Unbrick');
    await expect(page.getByText('DIGADIAG.FRM vs PANA__SD.FRM').first()).toBeVisible();
  });

  test('builds SD recovery kit from Centro Unbrick', async ({ page }) => {
    await page.click('button:has-text("Centro Unbrick")');
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
    await page.click('button:has-text("Inspetor Hexadecimal")');
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
    await expect(page.locator('header')).toContainText('Configurações, Temas');
    await page.getByRole('button', { name: /Alto Contraste/i }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'high-contrast');
    await page.getByRole('switch', { name: 'Ativar modo daltônico' }).click();
    await expect(page.locator('html')).toHaveAttribute('data-colorblind', 'true');
    await page.getByRole('button', { name: /Restaurar Layout & Padrões/i }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark-lab');
  });

  test('opens MobileSidebarDrawer cleanly on narrow 390x844 mobile viewport', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    const navTrigger = page.getByRole('button', { name: 'Abrir navegação principal' });
    await expect(navTrigger).toBeVisible();
    await navTrigger.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.locator('[role="dialog"] button[data-testid="nav-settings"]').click();
    await expect(page.locator('header')).toContainText('Configurações, Temas');
  });
});
