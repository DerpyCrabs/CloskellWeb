import { test, expect, type BrowserContext, type Page } from '@playwright/test'
import {
  TASKBAR_HEIGHT,
  gotoWorkspace,
  openBrowserWindow,
  getWindowGroups,
  getRndWrapper,
  getDragHandle,
  getWindowBounds,
  waitForWindowBoundsStable,
  dragFromTo,
  assistMiniGrid,
  assistMiniGridCell,
  dragToEdge,
  presetAssistGridShapeBeforeGoto,
  WORKSPACE_VISIBLE_WINDOW_GROUP,
} from '../e2e/workspace-layout-helpers'
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

test.describe('Tiling Layout Picker', () => {
  test('selects full-screen layout', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click()
    await waitForWindowBoundsStable(page, groups.first())

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT

    expect(bounds.width).toBeGreaterThan(viewport.width - 10)
    expect(bounds.height).toBeGreaterThan(containerH - 10)
  })

  test('selects left column tile on default 3×2 grid', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()

    await assistMiniGrid(page, '3x2').getByTestId('snap-assist-master-cell').click()
    await waitForWindowBoundsStable(page, groups.first())

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const thirdW = Math.round(viewport.width / 3)
    const containerH = viewport.height - TASKBAR_HEIGHT
    const halfH = Math.round(containerH / 2)

    expect(bounds.x).toBeLessThanOrEqual(2)
    expect(bounds.y).toBeLessThanOrEqual(2)
    expect(bounds.width).toBeGreaterThan(thirdW - 24)
    expect(bounds.width).toBeLessThan(thirdW + 24)
    expect(bounds.height).toBeGreaterThan(halfH - 24)
    expect(bounds.height).toBeLessThan(halfH + 24)
  })

  test('tiling picker hover highlight moves from first quarter cell to second', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()

    const mini = assistMiniGrid(page, '2x2')
    const leftTop = assistMiniGridCell(mini, 0, 0, 0, 0)
    const rightTop = assistMiniGridCell(mini, 1, 1, 0, 0)
    await expect(leftTop).toBeVisible()
    await expect(rightTop).toBeVisible()

    const lbox = await leftTop.boundingBox()
    const rbox = await rightTop.boundingBox()
    if (!lbox || !rbox) throw new Error('Mini grid cells not laid out')

    const workspacePreview = page.locator('[data-snap-preview]')

    await page.mouse.move(lbox.x + lbox.width / 2, lbox.y + lbox.height / 2)
    await expect(leftTop).toHaveAttribute('data-snap-assist-hover-active', '')
    await expect(rightTop).not.toHaveAttribute('data-snap-assist-hover-active', '')
    await expect(workspacePreview).toBeVisible()
    const previewBoxLeft = await workspacePreview.boundingBox()
    if (!previewBoxLeft) throw new Error('Workspace layout preview not laid out')
    expect(previewBoxLeft.x).toBeLessThanOrEqual(8)
    expect(previewBoxLeft.width).toBeGreaterThan(64)

    await page.mouse.move(rbox.x + rbox.width / 2, rbox.y + rbox.height / 2, { steps: 12 })
    await expect(rightTop).toHaveAttribute('data-snap-assist-hover-active', '')
    await expect(leftTop).not.toHaveAttribute('data-snap-assist-hover-active', '')
    await expect(workspacePreview).toBeVisible()
    const previewBoxRight = await workspacePreview.boundingBox()
    if (!previewBoxRight) throw new Error('Workspace layout preview not laid out after move')
    expect(previewBoxRight.x).toBeGreaterThan(previewBoxLeft.x + 8)
  })

  test('selects quarter layout via 2×2 grid', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()

    await assistMiniGrid(page, '2x2').getByTestId('snap-assist-master-cell').click()
    await waitForWindowBoundsStable(page, groups.first())

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const halfW = Math.round(viewport.width / 2)
    const containerH = viewport.height - TASKBAR_HEIGHT
    const halfH = Math.round(containerH / 2)

    expect(bounds.x).toBeLessThanOrEqual(2)
    expect(bounds.y).toBeLessThanOrEqual(2)
    expect(bounds.width).toBeGreaterThan(halfW - 20)
    expect(bounds.width).toBeLessThan(halfW + 20)
    expect(bounds.height).toBeGreaterThan(halfH - 20)
    expect(bounds.height).toBeLessThan(halfH + 20)
  })

  test('picker closes on escape', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()

    await page.keyboard.press('Escape')

    await expect(page.locator('[data-tiling-picker]')).not.toBeVisible()
  })

  test('picker closes on outside click', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()

    const content = groups.first().locator('.workspace-window-content')
    await content.click({ position: { x: 10, y: 10 } })

    await expect(page.locator('[data-tiling-picker]')).not.toBeVisible()
  })
})

