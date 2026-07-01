import { test, expect, type Page } from '@playwright/test'
import path from 'path'
import fs from 'fs'

const sessionFile = process.env.BATCH_ID ? `session-${process.env.BATCH_ID}.json` : 'session.json'
const authStoragePath = path.resolve(__dirname, '../fixtures/.auth', sessionFile)
const batchId = process.env.BATCH_ID
const mediaDir = batchId ? `test-media-${batchId}` : 'test-media'

function uniqueId() {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

async function createEditableShare(page: Page): Promise<string> {
  const res = await page.request.post('/api/shares', {
    data: {
      path: 'SharedContent',
      isDirectory: true,
      editable: true,
      restrictions: { allowUpload: true, allowEdit: true, allowDelete: true },
    },
  })
  const json = await res.json()
  const base = `/share/${json.share.token}`
  return json.share.passcode ? `${base}?p=${encodeURIComponent(json.share.passcode)}` : base
}

function getShareToken(shareUrl: string): string {
  return new URL(shareUrl, 'http://localhost').pathname.split('/')[2]
}

test.describe('Share folder browser parity', () => {
  test('renames a file from context menu on standalone share', async ({ browser }) => {
    const context = await browser.newContext({ storageState: authStoragePath })
    const page = await context.newPage()
    const shareUrl = await createEditableShare(page)
    const id = uniqueId()
    const oldName = `share-br-ren-${id}.txt`
    const newName = `share-br-ren-${id}-done.txt`
    await page.close()
    await context.close()

    const ctx2 = await browser.newContext()
    const p = await ctx2.newPage()
    await p.goto(shareUrl)
    await expect(p.getByText('public-doc.txt')).toBeVisible()

    await p.locator('button[title="Create new file"]').click()
    await p.locator('[role="dialog"]').getByRole('textbox').fill(oldName)
    await p.locator('[role="dialog"]').getByRole('button', { name: 'Create', exact: true }).click()
    await expect(p.locator('table').getByText(oldName)).toBeVisible()

    await p.locator('table tr').filter({ hasText: oldName }).click({ button: 'right' })
    await p.locator('[data-slot="context-menu-item"]').getByText('Rename').click()
    await p.locator('[role="dialog"]').getByPlaceholder('New name').fill(newName)
    await p
      .locator('[role="dialog"]')
      .getByRole('button', { name: /^Rename$/i })
      .click()
    await expect(p.locator('table').getByText(newName)).toBeVisible()

    const token = getShareToken(shareUrl)
    await p.request.post(`/api/share/${token}/delete`, { data: { path: newName } })
    await ctx2.close()
  })

  test('uploads a file on standalone editable share', async ({ browser }) => {
    const context = await browser.newContext({ storageState: authStoragePath })
    const page = await context.newPage()
    const shareUrl = await createEditableShare(page)
    const id = uniqueId()
    const fileName = `share-br-up-${id}.txt`
    await page.close()
    await context.close()

    const tmpFile = path.resolve(mediaDir, fileName)
    fs.writeFileSync(tmpFile, 'upload parity test')

    const ctx2 = await browser.newContext()
    const p = await ctx2.newPage()
    try {
      await p.goto(shareUrl)
      await expect(p.getByText('public-doc.txt')).toBeVisible()

      const fileChooserPromise = p.waitForEvent('filechooser')
      await p.locator('button[title="Upload"]').click()
      await p.getByText('Upload files').click()
      const fileChooser = await fileChooserPromise
      await fileChooser.setFiles(tmpFile)

      await expect(p.getByText('Upload complete').or(p.getByText('Uploading'))).toBeVisible({
        timeout: 15_000,
      })
      await expect(p.locator('table').getByText(fileName)).toBeVisible({ timeout: 15_000 })

      const token = getShareToken(shareUrl)
      const del = await p.request.post(`/api/share/${token}/delete`, { data: { path: fileName } })
      expect(del.ok()).toBeTruthy()
    } finally {
      fs.rmSync(tmpFile, { force: true })
      await ctx2.close()
    }
  })
})
