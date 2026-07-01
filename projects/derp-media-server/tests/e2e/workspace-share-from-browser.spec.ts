import { test, expect, type Page } from '@playwright/test'
import path from 'path'
import { gotoWorkspace, getWindowGroups } from './workspace-layout-helpers'
import { navigateToSharedContent } from './workspace-cross-dnd-helpers'

const sessionFile = process.env.BATCH_ID ? `session-${process.env.BATCH_ID}.json` : 'session.json'
const authStoragePath = path.resolve(__dirname, '../fixtures/.auth', sessionFile)

function getBrowserContent(page: Page) {
  return getWindowGroups(page).first().locator('.workspace-window-content')
}

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

function watchRequests(page: Page) {
  const requests: string[] = []
  page.on('request', (req) => requests.push(req.url()))
  return requests
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
    try {
      return forbiddenAdminPaths.has(new URL(url).pathname)
    } catch {
      return false
    }
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

test.describe('Workspace share from browser', () => {
  test('admin workspace lists Share / Manage Share and opens Share Links dialog', async ({
    page,
  }) => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await navigateToSharedContent(content)

    await content.locator('table tr').filter({ hasText: 'subfolder' }).click({ button: 'right' })
    await page
      .locator('[data-slot="context-menu-item"]')
      .getByText(/Share|Manage Share/)
      .click()

    await expect(page.getByRole('heading', { name: 'Share Links' })).toBeVisible()
    await page.keyboard.press('Escape')
  })

  test('guest share workspace session does not call admin APIs', async ({ browser }) => {
    const adminContext = await browser.newContext({ storageState: authStoragePath })
    const adminPage = await adminContext.newPage()
    const shareUrl = await createShare(adminPage, { path: 'SharedContent', isDirectory: true })
    await adminPage.close()
    await adminContext.close()

    const guest = await browser.newContext()
    const guestPage = await guest.newPage()
    const requests = watchRequests(guestPage)
    const wsPath = toWorkspaceUrl(shareUrl)
    await guestPage.goto(wsPath)
    await expect(
      guestPage.locator('[data-window-group]:not([data-workspace-window-minimized])'),
    ).toBeVisible()

    const content = getWindowGroups(guestPage).first().locator('.workspace-window-content')
    await expect(content.getByText('public-doc.txt')).toBeVisible()
    await content.getByText('subfolder').first().click()
    await expect(content.getByText('nested.txt')).toBeVisible()

    expectNoAdminShareLeaks(requests)
    await guest.close()
  })
})
