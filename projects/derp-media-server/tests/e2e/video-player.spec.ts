import { test, expect } from '@playwright/test'

const VIDEO_DIR = 'Videos'
const VIDEO_FILE = 'Videos/sample.mp4'

test.describe('Video Player', () => {
  test('opens video player when clicking a video file', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}`)
    await page.locator('table').getByText('sample.mp4').click()
    await page.waitForURL(/playing=/)
    await expect(page.locator('video')).toBeVisible()
  })

  test('video element has native controls', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}&playing=${encodeURIComponent(VIDEO_FILE)}`)
    const video = page.locator('video')
    await expect(video).toBeVisible()
    await expect(video).toHaveAttribute('controls', '')
  })

  test('shows audio-only mode toggle', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}&playing=${encodeURIComponent(VIDEO_FILE)}`)
    await expect(page.locator('button[aria-label="Audio only mode"]')).toBeVisible()
  })

  test('switches to audio-only mode', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}&playing=${encodeURIComponent(VIDEO_FILE)}`)
    await page.locator('button[aria-label="Audio only mode"]').click()
    await page.waitForURL(/audioOnly=true/)
    await expect(page.locator('video')).not.toBeVisible()
  })

  test('plays audio when switching to audio-only mode', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}&playing=${encodeURIComponent(VIDEO_FILE)}`)
    await page.locator('button[aria-label="Audio only mode"]').click()
    await page.waitForURL(/audioOnly=true/)
    const audio = page.locator('audio').first()
    await expect(audio).toBeAttached()
    await expect.poll(async () => audio.evaluate((el: HTMLAudioElement) => !el.paused)).toBe(true)
  })

  test('closes video player', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}&playing=${encodeURIComponent(VIDEO_FILE)}`)
    await expect(page.locator('video')).toBeVisible()
    await page.getByRole('button', { name: 'Close player' }).click()
    await expect(page.locator('video')).not.toBeVisible()
    await expect(page).not.toHaveURL(/playing=/)
  })

  test('minimizes video player', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}&playing=${encodeURIComponent(VIDEO_FILE)}`)
    await expect(page.locator('video')).toBeVisible()
    await page.getByRole('button', { name: 'Minimize player' }).click()
    await expect(page.locator('video')).toBeVisible()
  })

  test('reflects playing file in URL', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}`)
    await page.locator('table').getByText('sample.mp4').click()
    await expect(page).toHaveURL(/playing=/)
  })

  test('video loads with a valid source', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}&playing=${encodeURIComponent(VIDEO_FILE)}`)
    const video = page.locator('video')
    await expect(video).toBeVisible()
    await expect(video).toHaveAttribute('src', /\/api\/media\//)
  })

  test('video thumbnails appear in grid view', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}`)
    await page.locator('button:has(.lucide-layout-grid)').click()
    const card = page
      .locator('[data-testid=file-browser] .file-browser-grid [role=button]')
      .filter({
        hasText: 'sample.mp4',
      })
    const thumb = card.locator('[data-testid=file-browser-video-thumbnail]')
    await expect(thumb).toBeVisible()
    await expect(thumb).toHaveAttribute('src', /\/api\/thumbnail\//)
    await expect
      .poll(async () => thumb.evaluate((el: HTMLImageElement) => el.naturalWidth))
      .toBeGreaterThan(0)
  })

  test('maximize restores from minimized state', async ({ page }) => {
    await page.goto(`/?dir=${VIDEO_DIR}&playing=${encodeURIComponent(VIDEO_FILE)}`)
    await page.getByRole('button', { name: 'Minimize player' }).click()
    await page.getByRole('button', { name: 'Maximize player' }).click()
    await expect(page.locator('video')).toBeVisible()
  })
})
