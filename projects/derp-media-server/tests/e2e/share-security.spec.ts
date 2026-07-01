import { test, expect, Page } from '@playwright/test'
import path from 'path'

const sessionFile = process.env.BATCH_ID ? `session-${process.env.BATCH_ID}.json` : 'session.json'
const authStoragePath = path.resolve(__dirname, '../fixtures/.auth', sessionFile)

let kbShareToken: string
let kbSharePasscode: string
let folderShareToken: string
let folderSharePasscode: string

async function createShare(
  page: Page,
  body: Record<string, unknown>,
): Promise<{ token: string; passcode: string }> {
  const res = await page.request.post('/api/shares', { data: body })
  const json = await res.json()
  return { token: json.share.token, passcode: json.share.passcode || '' }
}

async function authenticateShare(page: Page, token: string, passcode: string) {
  if (!passcode) return
  await page.request.post(`/api/share/${token}/verify`, { data: { passcode } })
}

test.describe('Share Security', () => {
  test.beforeAll(async ({ browser }) => {
    const context = await browser.newContext({ storageState: authStoragePath })
    const page = await context.newPage()
    const folder = await createShare(page, { path: 'SharedContent', isDirectory: true })
    folderShareToken = folder.token
    folderSharePasscode = folder.passcode
    const kb = await createShare(page, { path: 'Notes', isDirectory: true })
    kbShareToken = kb.token
    kbSharePasscode = kb.passcode
    await page.close()
    await context.close()
  })

  test('rejects path traversal in share file listing', async ({ page }) => {
    await authenticateShare(page, folderShareToken, folderSharePasscode)
    const res = await page.request.get(
      `/api/share/${folderShareToken}/files?dir=${encodeURIComponent('../../')}`,
    )
    expect(res.status()).toBeGreaterThanOrEqual(400)
  })

  test('rejects path traversal with ../ in file listing', async ({ page }) => {
    await authenticateShare(page, folderShareToken, folderSharePasscode)
    const res = await page.request.get(
      `/api/share/${folderShareToken}/files?dir=${encodeURIComponent('../Documents')}`,
    )
    expect(res.status()).toBeGreaterThanOrEqual(400)
  })

  test('rejects path traversal in KB search', async ({ page }) => {
    await authenticateShare(page, kbShareToken, kbSharePasscode)
    const res = await page.request.get(
      `/api/share/${kbShareToken}/kb/search?q=test&dir=${encodeURIComponent('../../Documents')}`,
    )
    expect(res.status()).toBeGreaterThanOrEqual(400)
  })

  test('rejects path traversal in KB recent', async ({ page }) => {
    await authenticateShare(page, kbShareToken, kbSharePasscode)
    const res = await page.request.get(
      `/api/share/${kbShareToken}/kb/recent?dir=${encodeURIComponent('../../Documents')}`,
    )
    expect(res.status()).toBeGreaterThanOrEqual(400)
  })

  test('share folder download returns ZIP', async ({ page }) => {
    await authenticateShare(page, folderShareToken, folderSharePasscode)
    const res = await page.request.get(`/api/share/${folderShareToken}/download`)
    expect(res.ok()).toBeTruthy()
    const disposition = res.headers()['content-disposition']
    expect(disposition).toContain('.zip')
  })
})

test.describe('Auth - Protected Stream Endpoints', () => {
  test('unauthenticated /api/events/stream returns 401', async ({ baseURL }) => {
    const res = await fetch(`${baseURL}/api/events/stream`)
    expect(res.status).toBe(401)
  })
})
