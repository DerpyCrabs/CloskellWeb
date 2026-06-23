import { expect, test } from '@playwright/test'
import { FIXTURE_PATH, loadSpec, specUrl } from './helpers'

test.describe('spec loading', () => {
  test('loads direct OpenAPI URL', async ({ page }) => {
    await loadSpec(page, specUrl('minimal.json'))
    await expect(page.getByTestId('api-title')).toHaveText('Minimal API')
    await page.getByTestId('tag-section-pets').getByRole('button').click()
    await expect(page.getByTestId('operation-get:/pets')).toBeVisible()
  })

  test('loads via ?url= query param on Swagger UI page', async ({ page }) => {
    await loadSpec(
      page,
      `${FIXTURE_PATH}/swagger-ui/query-url.html?url=${encodeURIComponent('/fixtures/openapi/minimal.json')}`,
    )
    await expect(page.getByTestId('api-title')).toHaveText('Minimal API')
  })

  test('loads via embedded spec url in HTML page', async ({ page }) => {
    await loadSpec(page, `${FIXTURE_PATH}/pages/minimal-spec.html`)
    await expect(page.getByTestId('api-title')).toHaveText('Minimal API')
  })

  test('loads via configUrl in HTML page with multiple definitions', async ({ page }) => {
    await loadSpec(page, `${FIXTURE_PATH}/swagger-ui/config-url/index.html`)
    await expect(page.getByTestId('definition-select')).toBeVisible()
    await expect(page.getByTestId('api-title')).toHaveText('Definition A')
  })

  test('loads via swagger-config path discovery', async ({ page }) => {
    await loadSpec(page, `${FIXTURE_PATH}/swagger-ui/swagger-config/index.html`)
    await expect(page.getByTestId('definition-select')).toBeVisible()
    await expect(page.getByTestId('api-title')).toHaveText('Definition A')
  })

  test('switches definition and updates title', async ({ page }) => {
    await loadSpec(page, `${FIXTURE_PATH}/swagger-ui/config-url/index.html`)
    await page.getByTestId('definition-select').selectOption('API B')
    await expect(page.getByTestId('api-title')).toHaveText('Definition B', { timeout: 15_000 })
  })

  test('loads direct OpenAPI YAML URL', async ({ page }) => {
    await loadSpec(page, specUrl('minimal.yaml'))
    await expect(page.getByTestId('api-title')).toHaveText('Minimal API')
  })

  test('loads uploaded YAML file', async ({ page }) => {
    await page.goto('/')
    await page.getByTestId('spec-file-input').setInputFiles('tests/fixtures/openapi/minimal.yaml')
    await expect(page.getByTestId('api-title')).toHaveText('Minimal API', { timeout: 15_000 })
  })

  test('loads pasted YAML content', async ({ page }) => {
    const yaml = `openapi: 3.0.3
info:
  title: Minimal API
  version: 1.0.0
paths:
  /pets:
    get:
      responses:
        '200':
          description: OK`

    await page.goto('/')
    await page.getByTestId('url-input').focus()
    await page.evaluate((content) => {
      const input = document.querySelector('[data-testid="url-input"]') as HTMLInputElement | null
      if (!input) throw new Error('url input not found')

      const clipboard = new DataTransfer()
      clipboard.setData('text/plain', content)
      input.dispatchEvent(new ClipboardEvent('paste', { clipboardData: clipboard, bubbles: true }))
    }, yaml)

    await expect(page.getByTestId('api-title')).toHaveText('Minimal API', { timeout: 15_000 })
  })

  test('shows error when spec cannot be loaded', async ({ page }) => {
    await page.goto('/')
    await page.getByTestId('url-input').fill('http://127.0.0.1:9/openapi.json')
    await page.getByTestId('load-form').evaluate((form) => {
      ;(form as HTMLFormElement).requestSubmit()
    })
    await expect(page.getByText(/Could not fetch|Could not find OpenAPI|Failed to load spec/i)).toBeVisible({
      timeout: 15_000,
    })
  })
})
