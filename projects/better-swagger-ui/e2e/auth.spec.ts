import { expect, test } from '@playwright/test'
import {
  clearAuthStorage,
  executeTryItOut,
  expandOperation,
  FIXTURE_PATH,
  loadSpec,
  mockApi,
  openTryItOut,
  operationLocator,
  specUrl,
} from './helpers'

test.describe('authorization', () => {
  test.beforeEach(async ({ page }) => {
    await clearAuthStorage(page)
    await loadSpec(page, specUrl('security-schemes.json'))
  })

  test('sends API key header after authorize', async ({ page }) => {
    let headers: Record<string, string> = {}

    await mockApi(page, async (route) => {
      headers = route.request().headers()
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await page.getByTestId('authorize-button').click()
    await expect(page.getByTestId('authorize-dialog')).toBeVisible()
    await page.getByPlaceholder('X-API-Key').first().fill('secret-key')
    await page.getByTestId('ApiKeyAuth-authorize').click()

    await expandOperation(page, 'get:/secure')
    await openTryItOut(page, 'get:/secure')
    await executeTryItOut(page, 'get:/secure')

    await expect(operationLocator(page, 'get:/secure').getByTestId('response-status')).toContainText(
      '200',
      { timeout: 10_000 },
    )
    expect(headers['x-api-key']).toBe('secret-key')
  })

  test('sends Bearer token after authorize', async ({ page }) => {
    let headers: Record<string, string> = {}

    await mockApi(page, async (route) => {
      headers = route.request().headers()
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await page.getByTestId('authorize-button').click()
    await page.getByPlaceholder('Bearer token').fill('my-bearer-token')
    await page.getByTestId('BearerAuth-authorize').click()

    await expandOperation(page, 'get:/secure')
    await openTryItOut(page, 'get:/secure')
    await executeTryItOut(page, 'get:/secure')

    await expect(operationLocator(page, 'get:/secure').getByTestId('response-status')).toContainText(
      '200',
      { timeout: 10_000 },
    )
    expect(headers.authorization).toBe('Bearer my-bearer-token')
  })

  test('keeps focus while typing authorization fields', async ({ page }) => {
    await page.getByTestId('authorize-button').click()
    const tokenInput = page.getByPlaceholder('Bearer token')

    await tokenInput.click()
    await tokenInput.type('a')
    await expect(tokenInput).toHaveValue('a')
    await expect(tokenInput).toBeFocused()

    await tokenInput.type('b')
    await expect(tokenInput).toHaveValue('ab')
    await expect(tokenInput).toBeFocused()
  })

  test('OAuth password flow stores token for execute', async ({ page }) => {
    await page.route('**/fixtures/mock-api/oauth/token', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ access_token: 'oauth-access', expires_in: 3600 }),
      })
    })

    let headers: Record<string, string> = {}
    await page.route('**/fixtures/mock-api/secure', async (route) => {
      headers = route.request().headers()
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await page.getByTestId('authorize-button').click()
    await page.locator('input[name="username"]').fill('user')
    await page.locator('input[name="password"]').fill('pass')
    await page.locator('input[name="client_id"]').fill('test-client')
    await page.getByTestId('OAuthPassword-authorize').click()
    await expect(page.getByTestId('authorize-button')).toContainText('Authorized', {
      timeout: 10_000,
    })

    await expandOperation(page, 'get:/secure')
    await openTryItOut(page, 'get:/secure')
    await executeTryItOut(page, 'get:/secure')

    await expect(operationLocator(page, 'get:/secure').getByTestId('response-status')).toContainText(
      '200',
      { timeout: 15_000 },
    )
    expect(headers.authorization).toBe('Bearer oauth-access')
  })

  test('sends HTTP Basic credentials after authorize', async ({ page }) => {
    let headers: Record<string, string> = {}

    await mockApi(page, async (route) => {
      headers = route.request().headers()
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await page.getByTestId('authorize-button').click()
    await page.locator('input[name="basic_username"]').fill('alice')
    await page.locator('input[name="basic_password"]').fill('secret')
    await page.getByTestId('BasicAuth-authorize').click()

    await expandOperation(page, 'get:/secure')
    await openTryItOut(page, 'get:/secure')
    await executeTryItOut(page, 'get:/secure')

    await expect(operationLocator(page, 'get:/secure').getByTestId('response-status')).toContainText(
      '200',
      { timeout: 10_000 },
    )
    expect(headers.authorization).toBe(`Basic ${Buffer.from('alice:secret').toString('base64')}`)
  })

  test('sends query and cookie api keys after authorize', async ({ page }) => {
    let requestUrl = ''
    let headers: Record<string, string> = {}

    await mockApi(page, async (route) => {
      requestUrl = route.request().url()
      headers = route.request().headers()
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await page.getByTestId('authorize-button').click()
    await page.getByPlaceholder('api_key').fill('query-secret')
    await page.getByTestId('QueryApiKey-authorize').click()
    await page.getByTestId('authorize-button').click()
    await page.getByPlaceholder('session').fill('cookie-secret')
    await page.getByTestId('CookieApiKey-authorize').click()

    await expandOperation(page, 'get:/public')
    await openTryItOut(page, 'get:/public')
    await executeTryItOut(page, 'get:/public')

    await expect(operationLocator(page, 'get:/public').getByTestId('response-status')).toContainText(
      '200',
      { timeout: 10_000 },
    )
    expect(requestUrl).toContain('api_key=query-secret')
    expect(headers.cookie).toContain('session=cookie-secret')
  })

  test('logout clears authorization', async ({ page }) => {
    let headers: Record<string, string> = {}
    await mockApi(page, async (route) => {
      headers = route.request().headers()
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await page.getByTestId('authorize-button').click()
    await page.getByPlaceholder('Bearer token').fill('token')
    await page.getByTestId('BearerAuth-authorize').click()
    await expect(page.getByTestId('authorize-button')).toContainText('Authorized', {
      timeout: 5_000,
    })

    await page.getByTestId('authorize-button').click()
    await expect(page.getByTestId('authorize-dialog')).toBeVisible()
    await page.getByRole('button', { name: 'Logout' }).first().click()
    await page.getByTestId('authorize-dialog').getByText('Close', { exact: true }).click()
    await expect(page.getByTestId('authorize-button')).toContainText('Authorize')

    await expandOperation(page, 'get:/secure')
    await openTryItOut(page, 'get:/secure')
    await executeTryItOut(page, 'get:/secure')
    await expect(operationLocator(page, 'get:/secure').getByTestId('response-status')).toContainText(
      '200',
      { timeout: 10_000 },
    )
    expect(headers.authorization).toBeUndefined()
  })

  test('persists authorization for the loaded source URL', async ({ page }) => {
    await page.getByTestId('authorize-button').click()
    await page.getByPlaceholder('Bearer token').fill('persisted-token')
    await page.getByTestId('BearerAuth-authorize').click()
    await expect(page.getByTestId('authorize-button')).toContainText('Authorized', {
      timeout: 5_000,
    })

    await page.reload()
    await page.getByTestId('api-title').waitFor({ timeout: 15_000 })
    await expect(page.getByTestId('authorize-button')).toContainText('Authorized', {
      timeout: 5_000,
    })
  })

  test('shows OAuth authorize loader while token request is pending', async ({ page }) => {
    await page.route('**/fixtures/mock-api/oauth/token', async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 750))
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ access_token: 'oauth-access', expires_in: 3600 }),
      })
    })

    await page.getByTestId('authorize-button').click()
    await page.locator('input[name="username"]').fill('user')
    await page.locator('input[name="password"]').fill('pass')
    await page.locator('input[name="client_id"]').fill('test-client')

    const authorize = page.getByTestId('OAuthPassword-authorize')
    await authorize.click()
    await expect(authorize).toBeDisabled()
    await expect(authorize).toContainText('Authorizing…')
    await expect(authorize.locator('svg.lucide-loader-circle.animate-spin')).toBeVisible()
  })

  test('renders only OAuth scheme from Swagger UI initOAuth config', async ({ page }) => {
    const initResponsePromise = page.waitForResponse((response) =>
      response.url().includes('/fixtures/oauth-swagger-ui/swagger-ui/swagger-initializer.js'),
    )
    await loadSpec(page, `${FIXTURE_PATH}/oauth-swagger-ui/swagger-ui/index.html`)
    await expect((await initResponsePromise).ok()).toBe(true)

    await page.getByTestId('authorize-button').click()
    const dialog = page.getByTestId('authorize-dialog')
    await expect(dialog).toBeVisible()
    await expect(dialog.locator('h3')).toHaveCount(1)
    await expect(dialog.getByRole('heading', { name: 'oauth2 (OAuth2, password)' })).toBeVisible()
    await expect(dialog.getByText('ApiKeyAuth')).toHaveCount(0)
    await expect(dialog.getByText('BearerAuth')).toHaveCount(0)
    await expect(dialog.locator('input[name="client_id"]')).toHaveValue('sp-gate')
    await expect(dialog.locator('input[name="client_secret"]')).toHaveValue('top-secret')
  })
})
