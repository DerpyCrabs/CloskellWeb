import { randomUUID } from 'node:crypto'
import { test, expect, Page, BrowserContext } from '@playwright/test'
import path from 'path'

const sessionFile = process.env.BATCH_ID ? `session-${process.env.BATCH_ID}.json` : 'session.json'
const authStoragePath = path.resolve(__dirname, '../fixtures/.auth', sessionFile)

async function createAdminContext(
  browser: import('@playwright/test').Browser,
): Promise<BrowserContext> {
  return browser.newContext({ storageState: authStoragePath })
}

async function createShare(
  page: Page,
  body: Record<string, unknown>,
): Promise<{ url: string; token: string }> {
  const res = await page.request.post('/api/shares', { data: body })
  const json = await res.json()
  const base = `/share/${json.share.token}`
  const url = json.share.passcode ? `${base}?p=${encodeURIComponent(json.share.passcode)}` : base
  return { url, token: json.share.token }
}

async function gotoWithSSE(page: Page, url: string) {
  const expectShareStream = url.includes('/share/')
  const streamRequest = page.waitForRequest(
    (r) => {
      const u = r.url()
      if (expectShareStream) return u.includes('/api/share/') && u.includes('/stream')
      return u.includes('/api/events/stream')
    },
    { timeout: 10000 },
  )
  const consoleConnected = page.waitForEvent('console', {
    predicate: (msg) => {
      const text = msg.text()
      return (
        text.includes('[Admin SSE] Connected') ||
        text.includes('[Share SSE] Connected to share stream')
      )
    },
    timeout: 10000,
  })
  await page.goto(url)
  await Promise.race([streamRequest, consoleConnected])
}

async function deleteShare(page: Page, token: string) {
  await page.request.post('/api/shares/delete', { data: { token } })
}

async function deleteFile(page: Page, filePath: string) {
  await page.request.post('/api/files/delete', { data: { path: filePath } })
}

async function createFile(page: Page, filePath: string, content = 'test content') {
  await page.request.post('/api/files/create', {
    data: { type: 'file', path: filePath, content },
  })
}

async function renameFile(page: Page, oldPath: string, newPath: string) {
  await page.request.post('/api/files/rename', {
    data: { oldPath, newPath },
  })
}

async function fillCreateFileDialog(page: Page, baseName: string) {
  await page.locator('button[title="Create new file"]').click()
  const nameInput = page.locator(
    'input[placeholder="notes.txt"], input[placeholder="notes.md"], input[placeholder*="File name"]',
  )
  await expect(nameInput).toBeVisible()
  await nameInput.fill(baseName)
  await page
    .getByRole('dialog', { name: /create.*file/i })
    .getByRole('button', { name: 'Create', exact: true })
    .click()
}

