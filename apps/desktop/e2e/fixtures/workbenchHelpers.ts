import { expect, type Page } from '@playwright/test';

export async function enterEngineeringMode(page: Page): Promise<void> {
  await expect(page.getByTestId('express-shell')).toBeVisible();
  await page.getByTestId('express-to-engineering-btn').click();
  await expect(page.getByTestId('engineering-shell')).toBeVisible();
  await expect(page.locator('h1')).toContainText('DIGA SUITE');
}
