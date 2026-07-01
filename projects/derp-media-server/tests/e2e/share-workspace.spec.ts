import { test, expect, type Page } from '@playwright/test'
import path from 'path'

const sessionFile = process.env.BATCH_ID ? `session-${process.env.BATCH_ID}.json` : 'session.json'
const authStoragePath = path.resolve(__dirname, '../fixtures/.auth', sessionFile)

let folderShareUrl: string
let editableShareUrl: string
let folderShareWorkspaceUrl: string
let editableShareWorkspaceUrl: string

async function createShare(page: Page, body: Record<string, unknown>): Promise<string> {
  const res = await page.request.post('/api/shares', { data: body })
  const json = await res.json()
  const base = `/share/${json.share.token}`
  return json.share.passcode ? `${base}?p=${encodeURIComponent(json.share.passcode)}` : base
}

function toWorkspaceUrl(shareUrl: string): string {
  const url = new URL(shareUrl, 'http://localhost')
  const parts = url.pathname.split('/')
  parts.splice(3, 0, 'workspace')
  url.pathname = parts.join('/')
  return `${url.pathname}${url.search}`
}

function getShareToken(shareUrl: string): string {
  return new URL(shareUrl, 'http://localhost').pathname.split('/')[2]
}

function uniqueId() {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
}

