import { test, expect, Page } from '@playwright/test'
import path from 'path'

const sessionFile = process.env.BATCH_ID ? `session-${process.env.BATCH_ID}.json` : 'session.json'
const authStoragePath = path.resolve(__dirname, '../fixtures/.auth', sessionFile)

let shareToken: string
let shareUrl: string

async function authenticateShare(page: Page) {
  await page.goto(shareUrl)
  await page.waitForSelector('table')
}

test.describe('Share Audio API Endpoints', () => {
  test.beforeAll(async ({ browser }) => {
    const context = await browser.newContext({ storageState: authStoragePath })
    const page = await context.newPage()
    const res = await page.request.post('/api/shares', {
      data: { path: 'SharedContent', isDirectory: true },
    })
    const json = await res.json()
    shareToken = json.share.token
    shareUrl = json.share.passcode
      ? `/share/${shareToken}?p=${encodeURIComponent(json.share.passcode)}`
      : `/share/${shareToken}`
    await page.close()
    await context.close()
  })

  // ── Audio Metadata Endpoint ─────────────────────────────────────────

  test('audio metadata endpoint returns 200 with JSON for valid audio file', async ({ page }) => {
    await authenticateShare(page)
    const res = await page.request.get(`/api/share/${shareToken}/audio/metadata/track.mp3`)
    expect(res.status()).toBe(200)
    const json = await res.json()
    expect(json).toHaveProperty('title')
    expect(json).toHaveProperty('artist')
    expect(json).toHaveProperty('album')
    expect(json).toHaveProperty('duration')
  })

  test('audio metadata endpoint returns error for non-existent file', async ({ page }) => {
    await authenticateShare(page)
    const res = await page.request.get(`/api/share/${shareToken}/audio/metadata/nonexistent.mp3`)
    expect(res.status()).toBeGreaterThanOrEqual(400)
  })

  test('audio metadata endpoint returns error for invalid share token', async ({ page }) => {
    const res = await page.request.get(`/api/share/invalid-token-xyz/audio/metadata/track.mp3`)
    expect(res.status()).toBe(404)
    const json = await res.json()
    expect(json.error).toContain('Share not found')
  })

  // ── Audio Extraction Endpoint ───────────────────────────────────────

  test('audio extraction endpoint returns 200 with audio stream for video file', async ({
    page,
  }) => {
    await authenticateShare(page)
    const res = await page.request.get(`/api/share/${shareToken}/audio/extract/public-video.mp4`)
    expect(res.status()).toBe(200)
    const contentType = res.headers()['content-type']
    expect(contentType).toContain('audio/')
  })

  test('audio extraction endpoint returns error for non-existent video', async ({ page }) => {
    await authenticateShare(page)
    const res = await page.request.get(`/api/share/${shareToken}/audio/extract/nonexistent.mp4`)
    expect(res.status()).toBeGreaterThanOrEqual(400)
  })

  test('audio extraction endpoint returns error for invalid share token', async ({ page }) => {
    const res = await page.request.get(
      `/api/share/invalid-token-xyz/audio/extract/public-video.mp4`,
    )
    expect(res.status()).toBe(404)
    const json = await res.json()
    expect(json.error).toContain('Share not found')
  })

  test('audio extraction endpoint returns error for non-video file', async ({ page }) => {
    await authenticateShare(page)
    const res = await page.request.get(`/api/share/${shareToken}/audio/extract/public-doc.txt`)
    expect(res.status()).toBe(400)
  })
})
