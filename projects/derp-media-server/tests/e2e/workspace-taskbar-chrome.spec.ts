import { test, expect, type BrowserContext, type Page } from '@playwright/test'
import { WORKSPACE_VISIBLE_WINDOW_GROUP, gotoWorkspace } from './workspace-layout-helpers'
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

function getBrowserContent(page: Page) {
  return page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP).first().locator('.workspace-window-content')
}

test.describe('Workspace taskbar chrome', () => {
  test('taskbar audio shows current track after playing mp3 from browser', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await content.getByText('Music', { exact: true }).click()
    await content.locator('table').getByText('track.mp3').click()
    const audioControls = page.getByRole('button', { name: 'Open audio controls' })
    await expect(audioControls).toBeVisible()
    await expect(audioControls).toContainText('track.mp3')
  })

  test('persists dark mode from workspace taskbar settings after reload', async () => {
    await gotoWorkspace(page)
    await page.getByRole('button', { name: 'Open settings' }).click()
    const dialog = page.getByRole('dialog', { name: 'Settings' })
    await expect(dialog).toBeVisible()
    await dialog.getByRole('button', { name: 'Dark' }).click()
    await expect(page.locator('html')).toHaveAttribute('data-theme', /dark/)
    await page.reload()
    await expect(page.locator('html')).toHaveAttribute('data-theme', /dark/)
  })

  test('taskbar Show video restores workspace video after listen-only mode', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await content.getByText('Videos', { exact: true }).click()
    await expect(content.getByText('sample.mp4')).toBeVisible()
    await content.getByText('sample.mp4').click()

    const videos = page.locator(`${WORKSPACE_VISIBLE_WINDOW_GROUP} video`)
    await expect(videos).toHaveCount(1)
    await videos.first().hover()
    await page.locator('button[title="Listen only"]').click()
    await expect(videos).toHaveCount(0)

    await page.getByRole('button', { name: 'Open audio controls' }).click()
    await page
      .locator('[data-workspace-taskbar-audio-root]')
      .getByRole('button', { name: 'Show video' })
      .click()
    await expect(page.locator(`${WORKSPACE_VISIBLE_WINDOW_GROUP} video`)).toHaveCount(1)
  })

  test('workspace: listen-only ↔ show video round-trip twice', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await content.getByText('Videos', { exact: true }).click()
    await content.getByText('sample.mp4').click()

    const videos = page.locator(`${WORKSPACE_VISIBLE_WINDOW_GROUP} video`)
    const audioRoot = page.locator('[data-workspace-taskbar-audio-root]')

    for (let i = 0; i < 2; i++) {
      await expect(videos).toHaveCount(1)
      await videos.first().hover()
      await page.locator('button[title="Listen only"]').click()
      await expect(videos).toHaveCount(0)
      await expect(page.getByRole('button', { name: 'Open audio controls' })).toBeVisible()

      await page.getByRole('button', { name: 'Open audio controls' }).click()
      await audioRoot.getByRole('button', { name: 'Show video' }).click()
      await expect(videos).toHaveCount(1)
    }
  })

  test('snap assist toggle persists after reload', async () => {
    await gotoWorkspace(page)
    await page.getByRole('button', { name: 'Open settings' }).click()
    const settingsDialog = page.getByRole('dialog', { name: 'Settings' })
    await expect(settingsDialog).toBeVisible()
    const assistLabel = settingsDialog.getByText(
      'Show snap assist when dragging to the top-center strip (~300px wide)',
    )
    const assistCheckbox = assistLabel.locator('..').locator('input[type="checkbox"]')
    await assistCheckbox.setChecked(false)
    await page.keyboard.press('Escape')
    await expect(settingsDialog).not.toBeVisible()

    await page.reload()
    await page.waitForLoadState('domcontentloaded')
    await expect(page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP).first()).toBeVisible()

    await page.getByRole('button', { name: 'Open settings' }).click()
    const dialogAfter = page.getByRole('dialog', { name: 'Settings' })
    await expect(dialogAfter).toBeVisible()
    const assistLabelAfter = dialogAfter.getByText(
      'Show snap assist when dragging to the top-center strip (~300px wide)',
    )
    await expect(assistLabelAfter.locator('..').locator('input[type="checkbox"]')).not.toBeChecked()

    await assistLabelAfter.locator('..').locator('input[type="checkbox"]').setChecked(true)
    await page.keyboard.press('Escape')
  })
})