function watchRequests(page: Page) {
  const requests: string[] = []
  page.context().on('request', (req) => requests.push(req.url()))
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

function getWindowGroups(page: Page) {
  return page.locator('[data-window-group]:not([data-workspace-window-minimized])')
}

function getBrowserContent(page: Page) {
  return getWindowGroups(page).first().locator('.workspace-window-content')
}

async function chooseWorkspaceOpenTarget(page: Page, label: 'New tab' | 'New window') {
  const dialog = page
    .getByRole('dialog')
    .filter({ has: page.getByRole('heading', { name: 'Settings' }) })
  await dialog.getByRole('button', { name: label }).click()
  await page.keyboard.press('Escape')
}

async function gotoShareWorkspace(page: Page, url: string) {
  await page.goto(url)
  await expect(
    page.locator('[data-window-group]:not([data-workspace-window-minimized])'),
  ).toBeVisible()
}

test.describe('Share Workspace', () => {
  test.beforeAll(async ({ browser }) => {
    const context = await browser.newContext({ storageState: authStoragePath })
    const page = await context.newPage()
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
    folderShareWorkspaceUrl = toWorkspaceUrl(folderShareUrl)
    editableShareWorkspaceUrl = toWorkspaceUrl(editableShareUrl)
    await page.close()
    await context.close()
  })

  test('file-only share /workspace URL redirects to standard file share', async ({ page }) => {
    const shareUrl = await createShare(page, {
      path: 'Documents/readme.txt',
      isDirectory: false,
    })
    const workspaceUrl = toWorkspaceUrl(shareUrl)
    const token = getShareToken(shareUrl)
    await page.goto(workspaceUrl)
    await expect(page).toHaveURL(new RegExp(`.*/share/${token}(?:\\?|$)`))
  })

  test('opens share in workspace view and shows files', async ({ page }) => {
    const requests = watchRequests(page)

    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    const content = getBrowserContent(page)
    await expect(content.getByText('public-doc.txt')).toBeVisible()
    await expect(content.getByText('subfolder')).toBeVisible()
    expectNoAdminShareLeaks(requests)
  })

  test('navigates into subfolder via workspace browser', async ({ page }) => {
    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    const content = getBrowserContent(page)

    await content.getByText('subfolder').first().click()
    await expect(content.getByText('nested.txt')).toBeVisible()
  })

  test('navigates back via breadcrumbs', async ({ page }) => {
    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    const content = getBrowserContent(page)

    await content.getByText('subfolder').first().click()
    await expect(content.getByText('nested.txt')).toBeVisible()

    await content.getByRole('button', { name: 'SharedContent' }).click()
    await expect(content.getByText('public-doc.txt')).toBeVisible()
  })

  test('opens text file in viewer window', async ({ page }) => {
    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    const content = getBrowserContent(page)

    await content.locator('table').getByText('public-doc.txt').click()
    await expect(getWindowGroups(page)).toHaveCount(2)

    const viewerContent = getWindowGroups(page).nth(1).locator('.workspace-window-content')
    await expect(viewerContent.getByText('public document for share testing')).toBeVisible()
  })

  test('share workspace opens file in a new tab when setting is New tab', async ({ page }) => {
    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    await page.getByRole('button', { name: 'Open settings' }).click()
    await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible()
    await chooseWorkspaceOpenTarget(page, 'New tab')

    const content = getBrowserContent(page)
    await expect(content.getByText('public-doc.txt')).toBeVisible()
    await content.locator('table').getByText('public-doc.txt').click()

    await expect(getWindowGroups(page)).toHaveCount(1)
    const tabStrip = page.locator('.workspace-tab-strip')
    await expect(tabStrip.getByText('public-doc.txt')).toBeVisible()
  })

  test('share workspace opens file in a new window when setting is New window', async ({
    page,
  }) => {
    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    await page.getByRole('button', { name: 'Open settings' }).click()
    await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible()
    await chooseWorkspaceOpenTarget(page, 'New window')

    const content = getBrowserContent(page)
    await expect(getWindowGroups(page)).toHaveCount(1)
    await content.locator('table').getByText('public-doc.txt').click()
    await expect(getWindowGroups(page)).toHaveCount(2)
  })

  test('opens image in viewer window', async ({ page }) => {
    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    const content = getBrowserContent(page)

    await content.locator('table').getByText('photo.jpg').click()
    await expect(getWindowGroups(page)).toHaveCount(2)

    const viewerContent = getWindowGroups(page).nth(1).locator('.workspace-window-content')
    await expect(viewerContent.locator('img')).toBeVisible()
  })

  test('plays video in workspace player', async ({ page }) => {
    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    const content = getBrowserContent(page)

    await content.locator('table').getByText('public-video.mp4').click()
    await expect(page.locator('video')).toBeVisible()
  })

  test('workspace share requests stay scoped to share APIs', async ({ page }) => {
    const requests = watchRequests(page)
    const consoleLines = watchConsole(page)
    const token = getShareToken(folderShareWorkspaceUrl)

    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    const content = getBrowserContent(page)
    await expect(content.getByText('public-doc.txt')).toBeVisible()

    await content.getByText('subfolder').first().click()
    await expect(content.getByText('nested.txt')).toBeVisible()

    expect(
      requests.some((url) => new URL(url).pathname === `/api/share/${token}/stream`) ||
        sawShareSseConnect(consoleLines),
    ).toBeTruthy()
    expectNoAdminShareLeaks(requests)
  })

  test('non-editable share workspace hides edit controls', async ({ page }) => {
    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    const content = getBrowserContent(page)
    await expect(content.getByText('public-doc.txt')).toBeVisible()

    await expect(content.locator('button[title="Create new file"]')).not.toBeVisible()
    await expect(content.locator('button[title="Create new folder"]')).not.toBeVisible()
  })

  test('editable share workspace shows edit controls', async ({ page }) => {
    await gotoShareWorkspace(page, editableShareWorkspaceUrl)
    const content = getBrowserContent(page)
    await expect(content.getByText('public-doc.txt')).toBeVisible()

    await expect(content.locator('button[title="Create new file"]')).toBeVisible()
    await expect(content.locator('button[title="Create new folder"]')).toBeVisible()
  })

  test('creates and deletes a file in editable share workspace', async ({ page }) => {
    await gotoShareWorkspace(page, editableShareWorkspaceUrl)
    const content = getBrowserContent(page)
    await expect(content.getByText('public-doc.txt')).toBeVisible()

    await content.locator('button[title="Create new file"]').click()
    const createFileDialog = page.getByRole('dialog', { name: /create.*file/i })
    const nameInput = createFileDialog.getByRole('textbox')
    await nameInput.clear()
    await nameInput.fill('ws-test-file.txt')
    await createFileDialog.getByRole('button', { name: 'Create', exact: true }).click()

    await expect(content.locator('table').getByText('ws-test-file.txt')).toBeVisible()

    // Clean up: delete via API
    const token = getShareToken(editableShareWorkspaceUrl)
    const deleteRes = await page.request.post(`/api/share/${token}/delete`, {
      data: { path: 'ws-test-file.txt' },
    })
    expect(deleteRes.ok()).toBeTruthy()
  })

  test('creates and deletes a folder in editable share workspace', async ({ page }) => {
    await gotoShareWorkspace(page, editableShareWorkspaceUrl)
    const content = getBrowserContent(page)
    await expect(content.getByText('public-doc.txt')).toBeVisible()

    await content.locator('button[title="Create new folder"]').click()
    const createFolderDialog = page.getByRole('dialog', { name: /create.*folder/i })
    await createFolderDialog.locator('input[placeholder="Folder name"]').fill('ws-test-folder')
    await createFolderDialog.getByRole('button', { name: 'Create', exact: true }).click()

    await expect(content.getByText('ws-test-folder')).toBeVisible()

    // Clean up
    const token = getShareToken(editableShareWorkspaceUrl)
    const deleteRes = await page.request.post(`/api/share/${token}/delete`, {
      data: { path: 'ws-test-folder' },
    })
    expect(deleteRes.ok()).toBeTruthy()
  })

  test('context menu "Open in Workspace" appears on folders in share view', async ({ page }) => {
    await page.goto(folderShareUrl)
    await expect(page.getByText('subfolder')).toBeVisible()

    await page.locator('table tr').filter({ hasText: 'subfolder' }).click({ button: 'right' })
    await expect(
      page.locator('[data-slot="context-menu-item"]').getByText('Open in Workspace'),
    ).toBeVisible()
  })

  test('deletes a file via context menu in editable share workspace', async ({ page }) => {
    const id = uniqueId()
    const name = `ws-ctx-del-${id}.txt`
    await gotoShareWorkspace(page, editableShareWorkspaceUrl)
    const content = getBrowserContent(page)
    await expect(content.getByText('public-doc.txt')).toBeVisible()

    await content.locator('button[title="Create new file"]').click()
    const createFileDialog = page.getByRole('dialog', { name: /create.*file/i })
    await createFileDialog.getByRole('textbox').fill(name)
    await createFileDialog.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(content.locator('table').getByText(name)).toBeVisible()

    await content.locator('table tr').filter({ hasText: name }).click({ button: 'right' })
    await page.getByRole('menuitem', { name: 'Delete' }).click()
    await page.getByRole('alertdialog').getByRole('button', { name: 'Delete' }).click()
    await expect(content.locator('table').getByText(name)).not.toBeVisible()
  })

  test('renames a file via context menu in editable share workspace', async ({ page }) => {
    const id = uniqueId()
    const oldName = `ws-ctx-ren-${id}.txt`
    const newName = `ws-ctx-ren-${id}-moved.txt`
    await gotoShareWorkspace(page, editableShareWorkspaceUrl)
    const content = getBrowserContent(page)
    await expect(content.getByText('public-doc.txt')).toBeVisible()

    await content.locator('button[title="Create new file"]').click()
    const createFileDialog2 = page.getByRole('dialog', { name: /create.*file/i })
    await createFileDialog2.getByRole('textbox').fill(oldName)
    await createFileDialog2.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(content.locator('table').getByText(oldName)).toBeVisible()

    await content.locator('table tr').filter({ hasText: oldName }).click({ button: 'right' })
    await page.getByRole('menuitem', { name: 'Rename' }).click()
    await page.locator('[role="dialog"]').getByPlaceholder('New name').fill(newName)
    await page
      .locator('[role="dialog"]')
      .getByRole('button', { name: /^Rename$/ })
      .click()
    await expect(content.locator('table').getByText(newName)).toBeVisible()

    const token = getShareToken(editableShareWorkspaceUrl)
    await page.request.post(`/api/share/${token}/delete`, { data: { path: newName } })
  })

  test('share workspace grid view survives reload', async ({ page }) => {
    await gotoShareWorkspace(page, folderShareWorkspaceUrl)
    const content = getBrowserContent(page)
    await expect(content.getByText('public-doc.txt')).toBeVisible()

    await content.getByRole('button', { name: 'Grid view' }).click()
    await expect(content.locator('.aspect-video').first()).toBeVisible()

    await page.reload()
    await expect(
      page.locator('[data-window-group]:not([data-workspace-window-minimized])'),
    ).toBeVisible()
    const contentAfter = getBrowserContent(page)
    await expect(contentAfter.getByText('public-doc.txt')).toBeVisible()
    await expect(contentAfter.locator('.aspect-video').first()).toBeVisible()
  })
})
