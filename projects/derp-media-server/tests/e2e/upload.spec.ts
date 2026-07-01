import { test, expect } from '@playwright/test'
import path from 'path'
import fs from 'fs'

const batchId = process.env.BATCH_ID
const mediaDir = batchId ? `test-media-${batchId}` : 'test-media'
const UPLOAD_DIR = 'SharedContent'

test.describe('File Upload', () => {
  test('upload button is visible in editable folders', async ({ page }) => {
    await page.goto(`/?dir=${UPLOAD_DIR}`)
    await expect(page.locator('button[title="Upload"]')).toBeVisible()
  })

  test('upload button is hidden in non-editable folders', async ({ page }) => {
    await page.goto('/?dir=Documents')
    await expect(page.locator('button[title="Upload"]')).not.toBeVisible()
  })

  test('upload menu shows file and folder options', async ({ page }) => {
    await page.goto(`/?dir=${UPLOAD_DIR}`)
    await page.locator('button[title="Upload"]').click()
    await expect(page.getByText('Upload files')).toBeVisible()
    await expect(page.getByText('Upload folder')).toBeVisible()
  })

  test('uploads a file via file picker', async ({ page }) => {
    await page.goto(`/?dir=${UPLOAD_DIR}`)

    const tmpFile = path.resolve(mediaDir, 'upload-test-file.txt')
    fs.writeFileSync(tmpFile, 'uploaded content for testing')

    try {
      const fileChooserPromise = page.waitForEvent('filechooser')
      await page.locator('button[title="Upload"]').click()
      await page.getByText('Upload files').click()
      const fileChooser = await fileChooserPromise
      await fileChooser.setFiles(tmpFile)

      await expect(page.getByText('Upload complete')).toBeVisible({ timeout: 15_000 })
      await expect(page.locator('table').getByText('upload-test-file.txt')).toBeVisible()
    } finally {
      fs.rmSync(tmpFile, { force: true })
    }
  })

  test('uploaded file appears in file list', async ({ page }) => {
    await page.goto(`/?dir=${UPLOAD_DIR}`)
    await expect(page.locator('table').getByText('upload-test-file.txt')).toBeVisible()

    // Cleanup: delete the uploaded file via context menu
    await page
      .locator('table tr')
      .filter({ hasText: 'upload-test-file.txt' })
      .click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Delete').click()
    const confirmBtn = page.getByRole('alertdialog').getByRole('button', { name: /Delete/i })
    await confirmBtn.waitFor({ state: 'visible' })
    await confirmBtn.click()
    await expect(page.locator('table').getByText('upload-test-file.txt')).not.toBeVisible()
  })

  test('shows uploading progress toast', async ({ page }) => {
    await page.goto(`/?dir=${UPLOAD_DIR}`)

    const tmpFile = path.resolve(mediaDir, 'progress-test.txt')
    fs.writeFileSync(tmpFile, 'testing progress indicator')

    try {
      const fileChooserPromise = page.waitForEvent('filechooser')
      await page.locator('button[title="Upload"]').click()
      await page.getByText('Upload files').click()
      const fileChooser = await fileChooserPromise
      await fileChooser.setFiles(tmpFile)

      // Should see either the uploading state or the success state
      await expect(page.getByText('Uploading').or(page.getByText('Upload complete'))).toBeVisible({
        timeout: 15_000,
      })

      await expect(page.locator('table').getByText('progress-test.txt')).toBeVisible({
        timeout: 15_000,
      })

      // Cleanup
      await page
        .locator('table tr')
        .filter({ hasText: 'progress-test.txt' })
        .click({ button: 'right' })
      await page.locator('[data-slot="context-menu-item"]').getByText('Delete').click()
      await page.getByRole('button', { name: /Delete/i }).click()
    } finally {
      fs.rmSync(tmpFile, { force: true })
    }
  })
})
