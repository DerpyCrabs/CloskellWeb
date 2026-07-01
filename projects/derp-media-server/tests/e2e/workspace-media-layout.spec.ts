import { test, expect, type BrowserContext, type Locator, type Page } from '@playwright/test'
import { getWindowGroups, gotoWorkspace } from './workspace-layout-helpers'
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

function getVisibleContent(group: Locator) {
  return group.locator('[data-testid="workspace-window-visible-content"]')
}

test.describe('Workspace audio and video playback', () => {
  test('clicking an audio file wires taskbar audio and playback can start', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const content = getVisibleContent(groups.first())
    await content.getByText('Music', { exact: true }).click()
    await expect(content.locator('table').getByText('track.mp3')).toBeVisible()
    await content.locator('table').getByText('track.mp3').click()
    await expect(page.getByRole('button', { name: 'Open audio controls' })).toBeVisible({
      timeout: 10_000,
    })
    const audio = page.locator('[data-workspace-taskbar-media-audio]')
    await expect(audio).toBeAttached()
    await expect
      .poll(async () => audio.evaluate((el: HTMLAudioElement) => el.currentSrc || el.src), {
        timeout: 15_000,
      })
      .toMatch(/\/api\/(media|share)/)

    await page.getByRole('button', { name: 'Open audio controls' }).click()
    const playToggle = page
      .locator('[data-workspace-taskbar-audio-root] .bg-popover')
      .locator('button')
      .filter({ has: page.locator('.lucide-play') })
      .first()
    if (await playToggle.isVisible()) {
      await playToggle.click()
    }
    await page.waitForFunction(
      () => {
        const el = document.querySelector(
          '[data-workspace-taskbar-media-audio]',
        ) as HTMLAudioElement | null
        return !!el && !el.paused
      },
      { timeout: 15_000 },
    )
  })

  test('taskbar repeat keeps playing after track ends', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const content = getVisibleContent(groups.first())
    await content.getByText('Music', { exact: true }).click()
    await expect(content.locator('table').getByText('track.mp3')).toBeVisible()
    await content.locator('table').getByText('track.mp3').click()
    await expect(page.getByRole('button', { name: 'Open audio controls' })).toBeVisible({
      timeout: 10_000,
    })
    const audio = page.locator('[data-workspace-taskbar-media-audio]')
    await expect
      .poll(async () => audio.evaluate((el: HTMLAudioElement) => el.currentSrc || el.src), {
        timeout: 15_000,
      })
      .toMatch(/\/api\/(media|share)/)

    await page.getByRole('button', { name: 'Open audio controls' }).click()
    const popover = page.locator('[data-workspace-taskbar-audio-root] .bg-popover')
    const playToggle = popover
      .locator('button')
      .filter({ has: page.locator('.lucide-play') })
      .first()
    if (await playToggle.isVisible()) {
      await playToggle.click()
    }
    await page.waitForFunction(
      () => {
        const el = document.querySelector(
          '[data-workspace-taskbar-media-audio]',
        ) as HTMLAudioElement | null
        return !!el && !el.paused
      },
      { timeout: 15_000 },
    )

    await popover.locator('button:has(.lucide-repeat)').click()

    const duration = await audio.evaluate((el: HTMLAudioElement) => el.duration)
    if (!Number.isFinite(duration) || duration <= 1) {
      throw new Error('fixture MP3 should have finite duration (needs ffmpeg-generated test media)')
    }
    await audio.evaluate((el: HTMLAudioElement) => {
      const d = el.duration
      el.currentTime = Math.max(0, d - 0.4)
    })

    await expect
      .poll(
        async () =>
          audio.evaluate((el: HTMLAudioElement) => {
            if (el.paused) return 'paused'
            return el.currentTime < 1 ? 'looped-playing' : 'waiting'
          }),
        { timeout: 12_000 },
      )
      .toBe('looped-playing')
  })

  // Regression: transport must reload <audio> after stop; also `!paused` alone is weak (CI Chromium
  // often allows deferred play(); real browsers / stale currentSrc may differ).
  test('after stopping taskbar audio, clicking the same file plays again', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const content = getVisibleContent(groups.first())
    await content.getByText('Music', { exact: true }).click()
    await expect(content.locator('table').getByText('track.mp3')).toBeVisible()
    await content.locator('table').getByText('track.mp3').click()
    await expect(page.getByRole('button', { name: 'Open audio controls' })).toBeVisible({
      timeout: 10_000,
    })
    const audio = page.locator('[data-workspace-taskbar-media-audio]')
    await expect
      .poll(async () => audio.evaluate((el: HTMLAudioElement) => el.currentSrc || el.src), {
        timeout: 15_000,
      })
      .toMatch(/\/api\/(media|share)/)

    await page.getByRole('button', { name: 'Open audio controls' }).click()
    const playToggle = page
      .locator('[data-workspace-taskbar-audio-root] .bg-popover')
      .locator('button')
      .filter({ has: page.locator('.lucide-play') })
      .first()
    if (await playToggle.isVisible()) {
      await playToggle.click()
    }
    await page.waitForFunction(
      () => {
        const el = document.querySelector(
          '[data-workspace-taskbar-media-audio]',
        ) as HTMLAudioElement | null
        return !!el && !el.paused
      },
      { timeout: 15_000 },
    )

    await page
      .locator('[data-workspace-taskbar-audio-root]')
      .getByRole('button', { name: 'Stop playback' })
      .click()
    await expect(page.getByRole('button', { name: 'Open audio controls' })).toBeHidden({
      timeout: 5_000,
    })

    await content.locator('table').getByText('track.mp3').click()
    await expect(page.getByRole('button', { name: 'Open audio controls' })).toBeVisible({
      timeout: 10_000,
    })
    await expect
      .poll(async () => audio.evaluate((el: HTMLAudioElement) => el.currentSrc || el.src), {
        timeout: 15_000,
      })
      .toMatch(/\/api\/(media|share)/)

    await page.waitForFunction(
      () => {
        const el = document.querySelector(
          '[data-workspace-taskbar-media-audio]',
        ) as HTMLAudioElement | null
        return !!el && !el.paused
      },
      { timeout: 15_000 },
    )
    await expect
      .poll(async () => audio.evaluate((el: HTMLAudioElement) => el.currentTime), {
        timeout: 10_000,
      })
      .toBeGreaterThan(0.25)
  })

  test('video element fills most of the player window', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const content = getVisibleContent(groups.first())
    await content.getByText('Videos', { exact: true }).click()
    await expect(content.getByText('sample.mp4')).toBeVisible()
    await content.getByText('sample.mp4').click()
    const playerGroup = groups.nth(1)
    await expect(playerGroup.locator('video')).toBeVisible({ timeout: 10_000 })
    await expect
      .poll(async () => (await playerGroup.boundingBox())?.height ?? 0, { timeout: 15_000 })
      .toBeGreaterThan(320)
    const winBox = await playerGroup.boundingBox()
    expect(winBox).toBeTruthy()
    const titleChrome = 40
    const innerH = winBox!.height - titleChrome
    const mediaAreaH = await playerGroup
      .locator('video')
      .evaluate((v) => (v.parentElement as HTMLElement | null)?.getBoundingClientRect().height ?? 0)
    expect(mediaAreaH).toBeGreaterThan(innerH * 0.5)
  })
})

test.describe('Workspace viewer pane height', () => {
  test('text viewer body region fills most of window below toolbar', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const content = getVisibleContent(groups.first())
    await content.getByText('Documents', { exact: true }).click()
    await expect(content.getByText('readme.txt')).toBeVisible()
    await content.getByText('readme.txt').click()
    await expect(groups).toHaveCount(2)
    const viewer = groups.nth(1)
    const visible = viewer.locator('[data-testid="workspace-window-visible-content"]')
    const pre = visible.locator('pre')
    await expect(pre).toBeVisible({ timeout: 10_000 })
    const bodyHeight = await pre.evaluate((el) => {
      let n: HTMLElement | null = el.parentElement
      while (n) {
        const c = n.className
        if (typeof c === 'string' && c.includes('flex-1') && c.includes('overflow-hidden')) {
          return n.getBoundingClientRect().height
        }
        n = n.parentElement
      }
      return 0
    })
    const winBox = await viewer.boundingBox()
    expect(winBox).toBeTruthy()
    const chrome = 44
    const innerH = winBox!.height - chrome
    expect(bodyHeight).toBeGreaterThan(innerH * 0.45)
  })
})
