import { test, expect } from '@playwright/test'

const PASSCODE_TOKEN = 'test-passcode-share-token1'
const CORRECT_PASSCODE = 'secret123'

test.describe('Passcode-Protected Shares', () => {
  test('shows passcode gate for protected share', async ({ page }) => {
    await page.goto(`/share/${PASSCODE_TOKEN}`)
    await expect(page.getByText('Protected Share')).toBeVisible()
    await expect(page.locator('input[placeholder="Enter passcode"]')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Access Share' })).toBeVisible()
  })

  test('rejects wrong passcode', async ({ page }) => {
    await page.goto(`/share/${PASSCODE_TOKEN}`)
    await page.locator('input[placeholder="Enter passcode"]').fill('wrong')
    await page.getByRole('button', { name: 'Access Share' }).click()
    await expect(page.locator('.text-destructive')).toBeVisible()
  })

  test('accepts correct passcode and shows content', async ({ page }) => {
    await page.goto(`/share/${PASSCODE_TOKEN}`)
    await page.locator('input[placeholder="Enter passcode"]').fill(CORRECT_PASSCODE)
    await page.getByRole('button', { name: 'Access Share' }).click()

    await expect(page.getByText('public-doc.txt')).toBeVisible()
  })

  test('passcode in URL auto-authenticates', async ({ page }) => {
    await page.goto(`/share/${PASSCODE_TOKEN}?p=${CORRECT_PASSCODE}`)
    await expect(page.getByText('public-doc.txt')).toBeVisible()
  })
})