test.describe('Drag Restore', () => {
  test('dragging a snapped window restores its pre-snap size', async () => {
    await presetAssistGridShapeBeforeGoto(page, '3x2')
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)

    const preBounds = await getWindowBounds(groups.first())

    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'left')

    const snappedBounds = await getWindowBounds(groups.first())
    const thirdW = Math.round(page.viewportSize()!.width / 3)
    expect(snappedBounds.width).toBeGreaterThan(thirdW - 40)
    expect(snappedBounds.width).toBeLessThan(thirdW + 40)

    const handle2 = getDragHandle(groups.first())
    const box = await handle2.boundingBox()
    if (!box) throw new Error('Handle not visible')

    await dragFromTo(
      page,
      box.x + box.width / 2,
      box.y + box.height / 2,
      page.viewportSize()!.width / 2,
      page.viewportSize()!.height / 3,
    )
    await waitForWindowBoundsStable(page, groups.first())

    const restoredBounds = await getWindowBounds(groups.first())
    expect(restoredBounds.width).toBeGreaterThan(preBounds.width - 50)
    expect(restoredBounds.width).toBeLessThan(preBounds.width + 50)
  })

  test('dragging a maximized window restores it', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const viewport = page.viewportSize()!

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click()
    await waitForWindowBoundsStable(page, groups.first())

    const maxBounds = await getWindowBounds(groups.first())
    expect(maxBounds.width).toBeGreaterThan(viewport.width - 10)

    const handle = getDragHandle(groups.first())
    const box = await handle.boundingBox()
    if (!box) throw new Error('Handle not visible')
    await dragFromTo(
      page,
      box.x + box.width / 2,
      box.y + box.height / 2,
      viewport.width / 2,
      viewport.height / 3,
    )
    await waitForWindowBoundsStable(page, groups.first())

    const restoredBounds = await getWindowBounds(groups.first())
    expect(restoredBounds.width).toBeLessThan(viewport.width - 50)
  })

  test('restored window follows cursor position', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)

    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'left')

    const handle2 = getDragHandle(groups.first())
    const box = await handle2.boundingBox()
    if (!box) throw new Error('Handle not visible')

    const dropX = 600
    const dropY = 200
    await dragFromTo(page, box.x + box.width / 2, box.y + box.height / 2, dropX, dropY)
    await waitForWindowBoundsStable(page, groups.first())

    const restoredBounds = await getWindowBounds(groups.first())
    expect(restoredBounds.x).toBeGreaterThan(dropX - restoredBounds.width)
    expect(restoredBounds.x).toBeLessThan(dropX + 10)
  })
})

test.describe('Window Minimum Size', () => {
  test('window cannot be resized below 360x260', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)

    const rnd = getRndWrapper(groups.first())
    const box = await rnd.boundingBox()
    if (!box) throw new Error('Window not visible')

    await dragFromTo(page, box.x + box.width, box.y + box.height, box.x + 100, box.y + 100, 20)
    await waitForWindowBoundsStable(page, groups.first())

    const newBounds = await getWindowBounds(groups.first())
    expect(newBounds.width).toBeGreaterThanOrEqual(360)
    expect(newBounds.height).toBeGreaterThanOrEqual(260)
  })
})

