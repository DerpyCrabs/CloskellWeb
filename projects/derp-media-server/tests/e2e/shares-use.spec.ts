import { test, expect, Page } from '@playwright/test'
import path from 'path'

const sessionFile = process.env.BATCH_ID ? `session-${process.env.BATCH_ID}.json` : 'session.json'
const authStoragePath = path.resolve(__dirname, '../fixtures/.auth', sessionFile)

let fileShareUrl: string
let folderShareUrl: string
let editableShareUrl: string

async function createShare(page: Page, body: Record<string, unknown>): Promise<string> {
  const res = await page.request.post('/api/shares', { data: body })
  const json = await res.json()
  const base = `/share/${json.share.token}`
  return json.share.passcode ? `${base}?p=${encodeURIComponent(json.share.passcode)}` : base
}

function watchShareRequests(page: Page) {
  const requests: string[] = []
  page.context().on('request', (request) => {
    requests.push(request.url())
  })
  return requests
}

function watchConsole(page: Page) {
  const lines: string[] = []
  page.on('console', (msg) => lines.push(msg.text()))
  return lines
}

function sawShareSseConnect(consoleLines: string[]) {
  return consoleLines.some((l) => l.includes('[Share SSE] Connected to share stream'))
}

function getShareToken(shareUrl: string): string {
  return new URL(shareUrl, 'http://localhost').pathname.split('/')[2]
}

function expectNoAdminShareLeaks(requests: string[]) {
  const forbiddenAdminPaths = new Set([
    '/api/auth/config',
    '/api/settings',
    '/api/stats/views',
    '/api/shares',
    '/api/files',
    '/api/events/stream',
  ])

  const adminLeaks = requests.filter((url) => {
    const pathname = new URL(url).pathname
    return forbiddenAdminPaths.has(pathname)
  })
  expect(adminLeaks).toEqual([])

  const unscopedLeaks = requests.filter((url) => {
    if (!url.includes('/api/share/')) return false
    return (
      url.includes('SharedContent%2F') ||
      url.includes('/SharedContent/') ||
      url.includes('dir=SharedContent') ||
      url.includes('path=SharedContent')
    )
  })
  expect(unscopedLeaks).toEqual([])
}

