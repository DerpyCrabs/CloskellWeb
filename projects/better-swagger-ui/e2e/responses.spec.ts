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

const PNG_BYTES = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
  'base64',
)

test.describe('response handling', () => {
  test.beforeEach(async ({ page }) => {
    await loadSpec(page, specUrl('responses-mixed.json'))
  })

  test('displays JSON response with status, metadata, and copy action', async ({ page, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write'])

    await mockApi(page, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ message: 'hello' }),
      })
    })

    await expandOperation(page, 'get:/json')
    await openTryItOut(page, 'get:/json')
    await executeTryItOut(page, 'get:/json')

    const op = operationLocator(page, 'get:/json')
    await expect(op.getByTestId('response-status')).toContainText('200')
    await expect(op.getByTestId('response-status')).toContainText('OK')
    await expect(op.getByText(/^\d+ ms$/)).toBeVisible()
    await expect(op.locator('[data-testid="response-status"] + span + span')).toContainText('application/json')
    await expect(op.getByTestId('response-body')).toContainText('hello')

    await op.getByTestId('copy-response').click()
    const copied = JSON.parse(await page.evaluate(() => navigator.clipboard.readText()))
    expect(copied).toEqual({ message: 'hello' })
  })

  test('updates virtualized JSON rows between executions', async ({ page }) => {
    let calls = 0

    await mockApi(page, async (route) => {
      calls += 1
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(
          calls === 1
            ? { items: Array.from({ length: 120 }, (_, index) => ({ index })) }
            : { ok: true },
        ),
      })
    })

    await expandOperation(page, 'get:/json')
    await openTryItOut(page, 'get:/json')
    await executeTryItOut(page, 'get:/json')

    const op = operationLocator(page, 'get:/json')
    const body = op.getByTestId('response-body')
    await expect(body).toContainText('"items"')
    await body.locator('.virtual-json-scroll').evaluate((element) => {
      element.scrollTop = element.scrollHeight
    })

    await executeTryItOut(page, 'get:/json')
    await expect(body).toContainText('"ok"')
    await expect(body).not.toContainText('"items"')
  })

  test('shows file download for CSV', async ({ page }) => {
    await mockApi(page, async (route) => {
      await route.fulfill({
        status: 200,
        headers: {
          'Content-Type': 'text/csv',
          'Content-Disposition': 'attachment; filename="report.csv"',
        },
        body: 'a,b\n1,2',
      })
    })

    await expandOperation(page, 'get:/csv')
    await openTryItOut(page, 'get:/csv')
    await executeTryItOut(page, 'get:/csv')

    const op = operationLocator(page, 'get:/csv')
    await expect(op.getByTestId('download-response')).toBeVisible()
    await expect(op.getByText('report.csv')).toBeVisible()
  })

  test('uses encoded filename from content disposition', async ({ page }) => {
    await mockApi(page, async (route) => {
      await route.fulfill({
        status: 200,
        headers: {
          'Content-Type': 'text/csv',
          'Content-Disposition': "attachment; filename*=UTF-8''report%20Q1.csv",
        },
        body: 'a,b\n1,2',
      })
    })

    await expandOperation(page, 'get:/csv')
    await openTryItOut(page, 'get:/csv')
    await executeTryItOut(page, 'get:/csv')

    const op = operationLocator(page, 'get:/csv')
    await expect(op.getByTestId('download-response')).toBeVisible()
    await expect(op.getByText('report Q1.csv')).toBeVisible()
  })

  test('shows download for octet-stream with disposition filename', async ({ page }) => {
    await mockApi(page, async (route) => {
      await route.fulfill({
        status: 200,
        headers: {
          'Content-Type': 'application/octet-stream',
          'Content-Disposition': 'attachment; filename="data.bin"',
        },
        body: 'abc',
      })
    })

    await expandOperation(page, 'get:/file')
    await openTryItOut(page, 'get:/file')
    await executeTryItOut(page, 'get:/file')

    const op = operationLocator(page, 'get:/file')
    await expect(op.getByTestId('response-status')).toContainText('200', { timeout: 10_000 })
    await expect(op.getByTestId('download-response')).toBeVisible()
    await expect(op.getByText('data.bin')).toBeVisible()
  })

  test('renders image preview for png response', async ({ page }) => {
    await mockApi(page, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'image/png',
        body: PNG_BYTES,
      })
    })

    await expandOperation(page, 'get:/image')
    await openTryItOut(page, 'get:/image')
    await executeTryItOut(page, 'get:/image')

    const op = operationLocator(page, 'get:/image')
    await expect(op.getByTestId('response-status')).toContainText('200', { timeout: 10_000 })
    await expect(op.locator('img[src^="data:image/png"], img[src^="blob:"]')).toBeVisible()
  })

  test('displays error status and body', async ({ page }) => {
    await mockApi(page, async (route) => {
      await route.fulfill({
        status: 400,
        contentType: 'application/problem+json',
        body: JSON.stringify({ title: 'Bad Request' }),
      })
    })

    await expandOperation(page, 'get:/error')
    await openTryItOut(page, 'get:/error')
    await executeTryItOut(page, 'get:/error')

    const op = operationLocator(page, 'get:/error')
    await expect(op.getByTestId('response-status')).toContainText('400')
    await expect(op.getByTestId('response-body')).toContainText('Bad Request')
  })
})
