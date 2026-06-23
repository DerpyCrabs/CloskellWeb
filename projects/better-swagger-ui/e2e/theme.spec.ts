import { expect, test } from '@playwright/test'
import { clearThemeStorage, loadSpec, specUrl } from './helpers'

test.describe('theme', () => {
  test.beforeEach(async ({ page }) => {
    await clearThemeStorage(page)
  })

  test('toggles dark and light theme', async ({ page }) => {
    await loadSpec(page, specUrl('minimal.json'))
    const toggle = page.getByRole('button', { name: /Switch to light theme/i })
    await expect(toggle).toBeVisible()
    await toggle.click()
    await expect(page.locator('html')).not.toHaveClass(/dark/)
    await page.getByRole('button', { name: /Switch to dark theme/i }).click()
    await expect(page.locator('html')).toHaveClass(/dark/)
  })

  test('persists theme after reload', async ({ page }) => {
    await page.goto('/')
    await page.getByRole('button', { name: /Switch to light theme/i }).click()
    await page.reload()
    await expect(page.locator('html')).not.toHaveClass(/dark/)
  })
})
