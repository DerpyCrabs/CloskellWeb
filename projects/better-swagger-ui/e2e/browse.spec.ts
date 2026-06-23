import { expect, test } from '@playwright/test'
import { expandOperation, loadSpec, specUrl } from './helpers'

test.describe('browse API documentation', () => {
  test.beforeEach(async ({ page }) => {
    await loadSpec(page, specUrl('multi-tag.json'))
  })

  test('renders API metadata and markdown description', async ({ page }) => {
    await expect(page.getByTestId('api-title')).toHaveText('Multi Tag API')
    await expect(page.getByText('v2.0.0')).toBeVisible()
    await expect(page.getByText('multiple')).toBeVisible()
    await expect(page.getByText('/fixtures/mock-api')).toBeVisible()
    await expect(page.getByTestId('download-schema')).toBeVisible()
    await expect(page.getByTestId('open-schema')).toBeVisible()
  })

  test('collapses and expands tag sections', async ({ page }) => {
    const tagToggle = page.getByTestId('tag-section-alpha').locator(':scope > button').first()
    await tagToggle.click()
    await expect(page.getByTestId('operation-get:/alpha/a')).toBeVisible()
    await tagToggle.click()
    await expect(page.getByTestId('operation-get:/alpha/a')).not.toBeVisible()
    await tagToggle.click()
    await expect(page.getByTestId('operation-get:/alpha/a')).toBeVisible()
  })

  test('shows operation method, path, summary, and lock icon', async ({ page }) => {
    await page.getByTestId('tag-section-alpha').getByRole('button').click()
    const op = page.getByTestId('operation-get:/alpha/a')
    const toggle = op.getByRole('button').first()
    await expect(toggle).toContainText('get')
    await expect(toggle).toContainText('/alpha/a')
    await expect(toggle).toContainText('Alpha A')
    await expect(op.getByTestId('operation-authorize-lock')).toBeVisible()
  })

  test('operation lock opens authorize dialog without expanding operation', async ({ page }) => {
    await page.getByTestId('tag-section-beta').getByRole('button').click()
    const op = page.getByTestId('operation-get:/beta/x')
    await op.getByTestId('operation-authorize-lock').click()
    await expect(page.getByTestId('authorize-dialog')).toBeVisible()
    await page.getByTestId('authorize-dialog').getByText('Close', { exact: true }).click()
    await expect(op.getByTestId('operation-authorize-lock')).toBeVisible()
    await expect(op.getByTestId('cancel-try-it-out')).not.toBeVisible()
  })

  test('operation lock expands and activates try it out after authorize', async ({ page }) => {
    await page.getByTestId('tag-section-beta').getByRole('button').click()
    const op = page.getByTestId('operation-get:/beta/x')
    await op.getByTestId('operation-authorize-lock').click()
    await page.getByPlaceholder('X-API-Key').fill('beta-key')
    await page.getByTestId('ApiKeyAuth-authorize').click()
    await expect(page.getByTestId('authorize-dialog')).not.toBeVisible()
    await expect(op.getByTestId('cancel-try-it-out')).toBeVisible()
    await expect(op.getByTestId('try-it-out')).not.toBeVisible()
  })

  test('copies request and response schemas for AI mocks', async ({ page, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write'])
    await loadSpec(page, specUrl('request-body.json'))
    await expandOperation(page, 'post:/items')

    await page.getByTestId('copy-request-body-schema').click()
    const bodySchema = JSON.parse(await page.evaluate(() => navigator.clipboard.readText()))
    expect(bodySchema.type).toBe('object')
    expect(bodySchema.properties.name.type).toBe('string')

    await page.getByTestId('copy-response-schema-201').click()
    const responseSchema = JSON.parse(await page.evaluate(() => navigator.clipboard.readText()))
    expect(responseSchema.type).toBe('object')
  })

  test('expands schema composition with refs and allOf', async ({ page }) => {
    await loadSpec(page, specUrl('schemas-composition.json'))
    await expandOperation(page, 'get:/pets/{id}')
    await expect(page.getByText('Responses', { exact: true })).toBeVisible()
    await expect(page.getByText('200', { exact: true })).toBeVisible()
  })

  test('shows no request body schema section for $ref requestBody (regression)', async ({ page }) => {
    await loadSpec(page, specUrl('refs-limits.json'))
    await expandOperation(page, 'post:/items')
    await expect(page.getByText('Create with ref body')).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Request body' })).not.toBeVisible()
  })
})
