import { test, expect, type BrowserContext, type Page } from '@playwright/test'
import {
  createTempFile,
  deleteFileViaContextMenu,
  dragToEdge,
  getDragHandle,
  getVisibleContent,
  getWindowGroups,
  gotoWorkspace as gotoWorkspaceDnd,
  html5DragDrop,
  navigateToSharedContent,
  openBrowserWindow,
} from '../e2e/workspace-cross-dnd-helpers'
import { WORKSPACE_VISIBLE_WINDOW_GROUP } from './workspace-layout-helpers'
import { createWorkspaceE2EContext } from './workspace-e2e-auth'

let sharedContext: BrowserContext
let page: Page

test.beforeAll(async ({ browser }) => {
  sharedContext = await createWorkspaceE2EContext(browser)
})

test.afterAll(async () => {
  await sharedContext.close()
})

test.beforeEach(async () => {
  page = await sharedContext.newPage()
})

test.afterEach(async () => {
  await page.close()
})

async function createShare(page: Page, body: Record<string, unknown>) {
  const res = await page.request.post('/api/shares', { data: body })
  const json = (await res.json()) as {
    share: { token: string; passcode?: string }
  }
  const base = `/share/${json.share.token}`
  const url = json.share.passcode ? `${base}?p=${encodeURIComponent(json.share.passcode)}` : base
  return url
}

function toShareWorkspaceUrl(shareUrl: string) {
  const url = new URL(shareUrl, 'http://localhost')
  const parts = url.pathname.split('/')
  parts.splice(3, 0, 'workspace')
  url.pathname = parts.join('/')
  return `${url.pathname}${url.search}`
}

async function gotoWorkspace(page: Page) {
  await page.goto('/workspace')
  await expect(page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP).first()).toBeVisible()
}

function getBrowserContent(page: Page) {
  return getWindowGroups(page).first().locator('.workspace-window-content')
}

