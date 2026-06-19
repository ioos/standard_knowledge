import { expect, test } from '@playwright/test';

// Drives the demo UI end to end: the custom elements (x-app, x-filter-standards,
// x-get-standard) wire the WASM library to the DOM, so these confirm the
// published package works in the actual page consumers will load.

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  // x-app instantiates the WASM library and renders the filter form once the
  // module's top-level `await init()` resolves.
  await expect(page.locator('#varName')).toBeVisible();
});

test('filter shows the prompt before any input', async ({ page }) => {
  await expect(page.locator('#filterResult')).toContainText('Please enter a keyword');
});

test('filter by unit returns matching standards', async ({ page }) => {
  await page.locator('#unit').fill('Pa');
  const result = page.locator('#filterResult');
  await expect(result.locator('ul li')).not.toHaveCount(0);
  await expect(result).toContainText('air_pressure_at_mean_sea_level');
});

test('filter with no matches reports none found', async ({ page }) => {
  await page.locator('#unit').fill('not_a_real_unit');
  await expect(page.locator('#filterResult')).toContainText('No standards found');
});

test('get a known standard renders its card', async ({ page }) => {
  await page.locator('#name').fill('air_pressure_at_mean_sea_level');
  await page.getByRole('button', { name: 'Show Standard' }).click();
  const card = page.locator('x-standard');
  await expect(card).toContainText('air_pressure_at_mean_sea_level');
  await expect(card).toContainText('Pa');
});

test('get an unknown standard shows an error', async ({ page }) => {
  await page.locator('#name').fill('not_a_real_standard');
  await page.getByRole('button', { name: 'Show Standard' }).click();
  await expect(page.locator('#result .alert-danger')).toContainText('Could not find standard');
});
