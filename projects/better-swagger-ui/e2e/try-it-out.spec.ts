import { expect, test } from '@playwright/test'
import {
  executeTryItOut,
  expandOperation,
  loadSpec,
  mockApi,
  openTryItOut,
  operationLocator,
  specUrl,
} from './helpers'

test.describe('try it out', () => {
  test.beforeEach(async ({ page }) => {
    await loadSpec(page, specUrl('params-full.json'))
  })

  test('prefills parameters from examples', async ({ page }) => {
    await expandOperation(page, 'get:/users/{id}')
    await openTryItOut(page, 'get:/users/{id}')
    await expect(operationLocator(page, 'get:/users/{id}').locator('input').first()).toHaveValue(
      '550e8400-e29b-41d4-a716-446655440000',
    )
  })

  test('renders Solid-compatible parameter defaults and placeholders', async ({ page }) => {
    await loadSpec(page, specUrl('placeholders.json'))
    await expandOperation(page, 'get:/search')
    await openTryItOut(page, 'get:/search')
    const op = operationLocator(page, 'get:/search')

    await expect(op.locator('input[type="number"]')).toHaveValue('1')
    await expect(op.getByPlaceholder('value1, value2')).toBeVisible()
    await expect(op.getByPlaceholder('{"key":"value"}')).toBeVisible()
    await expect(op.getByPlaceholder('status')).toBeVisible()
  })

  test('keeps focus while typing parameter fields', async ({ page }) => {
    await expandOperation(page, 'get:/users/{id}')
    await openTryItOut(page, 'get:/users/{id}')
    const input = operationLocator(page, 'get:/users/{id}').locator('input').first()

    await input.fill('')
    await input.click()
    await input.type('a')
    await expect(input).toHaveValue('a')
    await expect(input).toBeFocused()

    await input.type('b')
    await expect(input).toHaveValue('ab')
    await expect(input).toBeFocused()
  })

  test('blocks execute when required param is empty', async ({ page }) => {
    await expandOperation(page, 'get:/users/{id}')
    await openTryItOut(page, 'get:/users/{id}')
    await operationLocator(page, 'get:/users/{id}').locator('input').first().fill('')
    await executeTryItOut(page, 'get:/users/{id}')
    await expect(page.getByText('Fix parameter validation errors')).toBeVisible()
  })

  test('shows validation error for invalid uuid', async ({ page }) => {
    await expandOperation(page, 'get:/users/{id}')
    await openTryItOut(page, 'get:/users/{id}')
    await operationLocator(page, 'get:/users/{id}').locator('input').first().fill('not-uuid')
    await operationLocator(page, 'get:/users/{id}').locator('input').first().blur()
    await expect(page.getByText(/valid UUID/i)).toBeVisible()
  })

  test('executes request with correct URL, headers, and Accept', async ({ page }) => {
    let capturedUrl = ''
    let capturedHeaders: Record<string, string> = {}

    await mockApi(page, async (route) => {
      capturedUrl = route.request().url()
      capturedHeaders = route.request().headers()
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await expandOperation(page, 'get:/users/{id}')
    await openTryItOut(page, 'get:/users/{id}')
    const op = operationLocator(page, 'get:/users/{id}')
    await op.getByPlaceholder('user@example.com').fill('user@example.com')
    await op.getByPlaceholder('req-123').fill('req-abc')
    await executeTryItOut(page, 'get:/users/{id}')

    await expect(op.getByTestId('response-status')).toContainText('200', { timeout: 10_000 })
    expect(capturedUrl).toContain('/fixtures/mock-api/users/')
    expect(capturedUrl).toContain('email=user%40example.com')
    expect(capturedHeaders['x-request-id']).toBe('req-abc')
    expect(capturedHeaders.accept).toContain('application/json')
  })

  test('rejects invalid JSON body before fetch', async ({ page }) => {
    await loadSpec(page, specUrl('request-body.json'))
    await expandOperation(page, 'post:/items')
    await openTryItOut(page, 'post:/items')
    const op = operationLocator(page, 'post:/items')
    await op.locator('textarea').fill('{ invalid')
    await op.locator('textarea').blur()
    await executeTryItOut(page, 'post:/items')
    await expect(op.getByText(/Fix request body validation errors/i)).toBeVisible()
  })

  test('sends JSON body on POST', async ({ page }) => {
    let body = ''
    await mockApi(page, async (route) => {
      body = route.request().postData() ?? ''
      await route.fulfill({
        status: 201,
        contentType: 'application/json',
        body: JSON.stringify({ id: 1 }),
      })
    })

    await loadSpec(page, specUrl('request-body.json'))
    await expandOperation(page, 'post:/items')
    await openTryItOut(page, 'post:/items')
    await executeTryItOut(page, 'post:/items')

    await expect(operationLocator(page, 'post:/items').getByTestId('response-status')).toContainText(
      '201',
      { timeout: 10_000 },
    )
    expect(body).toContain('"name"')
    expect(JSON.parse(body)).toMatchObject({ name: 'Widget' })
  })

  test('cancel resets try it out state', async ({ page }) => {
    await expandOperation(page, 'get:/users/{id}')
    await openTryItOut(page, 'get:/users/{id}')
    await operationLocator(page, 'get:/users/{id}').locator('input').first().fill('')
    await operationLocator(page, 'get:/users/{id}').getByTestId('cancel-try-it-out').click()
    await expect(operationLocator(page, 'get:/users/{id}').getByTestId('try-it-out')).toBeVisible()
  })

  test('sends text/plain body', async ({ page }) => {
    let body = ''
    let contentType = ''

    await mockApi(page, async (route) => {
      body = route.request().postData() ?? ''
      contentType = route.request().headers()['content-type'] ?? ''
      await route.fulfill({
        status: 200,
        contentType: 'text/plain',
        body: 'ok',
      })
    })

    await loadSpec(page, specUrl('request-body-media.json'))
    await expandOperation(page, 'post:/notes')
    await openTryItOut(page, 'post:/notes')
    const op = operationLocator(page, 'post:/notes')
    await op.locator('textarea').fill('Plain note text')
    await executeTryItOut(page, 'post:/notes')

    await expect(op.getByTestId('response-status')).toContainText('200', { timeout: 10_000 })
    expect(contentType).toContain('text/plain')
    expect(body).toBe('Plain note text')
  })

  test('sends binary file upload', async ({ page }) => {
    let body = ''
    let contentType = ''

    await mockApi(page, async (route) => {
      body = route.request().postData() ?? ''
      contentType = route.request().headers()['content-type'] ?? ''
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await loadSpec(page, specUrl('request-body-media.json'))
    await expandOperation(page, 'post:/binary')
    await openTryItOut(page, 'post:/binary')
    const op = operationLocator(page, 'post:/binary')
    await op.getByTestId('request-body-file').setInputFiles({
      name: 'payload.bin',
      mimeType: 'application/octet-stream',
      buffer: Buffer.from('binary-data'),
    })
    await executeTryItOut(page, 'post:/binary')

    await expect(op.getByTestId('response-status')).toContainText('200', { timeout: 10_000 })
    expect(contentType).toContain('application/octet-stream')
    expect(body).toBe('binary-data')
  })

  test('sends multipart form upload', async ({ page }) => {
    let body = ''
    let contentType = ''

    await mockApi(page, async (route) => {
      body = route.request().postData() ?? ''
      contentType = route.request().headers()['content-type'] ?? ''
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await loadSpec(page, specUrl('request-body-media.json'))
    await expandOperation(page, 'post:/upload')
    await openTryItOut(page, 'post:/upload')
    const op = operationLocator(page, 'post:/upload')
    await op.getByTestId('request-body-multipart-file').setInputFiles({
      name: 'photo.png',
      mimeType: 'image/png',
      buffer: Buffer.from('png-bytes'),
    })
    await op.locator('input[type=text]').fill('A profile photo')
    await executeTryItOut(page, 'post:/upload')

    await expect(op.getByTestId('response-status')).toContainText('200', { timeout: 10_000 })
    expect(contentType).toContain('multipart/form-data')
    expect(body).toContain('photo.png')
    expect(body).toContain('A profile photo')
  })

  test('sends urlencoded form body', async ({ page }) => {
    let body = ''
    let contentType = ''

    await mockApi(page, async (route) => {
      body = route.request().postData() ?? ''
      contentType = route.request().headers()['content-type'] ?? ''
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await loadSpec(page, specUrl('request-body-media.json'))
    await expandOperation(page, 'post:/form')
    await openTryItOut(page, 'post:/form')
    const op = operationLocator(page, 'post:/form')
    await op.getByTestId('request-body-field-username').fill('demo')
    await op.getByTestId('request-body-field-password').fill('secret')
    await executeTryItOut(page, 'post:/form')

    await expect(op.getByTestId('response-status')).toContainText('200', { timeout: 10_000 })
    expect(contentType).toContain('application/x-www-form-urlencoded')
    expect(body).toBe('username=demo&password=secret')
  })

  test('serializes deepObject and collapsed array query params', async ({ page }) => {
    let capturedUrl = ''

    await mockApi(page, async (route) => {
      capturedUrl = route.request().url()
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true }),
      })
    })

    await loadSpec(page, specUrl('params-serialization.json'))
    await expandOperation(page, 'get:/search')
    await openTryItOut(page, 'get:/search')
    const op = operationLocator(page, 'get:/search')
    await op.getByPlaceholder('a,b,c').fill('x,y')
    await op.locator('textarea').fill('{"role":"admin","status":"active"}')
    await executeTryItOut(page, 'get:/search')

    await expect(op.getByTestId('response-status')).toContainText('200', { timeout: 10_000 })
    expect(capturedUrl).toContain('tags=x%2Cy')
    expect(capturedUrl).toContain('filter%5Brole%5D=admin')
    expect(capturedUrl).toContain('filter%5Bstatus%5D=active')
  })
})
