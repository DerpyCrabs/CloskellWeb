import { test, expect, type BrowserContext, type Page } from '@playwright/test'
import {
  createTempFile,
  deleteFileViaContextMenu,
  dragFromTo,
  dragToEdge,
  getDragHandle,
  getVisibleContent,
  getWindowGroups,
  gotoWorkspace,
  html5DragDrop,
  navigateToSharedContent,
  openBrowserWindow,
} from '../e2e/workspace-cross-dnd-helpers'
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

test.describe('Cross-Window File Move', () => {
  test('drags a file from one browser into a folder in another', async () => {
    await gotoWorkspace(page)
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

    const tempFile = 'cross-dnd-test.txt'
    await createTempFile(page, contentA, tempFile)

    const sourceRow = contentA.locator('tr').filter({ hasText: tempFile })
    const targetRow = contentB.locator('tr').filter({ hasText: 'subfolder' }).first()

    await html5DragDrop(sourceRow, targetRow)

    await expect(contentA.getByText(tempFile)).not.toBeVisible({ timeout: 5_000 })

    await contentB.getByText('subfolder').first().click()
    await expect(contentB.getByText(tempFile)).toBeVisible({ timeout: 5_000 })

    await deleteFileViaContextMenu(page, contentB, tempFile)
  })

  test('non-editable files stay draggable for tab bar (copy) even when move is disallowed', async () => {
    await gotoWorkspace(page)

    const groups = getWindowGroups(page)
    const content = getVisibleContent(groups.first())

    await content.getByText('Documents', { exact: true }).click()
    await expect(content.getByText('readme.txt')).toBeVisible({ timeout: 5_000 })

    const readmeRow = content.locator('tr').filter({ hasText: 'readme.txt' })
    const draggable = await readmeRow.getAttribute('draggable')

    expect(draggable).toBe('true')
  })
})

test.describe('Drop File onto Tab Bar', () => {
  test('dropping a folder onto the tab bar opens it as a new browser tab', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)
    await dragToEdge(page, getDragHandle(groups.first()), 'left')
    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)
    await dragToEdge(page, getDragHandle(groups.nth(1)), 'right')

    const contentA = getVisibleContent(groups.first())
    await expect(contentA.getByText('SharedContent', { exact: true })).toBeVisible()
    await navigateToSharedContent(contentA)

    const folderRow = contentA.locator('tr').filter({ hasText: 'subfolder' }).first()
    const headerB = groups.nth(1).locator('[data-tab-drop-slot]').first()

    await html5DragDrop(folderRow, headerB)
    await page.waitForTimeout(150)

    const tabStrip = groups.nth(1).locator('.workspace-tab-strip')
    await expect(tabStrip).toBeVisible({ timeout: 5_000 })
    await expect(tabStrip.getByText('subfolder', { exact: true })).toBeVisible()
  })

  test('dropping a file onto the tab bar opens a viewer tab', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)
    await dragToEdge(page, getDragHandle(groups.first()), 'left')
    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)
    await dragToEdge(page, getDragHandle(groups.nth(1)), 'right')

    const contentA = getVisibleContent(groups.first())
    await navigateToSharedContent(contentA)

    const fileRow = contentA.locator('tr').filter({ hasText: 'public-doc.txt' })
    const headerB = groups.nth(1).locator('[data-tab-drop-slot]').first()

    await html5DragDrop(fileRow, headerB)
    await page.waitForTimeout(150)

    const tabStrip = groups.nth(1).locator('.workspace-tab-strip')
    await expect(tabStrip).toBeVisible({ timeout: 5_000 })
    await expect(tabStrip.getByText('public-doc.txt', { exact: true })).toBeVisible()
  })

  test('dropping a video onto the tab bar shows video in viewer tab', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)
    await dragToEdge(page, getDragHandle(groups.first()), 'left')
    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)
    await dragToEdge(page, getDragHandle(groups.nth(1)), 'right')

    const contentA = getVisibleContent(groups.first())
    await navigateToSharedContent(contentA)

    const videoRow = contentA.locator('tr').filter({ hasText: 'public-video.mp4' })
    if ((await videoRow.count()) === 0) {
      test.skip()
      return
    }

    const headerB = groups.nth(1).locator('[data-tab-drop-slot]').first()
    await html5DragDrop(videoRow.first(), headerB)
    await page.waitForTimeout(200)

    const targetGroup = groups.nth(1)
    await expect(targetGroup.locator('video')).toBeVisible({ timeout: 10_000 })
    await expect(targetGroup.locator('.workspace-tab-strip')).toBeVisible()
    await expect(
      targetGroup.locator('.workspace-tab-strip').getByText('public-video.mp4', { exact: true }),
    ).toBeVisible()
  })
})

test.describe('Window Merge Still Works', () => {
  test('dragging window title bar onto another still merges them', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)
    await expect(groups).toHaveCount(2)

    const handleB = getDragHandle(groups.nth(1))
    const boxB = await handleB.boundingBox()
    const boxA = await getDragHandle(groups.first()).boundingBox()
    if (!boxB || !boxA) throw new Error('Handles not visible')

    await dragFromTo(
      page,
      boxB.x + boxB.width / 2,
      boxB.y + boxB.height / 2,
      boxA.x + boxA.width / 2,
      boxA.y + 16,
    )
    await page.waitForTimeout(100)

    await expect(getWindowGroups(page)).toHaveCount(1)
    const tabStrip = page.locator('.workspace-tab-strip')
    await expect(tabStrip).toBeVisible()
  })
})