test.describe('Vertical viewport (portrait)', () => {
  test.beforeEach(async () => {
    await page.setViewportSize({ width: 700, height: 1100 })
    await presetAssistGridShapeBeforeGoto(page, '3x2')
    await gotoWorkspace(page)
  })

  test('default window uses most of width and is not slim', async () => {
    const groups = getWindowGroups(page)
    await expect(groups).toHaveCount(1)

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const minExpectedWidth = viewport.width * 0.8

    expect(bounds.width).toBeGreaterThanOrEqual(minExpectedWidth)
    expect(bounds.height).toBeLessThan(viewport.height - TASKBAR_HEIGHT)
  })

  test('layout picker shows four mini grids', async () => {
    const groups = getWindowGroups(page)

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()

    await expect(page.locator('[data-assist-mini-grid]')).toHaveCount(4)
  })

  test('snapping to top-half via picker fills top half of viewport', async () => {
    const groups = getWindowGroups(page)

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()

    await assistMiniGrid(page, '2x2').getByTestId('snap-assist-vgutter-two-cols-top').click()
    await waitForWindowBoundsStable(page, groups.first())

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT
    const halfH = Math.round(containerH / 2)

    expect(bounds.x).toBeLessThanOrEqual(2)
    expect(bounds.y).toBeLessThanOrEqual(2)
    expect(bounds.width).toBeGreaterThan(viewport.width - 10)
    expect(bounds.height).toBeGreaterThan(halfH - 20)
    expect(bounds.height).toBeLessThan(halfH + 20)
  })

  test('snapping to bottom-half via picker fills bottom half of viewport', async () => {
    const groups = getWindowGroups(page)

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()

    await assistMiniGrid(page, '2x2')
      .locator(
        '[data-assist-master-grid] button[data-grid-cols="2"][data-gc0="0"][data-gc1="1"][data-gr0="1"][data-gr1="1"]',
      )
      .click()
    await waitForWindowBoundsStable(page, groups.first())

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT
    const halfH = Math.round(containerH / 2)

    expect(bounds.x).toBeLessThanOrEqual(2)
    expect(bounds.y).toBeGreaterThan(halfH - 20)
    expect(bounds.y).toBeLessThan(halfH + 20)
    expect(bounds.width).toBeGreaterThan(viewport.width - 10)
    expect(bounds.height).toBeGreaterThan(halfH - 20)
  })

  test('dragging to top edge (off center) snaps merged pair of top-row tiles on 3×2', async () => {
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'top-half')

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT
    const twoThirdsW = Math.round((viewport.width * 2) / 3)
    const halfH = Math.round(containerH / 2)

    expect(bounds.y).toBeLessThanOrEqual(2)
    expect(bounds.width).toBeGreaterThan(twoThirdsW - 28)
    expect(bounds.width).toBeLessThan(twoThirdsW + 28)
    expect(bounds.height).toBeGreaterThan(halfH - 24)
    expect(bounds.height).toBeLessThan(halfH + 24)
  })

  test('dragging to bottom edge snaps to bottom-row tile on 3×2', async () => {
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'bottom-half')

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT
    const thirdW = Math.round(viewport.width / 3)
    const halfH = Math.round(containerH / 2)

    expect(bounds.y).toBeGreaterThan(halfH - 24)
    expect(bounds.y).toBeLessThan(halfH + 24)
    expect(bounds.width).toBeGreaterThan(thirdW - 24)
    expect(bounds.width).toBeLessThan(thirdW + 24)
    expect(bounds.height).toBeGreaterThan(halfH - 24)
  })

  test('dragging to top edge off assist band snaps to first top-row tile in portrait', async () => {
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'top')

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT
    const thirdW = Math.round(viewport.width / 3)
    const halfH = Math.round(containerH / 2)

    expect(bounds.y).toBeLessThanOrEqual(2)
    expect(bounds.width).toBeGreaterThan(thirdW - 30)
    expect(bounds.width).toBeLessThan(thirdW + 30)
    expect(bounds.height).toBeGreaterThan(halfH - 30)
    expect(bounds.height).toBeLessThan(halfH + 30)
  })
})

