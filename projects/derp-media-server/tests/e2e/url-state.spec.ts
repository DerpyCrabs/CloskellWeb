import { test, expect, Page } from '@playwright/test'
import path from 'path'

const sessionFile = process.env.BATCH_ID ? `session-${process.env.BATCH_ID}.json` : 'session.json'
const authStoragePath = path.resolve(__dirname, '../fixtures/.auth', sessionFile)

const AUDIO_FILE = 'Music/track.mp3'
const VIDEO_FILE = 'Videos/sample.mp4'
const TEXT_FILE = 'Documents/readme.txt'

test.describe('URL State – Main Page', () => {
  test('viewing a file preserves playing param', async ({ page }) => {
    await page.goto(`/?dir=Documents&playing=${encodeURIComponent(AUDIO_FILE)}`)
    await page.locator('table').getByText('readme.txt').click()
    await expect(page).toHaveURL(/viewing=/)
    await expect(page).toHaveURL(/playing=/)
  })

  test('navigating to a folder preserves playing param', async ({ page }) => {
    await page.goto(`/?playing=${encodeURIComponent(AUDIO_FILE)}`)
    await page.locator('table').getByText('Documents', { exact: true }).click()
    await expect(page).toHaveURL(/dir=Documents/)
    await expect(page).toHaveURL(/playing=/)
  })

  test('closing viewer preserves playing param', async ({ page }) => {
    await page.goto(
      `/?dir=Documents&viewing=${encodeURIComponent(TEXT_FILE)}&playing=${encodeURIComponent(AUDIO_FILE)}`,
    )
    await expect(page.locator('[role="dialog"]')).toBeVisible()
    await page.locator('[role="dialog"] button[title="Close"]').click()
    await expect(page).not.toHaveURL(/viewing=/)
    await expect(page).toHaveURL(/playing=/)
  })

  test('closing player preserves viewing param', async ({ page }) => {
    await page.goto(
      `/?dir=Videos&viewing=${encodeURIComponent(TEXT_FILE)}&playing=${encodeURIComponent(VIDEO_FILE)}`,
    )
    await expect(page.locator('video')).toBeVisible()
    // The viewer dialog overlays the video player close button; dispatch click via JS
    await page
      .locator('video')
      .locator('..')
      .locator('button:has(.lucide-x)')
      .dispatchEvent('click')
    await expect(page).not.toHaveURL(/playing=/)
    await expect(page).toHaveURL(/viewing=/)
  })
})

async function createShare(page: Page, body: Record<string, unknown>): Promise<string> {
  const res = await page.request.post('/api/shares', { data: body })
  const json = await res.json()
  const base = `/share/${json.share.token}`
  return json.share.passcode ? `${base}?p=${encodeURIComponent(json.share.passcode)}` : base
}

let folderShareUrl: string

test.describe('URL State – Share Page', () => {
  test.beforeAll(async ({ browser }) => {
    const context = await browser.newContext({ storageState: authStoragePath })
    const page = await context.newPage()
    folderShareUrl = await createShare(page, {
      path: 'SharedContent',
      isDirectory: true,
    })
    await page.close()
    await context.close()
  })

  test('viewing a file preserves playing param', async ({ page }) => {
    await page.goto(folderShareUrl)
    await expect(page.locator('table')).toBeVisible()
    await page.locator('table').getByText('public-video.mp4').click()
    await expect(page).toHaveURL(/playing=/)
    await page.locator('table').getByText('public-doc.txt').click()
    await expect(page).toHaveURL(/viewing=/)
    await expect(page).toHaveURL(/playing=/)
  })

  test('navigating to subfolder preserves playing param', async ({ page }) => {
    await page.goto(folderShareUrl)
    await expect(page.locator('table')).toBeVisible()
    await page.locator('table').getByText('public-video.mp4').click()
    await expect(page).toHaveURL(/playing=/)
    await page.locator('table').getByText('subfolder').first().click()
    await expect(page).toHaveURL(/dir=subfolder/)
    await expect(page).toHaveURL(/playing=/)
  })

  test('closing viewer preserves playing param', async ({ page }) => {
    await page.goto(folderShareUrl)
    await expect(page.locator('table')).toBeVisible()
    await page.locator('table').getByText('public-video.mp4').click()
    await expect(page).toHaveURL(/playing=/)
    await page.locator('table').getByText('public-doc.txt').click()
    await expect(page).toHaveURL(/viewing=/)
    await page.locator('button[title="Close"]').click()
    await expect(page).not.toHaveURL(/viewing=/)
    await expect(page).toHaveURL(/playing=/)
  })

  test('closing player preserves viewing param', async ({ page }) => {
    await page.goto(folderShareUrl)
    await expect(page.locator('table')).toBeVisible()
    await page.locator('table').getByText('public-video.mp4').click()
    await expect(page).toHaveURL(/playing=/)
    await expect(page.locator('video')).toBeVisible()
    await page.locator('table').getByText('public-doc.txt').click()
    await expect(page).toHaveURL(/viewing=/)
    await expect(page).toHaveURL(/playing=/)
    // The viewer dialog overlays the video player close button; dispatch click via JS
    await page
      .locator('video')
      .locator('..')
      .locator('button:has(.lucide-x)')
      .dispatchEvent('click')
    await expect(page).not.toHaveURL(/playing=/)
    await expect(page).toHaveURL(/viewing=/)
  })
})