test.describe('Using Shares', () => {
  test.beforeAll(async ({ browser }) => {
    const context = await browser.newContext({ storageState: authStoragePath })
    const page = await context.newPage()
    fileShareUrl = await createShare(page, {
      path: 'Documents/readme.txt',
      isDirectory: false,
    })
    folderShareUrl = await createShare(page, {
      path: 'SharedContent',
      isDirectory: true,
    })
    editableShareUrl = await createShare(page, {
      path: 'SharedContent',
      isDirectory: true,
      editable: true,
      restrictions: {
        allowUpload: true,
        allowEdit: true,
        allowDelete: true,
      },
    })
    await page.close()
    await context.close()
  })

  test('views a shared text file page', async ({ page }) => {
    await page.goto(fileShareUrl)
    await expect(page.getByText('readme.txt')).toBeVisible()
    await expect(page.getByText('TXT File')).toBeVisible()
  })

  test('shared file page shows download button', async ({ page }) => {
    await page.goto(fileShareUrl)
    await expect(
      page.getByRole('button', { name: /Download/i }).or(page.locator('a:has-text("Download")')),
    ).toBeVisible()
  })

  test('browses a shared folder', async ({ page }) => {
    const requests = watchShareRequests(page)

    await page.goto(folderShareUrl)
    await expect(page.getByText('public-doc.txt')).toBeVisible()
    await expect(page.getByText('subfolder')).toBeVisible()
    expectNoAdminShareLeaks(requests)
  })

  test('share interactions stay scoped to share APIs', async ({ page }) => {
    const requests = watchShareRequests(page)
    const consoleLines = watchConsole(page)
    const token = getShareToken(folderShareUrl)

    await page.goto(folderShareUrl)
    await expect(page.getByText('public-doc.txt')).toBeVisible()

    await page.getByText('public-doc.txt').click()
    await expect(page.getByText('public document for share testing')).toBeVisible()
    await page.getByRole('button', { name: 'Close' }).click()

    await page.getByText('subfolder').first().click()
    await page.waitForURL(/dir=subfolder/)
    await expect(page.getByText('nested.txt')).toBeVisible()

    await page.getByRole('button', { name: 'SharedContent' }).click()
    await expect(page.getByText('public-video.mp4')).toBeVisible()

    await page.getByText('public-video.mp4').click()
    await expect(page.locator('video')).toBeVisible()

    expect(
      requests.some((url) => new URL(url).pathname === `/api/share/${token}/stream`) ||
        sawShareSseConnect(consoleLines),
    ).toBeTruthy()
    expectNoAdminShareLeaks(requests)
  })

  test('share receives live updates through scoped stream', async ({ page }) => {
    const requests = watchShareRequests(page)
    const consoleLines = watchConsole(page)
    const token = getShareToken(editableShareUrl)
    const liveFileName = `live-share-update-${Date.now()}.txt`

    await page.goto(editableShareUrl)
    await expect(page.getByText('public-doc.txt')).toBeVisible()
    await page.waitForEvent('console', {
      predicate: (msg) => msg.text().includes('[Share SSE] Connected to share stream'),
      timeout: 10000,
    })

    const createResponse = await page.request.post(`/api/share/${token}/create`, {
      data: {
        type: 'file',
        path: liveFileName,
        content: 'live update',
      },
    })
    expect(createResponse.ok()).toBeTruthy()

    await expect(page.locator('table').getByText(liveFileName)).toBeVisible()
    expect(
      requests.some((url) => new URL(url).pathname === `/api/share/${token}/stream`) ||
        sawShareSseConnect(consoleLines),
    ).toBeTruthy()
    expectNoAdminShareLeaks(requests)

    const deleteResponse = await page.request.post(`/api/share/${token}/delete`, {
      data: { path: liveFileName },
    })
    expect(deleteResponse.ok()).toBeTruthy()
    await expect(page.locator('table').getByText(liveFileName)).not.toBeVisible()
  })

  test('navigates into subfolder within shared folder', async ({ page }) => {
    await page.goto(folderShareUrl)
    await page.getByText('subfolder').first().click()
    await page.waitForURL(/dir=subfolder/)
    await expect(page.getByText('nested.txt')).toBeVisible()
  })

  test('uses breadcrumbs to navigate within share', async ({ page }) => {
    const sep = folderShareUrl.includes('?') ? '&' : '?'
    await page.goto(`${folderShareUrl}${sep}dir=subfolder`)
    await expect(page.getByText('nested.txt')).toBeVisible()
    await page.getByRole('button', { name: 'SharedContent' }).click()
    await expect(page.getByText('public-doc.txt')).toBeVisible()
  })

  test('share folder breadcrumb context menu offers download and workspace', async ({ page }) => {
    const sep = folderShareUrl.includes('?') ? '&' : '?'
    await page.goto(`${folderShareUrl}${sep}dir=subfolder`)
    const root = page.locator('[data-testid="share-file-browser"]')
    await root.locator('[data-breadcrumb-path="subfolder"]').click({ button: 'right' })
    await expect(page.getByTestId('breadcrumb-menu-download-zip')).toBeVisible()
    await expect(page.getByTestId('breadcrumb-menu-open-workspace')).toBeVisible()
  })

  test('plays video in shared folder', async ({ page }) => {
    await page.goto(folderShareUrl)
    await page.getByText('public-video.mp4').click()
    await expect(page.locator('video')).toBeVisible()
  })

  test('views text file in shared folder', async ({ page }) => {
    await page.goto(folderShareUrl)
    await page.getByText('public-doc.txt').click()
    await expect(page.getByText('public document for share testing')).toBeVisible()
  })

  test('edits a file in editable share', async ({ page }) => {
    await page.goto(editableShareUrl)
    await page.locator('table').getByText('public-doc.txt').click()
    const textarea = page.locator('textarea')
    const closeButton = page.locator('button[title="Close"]')
    await expect(textarea).toBeVisible()

    await textarea.fill('Edited via share.\n')
    await Promise.all([
      page.waitForResponse(
        (resp) =>
          resp.url().includes('/api/share/') &&
          resp.url().endsWith('/edit') &&
          resp.status() === 200,
      ),
      closeButton.focus(),
    ])

    // Close and reopen to verify persistence
    await closeButton.click()
    await expect(page.locator('[role="dialog"]')).not.toBeVisible()
    await page.locator('table').getByText('public-doc.txt').click()
    await expect(page.locator('textarea')).toBeVisible()
    const content = await page.locator('textarea').inputValue()
    expect(content).toContain('Edited via share')

    // Restore original content
    await page.locator('textarea').fill('This is a public document for share testing.\n')
    await Promise.all([
      page.waitForResponse(
        (resp) =>
          resp.url().includes('/api/share/') &&
          resp.url().endsWith('/edit') &&
          resp.status() === 200,
      ),
      closeButton.focus(),
    ])
    await closeButton.click()
  })

  test('creates a file in editable share', async ({ page }) => {
    await page.goto(editableShareUrl)
    await page.locator('button[title="Create new file"]').click()
    const dialog = page.getByRole('dialog', { name: /create.*file/i })
    const nameInput = dialog.getByRole('textbox')
    await nameInput.clear()
    await nameInput.fill('share-created.txt')
    await Promise.all([
      page.waitForResponse(
        (r) => r.url().includes('/api/share/') && r.url().includes('/create') && r.status() === 200,
      ),
      nameInput.press('Enter'),
    ])
    await expect(page.locator('table').getByText('share-created.txt')).toBeVisible()
  })

  test('creates a folder in editable share', async ({ page }) => {
    await page.goto(editableShareUrl)
    await page.locator('button[title="Create new folder"]').click()
    const dialog = page.getByRole('dialog', { name: /create.*folder/i })
    await dialog.locator('input[placeholder="Folder name"]').fill('share-folder')
    await dialog.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page.getByText('share-folder')).toBeVisible()
  })

  test('deletes a file in editable share', async ({ page }) => {
    await page.goto(editableShareUrl)
    await expect(page.locator('table').getByText('share-created.txt')).toBeVisible()

    await page
      .locator('table tr')
      .filter({ hasText: 'share-created.txt' })
      .click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Delete').click()
    await page.getByRole('button', { name: /Delete/i }).click()

    await expect(page.locator('table').getByText('share-created.txt')).not.toBeVisible()
  })

  test('non-editable share hides edit controls', async ({ page }) => {
    await page.goto(folderShareUrl)
    await expect(page.locator('button[title="Create new file"]')).not.toBeVisible()
    await expect(page.locator('button[title="Create new folder"]')).not.toBeVisible()

    await page.locator('table').getByText('public-doc.txt').click()
    await expect(page.locator('[role="dialog"]')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Edit' })).not.toBeVisible()
  })
})