test.describe('Workspace taskbar pins', () => {
  test('Add to taskbar from context menu adds pinned icon', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await expect(content.getByText('Documents', { exact: true })).toBeVisible()
    await content
      .locator('table tr')
      .filter({ hasText: 'Documents' })
      .first()
      .click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Add to taskbar').click()

    await expect(page.locator('[title="Folder: Documents"]')).toBeVisible()
    await expect(
      page.locator('[data-taskbar-pin]').locator('[title="Folder: Documents"] svg.text-blue-500'),
    ).toBeVisible()
  })

  test('clicking pinned folder icon opens browser at that folder', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await content
      .locator('table tr')
      .filter({ hasText: 'Documents' })
      .first()
      .click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Add to taskbar').click()

    await expect(page.locator('[title="Folder: Documents"]')).toBeVisible()
    await page.locator('[title="Folder: Documents"]').click()

    await expect(getWindowGroups(page)).toHaveCount(2)
    const secondContent = getWindowGroups(page).nth(1).locator('.workspace-window-content')
    await expect(secondContent.getByText('readme.txt')).toBeVisible()
  })

  test('Unpin from context menu removes pinned icon', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await content
      .locator('table tr')
      .filter({ hasText: 'Documents' })
      .first()
      .click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Add to taskbar').click()

    await expect(page.locator('[title="Folder: Documents"]')).toBeVisible()
    await page.locator('[title="Folder: Documents"]').click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Unpin').click()

    await expect(page.locator('[title="Folder: Documents"]')).not.toBeVisible()
  })

  test('clicking pinned file icon opens viewer for that file', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await content.getByText('Documents', { exact: true }).click()
    await expect(content.getByText('readme.txt')).toBeVisible()
    await content.locator('table tr').filter({ hasText: 'readme.txt' }).click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Add to taskbar').click()

    await expect(page.locator('[title="File: Documents/readme.txt"]')).toBeVisible()
    await page.locator('[title="File: Documents/readme.txt"]').click()

    await expect(getWindowGroups(page)).toHaveCount(2)
    const viewerContent = getWindowGroups(page).nth(1).locator('.workspace-window-content')
    await expect(viewerContent.getByText('readme', { exact: false })).toBeVisible({
      timeout: 5_000,
    })
  })

  test('clicking pinned unsupported file shows unsupported file viewer', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await content.getByText('Documents', { exact: true }).click()
    await expect(content.getByText('unsupported.xyz')).toBeVisible()
    await content
      .locator('table tr')
      .filter({ hasText: 'unsupported.xyz' })
      .click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Add to taskbar').click()

    await expect(page.locator('[title="File: Documents/unsupported.xyz"]')).toBeVisible()
    await page.locator('[title="File: Documents/unsupported.xyz"]').click()

    await expect(getWindowGroups(page)).toHaveCount(2)
    const viewerContent = getWindowGroups(page).nth(1).locator('.workspace-window-content')
    await expect(viewerContent.getByText('This file type cannot be previewed.')).toBeVisible({
      timeout: 5_000,
    })
    await expect(viewerContent.getByRole('link', { name: 'Download File' })).toBeVisible()
  })

  test('admin workspace: pins persist in settings after reload', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    const savePinned = page.waitForResponse(
      (resp) =>
        resp.request().method() === 'POST' &&
        resp.url().includes('/api/settings/workspaceTaskbarPins'),
    )
    await expect(content.getByText('Documents', { exact: true })).toBeVisible()
    await content
      .locator('table tr')
      .filter({ hasText: 'Documents' })
      .first()
      .click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Add to taskbar').click()
    await expect(page.locator('[title="Folder: Documents"]')).toBeVisible()
    await savePinned

    await page.reload()
    await expect(page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP).first()).toBeVisible()
    await expect(page.locator('[title="Folder: Documents"]')).toBeVisible()
  })

  test('share workspace: pins persist on share after reload', async () => {
    const shareUrl = await createShare(page, {
      path: 'SharedContent',
      isDirectory: true,
    })
    await page.goto(toShareWorkspaceUrl(shareUrl))
    await expect(page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP).first()).toBeVisible()

    const content = getBrowserContent(page)
    await expect(content.getByText('subfolder', { exact: true })).toBeVisible()
    const savePinned = page.waitForResponse(
      (resp) =>
        resp.request().method() === 'POST' &&
        /\/api\/share\/[^/]+\/workspaceTaskbarPins/.test(new URL(resp.url()).pathname),
    )
    await content
      .locator('table tr')
      .filter({ hasText: 'subfolder' })
      .first()
      .click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Add to taskbar').click()
    await expect(page.locator('[title="Folder: SharedContent/subfolder"]')).toBeVisible()
    await savePinned

    await page.reload()
    await expect(page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP).first()).toBeVisible()
    await expect(page.locator('[title="Folder: SharedContent/subfolder"]')).toBeVisible()
  })

  test('dragging pinned file to folder in another browser moves the file', async () => {
    await gotoWorkspaceDnd(page)
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)
    await dragToEdge(page, getDragHandle(groups.first()), 'left')
    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)
    await dragToEdge(page, getDragHandle(groups.nth(1)), 'right')

    const contentA = getVisibleContent(groups.first())
    const contentB = getVisibleContent(groups.nth(1))

    await navigateToSharedContent(contentA)
    await navigateToSharedContent(contentB)

    const tempFile = 'pin-cross-dnd-test.txt'
    await createTempFile(page, contentA, tempFile)

    const fileRow = contentA.locator('tr').filter({ hasText: tempFile })
    await fileRow.dispatchEvent('contextmenu')
    await page.locator('[data-slot="context-menu-item"]').getByText('Add to taskbar').click()

    const pinTitle = `File: SharedContent/${tempFile}`
    const pinSource = page
      .locator('[data-taskbar-pin]')
      .filter({ has: page.locator(`[title="${pinTitle}"]`) })

    const targetRow = contentB.locator('tr').filter({ hasText: 'subfolder' }).first()
    await html5DragDrop(pinSource, targetRow)

    await expect(contentA.getByText(tempFile)).not.toBeVisible({ timeout: 5_000 })

    await contentB.getByText('subfolder').first().click()
    await expect(contentB.getByText(tempFile)).toBeVisible({ timeout: 5_000 })

    await deleteFileViaContextMenu(page, contentB, tempFile)
  })
})