test.describe('SSE Live Updates', () => {
  test('admin renames file in shared folder -> share view updates', async ({ browser }) => {
    const id = randomUUID().slice(0, 10)
    const startName = `sse-rename-${id}.txt`
    const endName = `sse-renamed-${id}.txt`
    const startPath = `SharedContent/${startName}`
    const endPath = `SharedContent/${endName}`

    const adminCtx = await createAdminContext(browser)
    const adminPage = await adminCtx.newPage()

    const { url: shareUrl, token } = await createShare(adminPage, {
      path: 'SharedContent',
      isDirectory: true,
    })

    await createFile(adminPage, startPath)

    const shareCtx = await browser.newContext()
    const sharePage = await shareCtx.newPage()
    await gotoWithSSE(sharePage, shareUrl)
    await expect(sharePage.getByText(startName)).toBeVisible()

    await renameFile(adminPage, startPath, endPath)

    await expect(sharePage.getByText(endName)).toBeVisible()
    await expect(sharePage.getByText(startName)).not.toBeVisible()

    await deleteFile(adminPage, endPath)
    await deleteShare(adminPage, token)
    await sharePage.close()
    await shareCtx.close()
    await adminPage.close()
    await adminCtx.close()
  })

  test('admin changes are seen by another admin user', async ({ browser }) => {
    const id = randomUUID().slice(0, 10)
    const fileName = `sse-admin-sync-${id}.txt`
    const filePath = `SharedContent/${fileName}`

    const ctx1 = await createAdminContext(browser)
    const admin1 = await ctx1.newPage()
    const ctx2 = await createAdminContext(browser)
    const admin2 = await ctx2.newPage()

    await admin1.goto('/?dir=SharedContent')
    await expect(admin1.locator('table')).toBeVisible()
    await gotoWithSSE(admin2, '/?dir=SharedContent')
    await expect(admin2.locator('table')).toBeVisible()

    await createFile(admin1, filePath, 'synced')
    await expect(admin1.locator('table').getByText(fileName)).toBeVisible()

    await expect(admin2.locator('table').getByText(fileName)).toBeVisible()

    await deleteFile(admin1, filePath)
    await expect(admin2.locator('table').getByText(fileName)).not.toBeVisible()

    await admin1.close()
    await ctx1.close()
    await admin2.close()
    await ctx2.close()
  })

  test('editable share changes are seen by admin user', async ({ browser }) => {
    const id = randomUUID().slice(0, 10)
    const fileName = `sse-share-to-admin-${id}.txt`
    const filePath = `SharedContent/${fileName}`

    const adminCtx = await createAdminContext(browser)
    const adminPage = await adminCtx.newPage()

    const { url: shareUrl, token } = await createShare(adminPage, {
      path: 'SharedContent',
      isDirectory: true,
      editable: true,
      restrictions: { allowUpload: true, allowEdit: true, allowDelete: true },
    })

    await gotoWithSSE(adminPage, '/?dir=SharedContent')
    await expect(adminPage.locator('table')).toBeVisible()

    const shareCtx = await browser.newContext()
    const sharePage = await shareCtx.newPage()
    await sharePage.goto(shareUrl)
    await expect(sharePage.locator('table')).toBeVisible()

    await fillCreateFileDialog(sharePage, fileName.replace(/\.txt$/, ''))
    await expect(sharePage.locator('table').getByText(fileName)).toBeVisible()

    await expect(adminPage.locator('table').getByText(fileName)).toBeVisible()

    await deleteFile(adminPage, filePath)
    await deleteShare(adminPage, token)
    await sharePage.close()
    await shareCtx.close()
    await adminPage.close()
    await adminCtx.close()
  })

  test('editable share changes are seen by another share user', async ({ browser }) => {
    const id = randomUUID().slice(0, 10)
    const fileName = `sse-share-to-share-${id}.txt`

    const adminCtx = await createAdminContext(browser)
    const adminPage = await adminCtx.newPage()

    const { url: shareUrl, token } = await createShare(adminPage, {
      path: 'SharedContent',
      isDirectory: true,
      editable: true,
      restrictions: { allowUpload: true, allowEdit: true, allowDelete: true },
    })

    const shareCtx1 = await browser.newContext()
    const share1 = await shareCtx1.newPage()
    await share1.goto(shareUrl)
    await expect(share1.locator('table')).toBeVisible()

    const shareCtx2 = await browser.newContext()
    const share2 = await shareCtx2.newPage()
    await gotoWithSSE(share2, shareUrl)
    await expect(share2.locator('table')).toBeVisible()

    await fillCreateFileDialog(share1, fileName.replace(/\.txt$/, ''))
    await expect(share1.locator('table').getByText(fileName)).toBeVisible()
    await expect(share1.locator('[role="dialog"]')).not.toBeVisible()

    await expect(share2.locator('table').getByText(fileName)).toBeVisible()

    await share1.locator('table tr').filter({ hasText: fileName }).click({ button: 'right' })
    await share1.locator('[data-slot="context-menu-item"]').getByText('Delete').click()
    const deleteConfirm = share1.getByRole('alertdialog')
    await expect(deleteConfirm).toBeVisible()
    await deleteConfirm.getByRole('button', { name: /Delete/i }).click({ force: true })

    await expect(share2.locator('table').getByText(fileName)).not.toBeVisible()

    await deleteShare(adminPage, token)
    await share1.close()
    await shareCtx1.close()
    await share2.close()
    await shareCtx2.close()
    await adminPage.close()
    await adminCtx.close()
  })
})