test.describe('Window Z-Ordering and Focus', () => {
  test('clicking background window brings it to front', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)
    const groups = getWindowGroups(page)

    const rndA = getRndWrapper(groups.first())
    const rndB = getRndWrapper(groups.nth(1))
    const getZ = (el: HTMLElement) => {
      const own = parseInt(el.style?.zIndex || '', 10)
      if (!Number.isNaN(own) && own !== 0) return own
      const p = el.parentElement as HTMLElement | null
      return parseInt(p?.style?.zIndex || '0', 10) || 0
    }

    const zB = await rndB.evaluate(getZ)
    const zA = await rndA.evaluate(getZ)
    expect(zB).toBeGreaterThan(zA)

    await groups.first().dispatchEvent('mousedown')
    await page.waitForTimeout(50)

    const newZA = await rndA.evaluate(getZ)
    expect(newZA).toBeGreaterThan(zB)
  })

  test('clicking window content brings it to front', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)
    const groups = getWindowGroups(page)

    const rndA = getRndWrapper(groups.first())
    const rndB = getRndWrapper(groups.nth(1))
    const getZ = (el: HTMLElement) => {
      const own = parseInt(el.style?.zIndex || '', 10)
      if (!Number.isNaN(own) && own !== 0) return own
      const p = el.parentElement as HTMLElement | null
      return parseInt(p?.style?.zIndex || '0', 10) || 0
    }

    const zB = await rndB.evaluate(getZ)
    const zA = await rndA.evaluate(getZ)
    expect(zB).toBeGreaterThan(zA)

    const contentA = groups.first().getByTestId('workspace-chrome-content')
    await contentA.click({ position: { x: 10, y: 50 }, force: true })
    await page.waitForTimeout(50)

    const newZA = await rndA.evaluate(getZ)
    expect(newZA).toBeGreaterThan(zB)
  })

  test('newly opened window is focused', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)
    const groups = getWindowGroups(page)

    await expect(groups.nth(1)).toHaveClass(/shadow-black\/20/)
  })

  test('active window has distinct border', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    await expect(groups.first()).toHaveClass(/border-border/)

    await openBrowserWindow(page)
    const updatedGroups = getWindowGroups(page)
    await expect(updatedGroups.first()).not.toHaveClass(/shadow-black\/20/)
    await expect(updatedGroups.nth(1)).toHaveClass(/shadow-black\/20/)
  })
})

test.describe('State Persistence', () => {
  test('windows survive page reload', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)
    await expect(getWindowGroups(page)).toHaveCount(2)

    // Wait for workspace state to persist before reload (allow debounce/save to complete)
    await page.waitForTimeout(1500)
    await page.reload()
    await page.waitForLoadState('domcontentloaded')
    await expect(getWindowGroups(page).first()).toBeVisible()

    await expect(getWindowGroups(page)).toHaveCount(2)
  })

  test('snap state persists across reload', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'left')
    await page.waitForTimeout(600)

    const snappedBounds = await getWindowBounds(groups.first())

    await page.reload()
    await page.waitForLoadState('domcontentloaded')
    await expect(getWindowGroups(page).first()).toBeVisible()

    const reloadedBounds = await getWindowBounds(getWindowGroups(page).first())
    expect(reloadedBounds.x).toBeLessThanOrEqual(2)
    expect(reloadedBounds.width).toBeGreaterThan(snappedBounds.width - 20)
    expect(reloadedBounds.width).toBeLessThan(snappedBounds.width + 20)
  })

  test('tab groups persist across reload', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)
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
    await page.waitForTimeout(600)
    await expect(getWindowGroups(page)).toHaveCount(1)

    await page.reload()
    await page.waitForLoadState('domcontentloaded')
    await expect(getWindowGroups(page).first()).toBeVisible()

    await expect(getWindowGroups(page)).toHaveCount(1)
    const tabStrip = page.locator('.workspace-tab-strip')
    await expect(tabStrip).toBeVisible()
  })

  test('video viewer tab is restored after reload', async () => {
    await gotoWorkspace(page)

    const groups = getWindowGroups(page)
    const content = groups.first().getByTestId('workspace-window-visible-content')
    await content.getByText('Videos', { exact: true }).click()
    await page.waitForTimeout(300)
    await content.getByText('sample.mp4').click()
    await page.waitForTimeout(600)

    const allGroups = getWindowGroups(page)
    const count = await allGroups.count()
    expect(count).toBeGreaterThanOrEqual(2)

    await page.reload()
    await page.waitForLoadState('domcontentloaded')
    await expect(getWindowGroups(page).first()).toBeVisible()

    const reloadedVideo = page.locator(`${WORKSPACE_VISIBLE_WINDOW_GROUP} video`)
    await expect(reloadedVideo).toBeVisible({ timeout: 10_000 })
  })
})
