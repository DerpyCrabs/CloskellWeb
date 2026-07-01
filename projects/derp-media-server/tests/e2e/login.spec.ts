import { test, expect } from '@playwright/test'

const PASSCODE_TOKEN = 'test-passcode-share-token1'

test.describe('Login & Auth', () => {
  test('unauthenticated visit to / redirects to /login', async ({ page }) => {
    await page.goto('/')
    await expect(page).toHaveURL(/\/login/)
  })

  test('login page shows password input and sign-in button', async ({ page }) => {
    await page.goto('/login')
    await expect(page.locator('input[type="password"]')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Sign in' })).toBeVisible()
    await expect(page.getByText('Media Server')).toBeVisible()
  })

  test('login page exposes theme settings control', async ({ page }) => {
    await page.goto('/login')
    await expect(page.getByRole('button', { name: 'Open theme settings' })).toBeVisible()
  })

  test('wrong password shows error message', async ({ page }) => {
    await page.goto('/login')
    await page.locator('input[type="password"]').fill('wrong-password')
    await page.getByRole('button', { name: 'Sign in' }).click()
    await expect(page.locator('.text-destructive')).toBeVisible()
  })

  test('correct password redirects to /', async ({ page }) => {
    await page.goto('/login')
    await page.locator('input[type="password"]').fill('test-password')
    await page.getByRole('button', { name: 'Sign in' }).click()
    await expect(page).toHaveURL(/\/(\?|$)/)
  })

  test('share pages remain accessible without login', async ({ page }) => {
    await page.goto(`/share/${PASSCODE_TOKEN}`)
    await expect(page).not.toHaveURL(/\/login/)
    await expect(page.getByText('Protected Share')).toBeVisible()
  })

  test('/share/ without a token shows main file browser like home', async ({ page }) => {
    await page.goto('/share/')
    await expect(page).toHaveURL(/\/share\/$/)
    await expect(page.getByTestId('file-browser')).toBeVisible()
  })

  test('/login/* paths are not treated as the login page', async ({ page }) => {
    await page.goto('/login/extra')
    await expect(page).toHaveURL(/\/login\/extra/)
    await expect(page.getByTestId('file-browser')).toBeVisible()
    await expect(page.locator('input[type="password"]')).toHaveCount(0)
  })

  test('invalid share token shows share not found', async ({ page }) => {
    await page.goto('/share/00000000-0000-4000-8000-000000000099')
    await expect(page.getByTestId('share-not-found')).toBeVisible()
    await expect(page.getByText('Share Not Found')).toBeVisible()
  })
})
