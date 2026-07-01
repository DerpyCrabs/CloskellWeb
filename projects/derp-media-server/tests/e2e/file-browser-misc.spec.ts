import { test, expect } from '@playwright/test'
import fs from 'fs'
import path from 'path'

const batchId = process.env.BATCH_ID
const mediaDir = batchId ? `test-media-${batchId}` : 'test-media'
const UPLOAD_DIR = 'SharedContent'

test.describe('File browser misc', () => {
  test('uploads a folder via upload menu', async ({ page }) => {
    await page.goto(`/?dir=${UPLOAD_DIR}`)
    const tmpRoot = fs.mkdtempSync(path.join(mediaDir, 'upload-dir-'))
    fs.writeFileSync(path.join(tmpRoot, 'folder-root-file.txt'), 'folder upload content')
    try {
      const fileChooserPromise = page.waitForEvent('filechooser')
      await page.locator('button[title="Upload"]').click()
      await page.getByText('Upload folder').click()
      const fileChooser = await fileChooserPromise
      await fileChooser.setFiles(tmpRoot)

      await expect(page.getByText('Upload complete')).toBeVisible({ timeout: 15_000 })
      await expect(page.locator('table').getByText('folder-root-file.txt')).toBeVisible()
    } finally {
      fs.rmSync(tmpRoot, { recursive: true, force: true })
    }
  })

  test('shows drop overlay when dragging files over listing', async ({ page, browserName }) => {
    test.skip(browserName !== 'chromium', 'DataTransfer simulation is validated on Chromium')
    await page.goto(`/?dir=${UPLOAD_DIR}`)
    await page.evaluate(() => {
      const zone = document.querySelector('[data-testid="upload-drop-zone"]')
      if (!zone) throw new Error('missing upload-drop-zone')
      const dt = new DataTransfer()
      dt.items.add(new File(['x'], 'ext-drop.txt', { type: 'text/plain' }))
      zone.dispatchEvent(
        new DragEvent('dragenter', { bubbles: true, cancelable: true, dataTransfer: dt }),
      )
    })
    await expect(page.getByText('Drop files to upload')).toBeVisible()
  })

  test('persists dark mode after reload', async ({ page }) => {
    await page.goto('/')
    await page.getByRole('button', { name: 'Open theme settings' }).click()
    await page.getByRole('menuitem', { name: 'Dark' }).click()
    await expect(page.locator('html')).toHaveAttribute('data-theme', /dark/)
    await page.reload()
    await expect(page.locator('html')).toHaveAttribute('data-theme', /dark/)
  })

  test('dismisses share dialog with Escape', async ({ page }) => {
    await page.goto('/?dir=Documents')
    await page.locator('table tr').filter({ hasText: 'readme.txt' }).click({ button: 'right' })
    await page
      .locator('[data-slot="context-menu-item"]')
      .getByText(/Share|Manage Share/)
      .click()
    await expect(page.getByRole('heading', { name: 'Share Links' })).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(page.getByRole('heading', { name: 'Share Links' })).not.toBeVisible()
  })
})

test.describe('File browser clipboard paste', () => {
  test.use({ permissions: ['clipboard-read', 'clipboard-write'] })

  test('opens paste dialog for text clipboard', async ({ page }) => {
    await page.goto(`/?dir=${UPLOAD_DIR}`)
    await page.getByTestId('file-browser').focus()
    await page.evaluate(async () => {
      await navigator.clipboard.write([
        new ClipboardItem({
          'text/plain': new Blob(['paste dialog e2e'], { type: 'text/plain' }),
        }),
      ])
    })
    await page.keyboard.press('Control+v')
    await expect(page.getByRole('heading', { name: /Paste Text/i })).toBeVisible()
    await page.getByRole('button', { name: 'Paste' }).click()
    await expect(page.locator('textarea').first()).toBeVisible({ timeout: 10_000 })
    await expect(page.locator('textarea').first()).toHaveValue('paste dialog e2e')
  })
})
