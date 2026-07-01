import { test, expect, type BrowserContext, type Page } from '@playwright/test'
import {
  TASKBAR_HEIGHT,
  gotoWorkspace,
  openBrowserWindow,
  getWindowGroups,
  getRndWrapper,
  getDragHandle,
  getWindowBounds,
  getSharedColumnResizeHandle,
  waitForWindowBoundsStable,
  dragFromTo,
  assistMiniGrid,
  assistMiniGridCell,
  dragToEdge,
  setAssistGridShapeForTest,
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

test.describe('Edge Snapping', () => {
  test('snaps window to left column on default 3×2 grid', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    await expect(groups).toHaveCount(1)

    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'left')

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const thirdW = Math.round(viewport.width / 3)

    expect(bounds.x).toBeLessThanOrEqual(2)
    expect(bounds.width).toBeGreaterThan(thirdW - 24)
    expect(bounds.width).toBeLessThan(thirdW + 24)
  })

  test('snaps window to right column on default 3×2 grid', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'right')

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const thirdW = Math.round(viewport.width / 3)

    expect(bounds.x).toBeGreaterThan(thirdW * 2 - 30)
    expect(bounds.width).toBeGreaterThan(thirdW - 24)
  })

  test('snaps window to top-left tile on default 3×2 grid', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'top-left')

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

  test('snaps window to top-right tile on default 3×2 grid', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'top-right')

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const thirdW = Math.round(viewport.width / 3)
    const containerH = viewport.height - TASKBAR_HEIGHT
    const halfH = Math.round(containerH / 2)

    expect(bounds.x).toBeGreaterThan(thirdW * 2 - 30)
    expect(bounds.y).toBeLessThanOrEqual(2)
    expect(bounds.width).toBeGreaterThan(thirdW - 24)
    expect(bounds.height).toBeGreaterThan(halfH - 24)
    expect(bounds.height).toBeLessThan(halfH + 24)
  })

  test('snaps window to bottom-left tile on default 3×2 grid', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'bottom-left')

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const thirdW = Math.round(viewport.width / 3)
    const containerH = viewport.height - TASKBAR_HEIGHT
    const halfH = Math.round(containerH / 2)

    expect(bounds.x).toBeLessThanOrEqual(2)
    expect(bounds.width).toBeGreaterThan(thirdW - 24)
    expect(bounds.width).toBeLessThan(thirdW + 24)
    expect(bounds.y).toBeGreaterThan(halfH - 24)
    expect(bounds.y).toBeLessThan(halfH + 24)
  })

  test('snaps window to bottom-right tile on default 3×2 grid', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'bottom-right')

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const thirdW = Math.round(viewport.width / 3)
    const containerH = viewport.height - TASKBAR_HEIGHT
    const halfH = Math.round(containerH / 2)

    expect(bounds.x).toBeGreaterThan(thirdW * 2 - 30)
    expect(bounds.y).toBeGreaterThan(halfH - 24)
    expect(bounds.width).toBeGreaterThan(thirdW - 24)
  })

  test('dragging to top edge off assist band snaps to first top-row tile', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'top')

    const bounds = await getWindowBounds(groups.first())
    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT
    const thirdW = Math.round(viewport.width / 3)
    const halfH = Math.round(containerH / 2)

    expect(bounds.y).toBeLessThanOrEqual(2)
    expect(bounds.height).toBeGreaterThan(halfH - 30)
    expect(bounds.height).toBeLessThan(halfH + 30)
    expect(bounds.width).toBeGreaterThan(thirdW - 30)
    expect(bounds.width).toBeLessThan(thirdW + 30)
  })

  test('minimize button works on first click after snapping window', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    await expect(groups).toHaveCount(1)

    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'left')

    const minimizeBtn = groups.first().locator('button:has(.lucide-minus)')
    await minimizeBtn.click()

    await expect(getWindowGroups(page)).toHaveCount(0)
  })

  test('shows snap preview while dragging near edge', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())

    const box = await handle.boundingBox()
    if (!box) throw new Error('Handle not visible')
    const startX = box.x + box.width / 2
    const startY = box.y + box.height / 2

    await page.mouse.move(startX, startY)
    await page.mouse.down()
    await page.mouse.move(5, page.viewportSize()!.height / 2, { steps: 10 })

    const preview = page.locator('[data-snap-preview]')
    await expect(preview).toHaveCSS('display', 'block')

    await page.mouse.up()
  })

  test('restores window from snapped state when dragged away', async () => {
    await gotoWorkspace(page)
    await setAssistGridShapeForTest(page, '3x2')
    const groups = getWindowGroups(page)

    const preBounds = await getWindowBounds(groups.first())

    const handle = getDragHandle(groups.first())
    await dragToEdge(page, handle, 'left')

    const snappedBounds = await getWindowBounds(groups.first())
    expect(snappedBounds.width).not.toBeCloseTo(preBounds.width, -1)

    const handle2 = getDragHandle(groups.first())
    const snappedBox = await handle2.boundingBox()
    if (!snappedBox) throw new Error('Handle not visible')
    await dragFromTo(
      page,
      snappedBox.x + snappedBox.width / 2,
      snappedBox.y + snappedBox.height / 2,
      page.viewportSize()!.width / 2,
      page.viewportSize()!.height / 3,
    )
    await waitForWindowBoundsStable(page, groups.first())

    const restoredBounds = await getWindowBounds(groups.first())
    expect(restoredBounds.width).toBeGreaterThanOrEqual(snappedBounds.width - 8)
    expect(Math.abs(restoredBounds.width - preBounds.width)).toBeLessThan(120)
  })
})

test.describe('Snap assist bar', () => {
  test('hover highlight moves from left 3×2 cell to center top cell while dragging', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    const hbox = await handle.boundingBox()
    if (!hbox) throw new Error('Handle not visible')

    const viewport = page.viewportSize()!

    await page.mouse.move(hbox.x + hbox.width / 2, hbox.y + hbox.height / 2)
    await page.mouse.down()
    await page.mouse.move(viewport.width / 2, 10, { steps: 12 })

    const assist = page.locator('[data-workspace-snap-assist]')
    await expect(assist).toBeVisible()

    const mini = assistMiniGrid(page, '3x2')
    const leftTop = assistMiniGridCell(mini, 0, 0, 0, 0)
    const centerTop = assistMiniGridCell(mini, 1, 1, 0, 0)
    await expect(leftTop).toBeVisible()
    await expect(centerTop).toBeVisible()

    const lbox = await leftTop.boundingBox()
    const cbox = await centerTop.boundingBox()
    if (!lbox || !cbox) throw new Error('Assist cells not laid out')

    await page.mouse.move(lbox.x + lbox.width / 2, lbox.y + lbox.height / 2, { steps: 10 })
    await expect(leftTop).toHaveAttribute('data-snap-assist-hover-active', '')
    await expect(centerTop).not.toHaveAttribute('data-snap-assist-hover-active', '')

    await page.mouse.move(cbox.x + cbox.width / 2, cbox.y + cbox.height / 2, { steps: 12 })
    await expect(centerTop).toHaveAttribute('data-snap-assist-hover-active', '')
    await expect(leftTop).not.toHaveAttribute('data-snap-assist-hover-active', '')

    await page.mouse.up()
    await waitForWindowBoundsStable(page, groups.first())
  })

  test('highlights master grid tile under pointer and snaps on drop', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    const hbox = await handle.boundingBox()
    if (!hbox) throw new Error('Handle not visible')

    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT

    await page.mouse.move(hbox.x + hbox.width / 2, hbox.y + hbox.height / 2)
    await page.mouse.down()
    await page.mouse.move(viewport.width / 2, 10, { steps: 12 })

    const assist = page.locator('[data-workspace-snap-assist]')
    await expect(assist).toBeVisible()

    const cell = assistMiniGrid(page, '3x2').getByTestId('snap-assist-master-cell')
    await expect(cell).toBeVisible()
    const cbox = await cell.boundingBox()
    if (!cbox) throw new Error('Master cell not laid out')

    await page.mouse.move(cbox.x + cbox.width / 2, cbox.y + cbox.height / 2, { steps: 10 })
    await expect(cell).toHaveAttribute('data-snap-assist-hover-active', '')

    await page.mouse.up()
    await waitForWindowBoundsStable(page, groups.first())

    const bounds = await getWindowBounds(groups.first())
    const thirdW = Math.round(viewport.width / 3)
    const halfH = Math.round(containerH / 2)

    expect(bounds.x).toBeLessThanOrEqual(4)
    expect(bounds.y).toBeLessThanOrEqual(4)
    expect(bounds.width).toBeGreaterThan(thirdW - 28)
    expect(bounds.width).toBeLessThan(thirdW + 28)
    expect(bounds.height).toBeGreaterThan(halfH - 28)
    expect(bounds.height).toBeLessThan(halfH + 28)
  })

  test('snaps to full column via gutter between rows', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    const hbox = await handle.boundingBox()
    if (!hbox) throw new Error('Handle not visible')

    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT

    await page.mouse.move(hbox.x + hbox.width / 2, hbox.y + hbox.height / 2)
    await page.mouse.down()
    await page.mouse.move(viewport.width / 2, 10, { steps: 12 })

    const gutter = assistMiniGrid(page, '3x2').getByTestId('snap-assist-hgutter-col0')
    await expect(gutter).toBeVisible()
    const gbox = await gutter.boundingBox()
    if (!gbox) throw new Error('Gutter not laid out')

    await page.mouse.move(gbox.x + gbox.width / 2, gbox.y + gbox.height / 2, { steps: 10 })
    await expect(gutter).toHaveAttribute('data-snap-assist-hover-active', '')

    await page.mouse.up()
    await waitForWindowBoundsStable(page, groups.first())

    const bounds = await getWindowBounds(groups.first())
    const thirdW = Math.round(viewport.width / 3)

    expect(bounds.x).toBeLessThanOrEqual(4)
    expect(bounds.y).toBeLessThanOrEqual(4)
    expect(bounds.width).toBeGreaterThan(thirdW - 28)
    expect(bounds.width).toBeLessThan(thirdW + 28)
    expect(bounds.height).toBeGreaterThan(containerH - 40)
  })

  test('vertical gutter between two top tiles snaps to two columns one row', async () => {
    await gotoWorkspace(page)
    const groups = getWindowGroups(page)
    const handle = getDragHandle(groups.first())
    const hbox = await handle.boundingBox()
    if (!hbox) throw new Error('Handle not visible')

    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT

    await page.mouse.move(hbox.x + hbox.width / 2, hbox.y + hbox.height / 2)
    await page.mouse.down()
    await page.mouse.move(viewport.width / 2, 10, { steps: 12 })

    const vg = assistMiniGrid(page, '3x2').getByTestId('snap-assist-vgutter-two-cols-top')
    await expect(vg).toBeVisible()
    const vbox = await vg.boundingBox()
    if (!vbox) throw new Error('V-gutter not laid out')

    await page.mouse.move(vbox.x + vbox.width / 2, vbox.y + vbox.height / 2, { steps: 10 })
    await expect(vg).toHaveAttribute('data-snap-assist-hover-active', '')

    await page.mouse.up()
    await waitForWindowBoundsStable(page, groups.first())

    const bounds = await getWindowBounds(groups.first())
    const twoThirdsW = Math.round((viewport.width * 2) / 3)
    const halfH = Math.round(containerH / 2)

    expect(bounds.x).toBeLessThanOrEqual(4)
    expect(bounds.y).toBeLessThanOrEqual(4)
    expect(bounds.width).toBeGreaterThan(twoThirdsW - 30)
    expect(bounds.width).toBeLessThan(twoThirdsW + 30)
    expect(bounds.height).toBeGreaterThan(halfH - 28)
    expect(bounds.height).toBeLessThan(halfH + 28)
  })
})

test.describe('Resizing Snapped Windows', () => {
  test('resizes shared edge between left and right snapped windows', async () => {
    await gotoWorkspace(page)
    await setAssistGridShapeForTest(page, '2x2')
    const groups = getWindowGroups(page)
    await openBrowserWindow(page)

    await dragToEdge(page, getDragHandle(groups.first()), 'left')
    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)
    await dragToEdge(page, getDragHandle(groups.nth(1)), 'right')

    const boundsA = await getWindowBounds(groups.first())
    const boundsB = await getWindowBounds(groups.nth(1))
    const leftWindow = boundsA.x < boundsB.x ? groups.first() : groups.nth(1)
    const rightWindow = boundsA.x < boundsB.x ? groups.nth(1) : groups.first()

    const leftBounds = boundsA.x < boundsB.x ? boundsA : boundsB
    const rightBounds = boundsA.x < boundsB.x ? boundsB : boundsA

    expect(leftBounds.x).toBeLessThanOrEqual(5)
    expect(rightBounds.x).toBeGreaterThan(leftBounds.width - 20)

    const resizeHandle = await getSharedColumnResizeHandle(leftWindow)
    await expect(resizeHandle).toBeAttached()

    const handleBox = await resizeHandle.boundingBox()
    if (!handleBox) throw new Error('Resize handle not found')

    const startX = handleBox.x + handleBox.width / 2
    const startY = handleBox.y + handleBox.height / 2
    await page.mouse.move(startX, startY)
    await page.mouse.down()
    await page.mouse.move(startX + 100, startY, { steps: 12 })
    await page.mouse.up()
    await waitForWindowBoundsStable(page, leftWindow)
    await waitForWindowBoundsStable(page, rightWindow)
    await page.waitForTimeout(80)

    const newLeftBounds = await getWindowBounds(leftWindow)
    const newRightBounds = await getWindowBounds(rightWindow)

    expect(newLeftBounds.width).toBeGreaterThan(leftBounds.width)
    expect(newRightBounds.x).toBeGreaterThan(rightBounds.x)
  })

  test('resizing left snapped column moves both top-right and bottom-right windows', async () => {
    await gotoWorkspace(page)
    await setAssistGridShapeForTest(page, '2x2')
    await openBrowserWindow(page)
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)
    await expect(groups).toHaveCount(3)

    await dragToEdge(page, getDragHandle(groups.nth(0)), 'left')

    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)
    await dragToEdge(page, getDragHandle(groups.nth(1)), 'top-right')

    await groups.nth(2).dispatchEvent('mousedown')
    await page.waitForTimeout(30)
    await dragToEdge(page, getDragHandle(groups.nth(2)), 'bottom-right')

    await expect(groups).toHaveCount(3)
    for (let i = 0; i < 3; i++) {
      await expect(groups.nth(i)).toBeVisible()
    }

    const b0 = await getWindowBounds(groups.nth(0))
    const b1 = await getWindowBounds(groups.nth(1))
    const b2 = await getWindowBounds(groups.nth(2))
    expect(b0).toBeTruthy()
    expect(b1).toBeTruthy()
    expect(b2).toBeTruthy()

    const byX = [b0, b1, b2]
      .map((bounds, i) => ({ bounds, i }))
      .sort((a, c) => a.bounds.x - c.bounds.x)
    const leftIdx = byX[0]!.i
    const rightTopIdx = byX[1]!.bounds.y <= byX[2]!.bounds.y ? byX[1]!.i : byX[2]!.i
    const rightBottomIdx = rightTopIdx === byX[1]!.i ? byX[2]!.i : byX[1]!.i

    const leftWindow = groups.nth(leftIdx)
    const topRightWindow = groups.nth(rightTopIdx)
    const bottomRightWindow = groups.nth(rightBottomIdx)

    const leftBounds = byX[0]!.bounds
    const topRightBefore = await getWindowBounds(topRightWindow)
    const bottomRightBefore = await getWindowBounds(bottomRightWindow)

    expect(leftBounds.x).toBeLessThanOrEqual(5)
    expect(topRightBefore.x).toBeGreaterThan(leftBounds.x + leftBounds.width - 25)
    expect(bottomRightBefore.x).toBeGreaterThan(leftBounds.x + leftBounds.width - 25)
    expect(Math.abs(topRightBefore.x - bottomRightBefore.x)).toBeLessThanOrEqual(8)
    expect(Math.abs(topRightBefore.width - bottomRightBefore.width)).toBeLessThanOrEqual(8)

    const resizeHandle = await getSharedColumnResizeHandle(leftWindow)
    await expect(resizeHandle).toBeAttached()

    const handleBox = await resizeHandle.boundingBox()
    if (!handleBox) throw new Error('Resize handle not found')

    const startX = handleBox.x + handleBox.width / 2
    const startY = handleBox.y + handleBox.height / 2
    await page.mouse.move(startX, startY)
    await page.mouse.down()
    await page.mouse.move(startX + 100, startY, { steps: 12 })
    await page.mouse.up()
    await waitForWindowBoundsStable(page, leftWindow)
    await waitForWindowBoundsStable(page, topRightWindow)
    await waitForWindowBoundsStable(page, bottomRightWindow)
    await page.waitForTimeout(80)

    const topRightAfter = await getWindowBounds(topRightWindow)
    const bottomRightAfter = await getWindowBounds(bottomRightWindow)
    const leftAfter = await getWindowBounds(leftWindow)

    expect(leftAfter.width).toBeGreaterThan(leftBounds.width)
    expect(topRightAfter.x).toBeGreaterThan(topRightBefore.x)
    expect(bottomRightAfter.x).toBeGreaterThan(bottomRightBefore.x)
    expect(Math.abs(topRightAfter.x - bottomRightAfter.x)).toBeLessThanOrEqual(8)
    expect(Math.abs(topRightAfter.width - bottomRightAfter.width)).toBeLessThanOrEqual(8)
  })

  test('resizes shared edge between top and bottom quarter windows', async () => {
    await gotoWorkspace(page)
    await setAssistGridShapeForTest(page, '2x2')
    const groups = getWindowGroups(page)
    await openBrowserWindow(page)

    await dragToEdge(page, getDragHandle(groups.first()), 'top-left')
    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)
    await dragToEdge(page, getDragHandle(groups.nth(1)), 'bottom-left')

    const boundsA = await getWindowBounds(groups.first())
    const boundsB = await getWindowBounds(groups.nth(1))
    const topWindow = boundsA.y < boundsB.y ? groups.first() : groups.nth(1)
    const bottomWindow = boundsA.y < boundsB.y ? groups.nth(1) : groups.first()

    const topBounds = boundsA.y < boundsB.y ? boundsA : boundsB
    const bottomBounds = boundsA.y < boundsB.y ? boundsB : boundsA

    expect(topBounds.y).toBeLessThanOrEqual(5)
    expect(bottomBounds.y).toBeGreaterThan(topBounds.height - 20)

    const topRnd = getRndWrapper(topWindow)
    const resizeHandle = topRnd.locator('div[style*="row-resize"]').first()
    await expect(resizeHandle).toBeAttached()

    const handleBox = await resizeHandle.boundingBox()
    if (!handleBox) throw new Error('Resize handle not found')

    const startX = handleBox.x + handleBox.width / 2
    const startY = handleBox.y + handleBox.height / 2
    await page.mouse.move(startX, startY)
    await page.mouse.down()
    await page.mouse.move(startX, startY + 60, { steps: 10 })
    await page.mouse.up()
    await waitForWindowBoundsStable(page, topWindow)

    const newTopBounds = await getWindowBounds(topWindow)
    const newBottomBounds = await getWindowBounds(bottomWindow)

    expect(newTopBounds.height).toBeGreaterThan(topBounds.height)
    expect(newBottomBounds.y).toBeGreaterThan(bottomBounds.y)
  })

  test('resizes shared edge between third layout windows (top-left-third and top-center-third)', async () => {
    await gotoWorkspace(page)
    await setAssistGridShapeForTest(page, '3x2')
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)

    const maximizeBtn = groups.first().locator('button:has(.lucide-maximize-2)')
    await maximizeBtn.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()
    await assistMiniGrid(page, '3x2').getByTestId('snap-assist-master-cell').click()
    await waitForWindowBoundsStable(page, groups.first())

    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)

    const maximizeBtn2 = groups.nth(1).locator('button:has(.lucide-maximize-2)')
    await maximizeBtn2.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()
    await assistMiniGrid(page, '3x2')
      .locator(
        '[data-assist-master-grid] button[data-grid-cols="3"][data-gc0="1"][data-gc1="1"][data-gr0="0"][data-gr1="0"]',
      )
      .click()
    await waitForWindowBoundsStable(page, groups.nth(1))

    const boundsA = await getWindowBounds(groups.first())
    const boundsB = await getWindowBounds(groups.nth(1))
    const leftWindow = boundsA.x < boundsB.x ? groups.first() : groups.nth(1)
    const rightWindow = boundsA.x < boundsB.x ? groups.nth(1) : groups.first()

    const leftBounds = boundsA.x < boundsB.x ? boundsA : boundsB
    const rightBounds = boundsA.x < boundsB.x ? boundsB : boundsA

    expect(leftBounds.x).toBeLessThanOrEqual(5)
    expect(rightBounds.x).toBeGreaterThan(leftBounds.width - 20)

    const leftRnd = getRndWrapper(leftWindow)
    const resizeHandle = leftRnd.locator('div[style*="col-resize"]').first()
    await expect(resizeHandle).toBeAttached()

    const handleBox = await resizeHandle.boundingBox()
    if (!handleBox) throw new Error('Resize handle not found')

    const startX = handleBox.x + handleBox.width / 2
    const startY = handleBox.y + handleBox.height / 2
    await page.mouse.move(startX, startY)
    await page.mouse.down()
    await page.mouse.move(startX + 80, startY, { steps: 10 })
    await page.mouse.up()
    await waitForWindowBoundsStable(page, leftWindow)

    const newLeftBounds = await getWindowBounds(leftWindow)
    const newRightBounds = await getWindowBounds(rightWindow)

    expect(newLeftBounds.width).toBeGreaterThan(leftBounds.width)
    expect(newRightBounds.x).toBeGreaterThan(rightBounds.x)
  })

  test('resizes shared edge between left-third and right-two-thirds (1/3 + 2/3 layout)', async () => {
    await gotoWorkspace(page)
    await setAssistGridShapeForTest(page, '3x2')
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)

    await dragToEdge(page, getDragHandle(groups.first()), 'left')
    await waitForWindowBoundsStable(page, groups.first())

    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)

    const maximizeBtn2 = groups.nth(1).locator('button:has(.lucide-maximize-2)')
    await maximizeBtn2.click({ button: 'right' })
    await expect(page.locator('[data-tiling-picker]')).toBeVisible()
    await assistMiniGrid(page, '3x2')
      .locator(
        '[data-assist-master-grid] button[data-grid-cols="3"][data-gc0="1"][data-gc1="2"][data-gr0="0"][data-gr1="1"]',
      )
      .click()
    await waitForWindowBoundsStable(page, groups.nth(1))

    const leftWindow = groups.first()
    const rightWindow = groups.nth(1)
    const leftBounds = await getWindowBounds(leftWindow)
    const rightBounds = await getWindowBounds(rightWindow)

    const viewport = page.viewportSize()!
    const thirdW = Math.round(viewport.width / 3)
    expect(leftBounds.x).toBeLessThanOrEqual(5)
    expect(leftBounds.width).toBeGreaterThan(thirdW - 50)
    expect(leftBounds.width).toBeLessThan(thirdW + 50)
    expect(rightBounds.x).toBeGreaterThan(leftBounds.width - 20)

    const leftRnd = getRndWrapper(leftWindow)
    const resizeHandle = leftRnd.locator('div[style*="col-resize"]').first()
    await expect(resizeHandle).toBeAttached()

    const handleBox = await resizeHandle.boundingBox()
    if (!handleBox) throw new Error('Resize handle not found')

    const startX = handleBox.x + handleBox.width / 2
    const startY = handleBox.y + handleBox.height / 2
    await page.mouse.move(startX, startY)
    await page.mouse.down()
    await page.mouse.move(startX + 80, startY, { steps: 10 })
    await page.mouse.up()
    await waitForWindowBoundsStable(page, leftWindow)

    const newLeftBounds = await getWindowBounds(leftWindow)
    const newRightBounds = await getWindowBounds(rightWindow)

    expect(newLeftBounds.width).toBeGreaterThan(leftBounds.width)
    expect(newRightBounds.x).toBeGreaterThan(rightBounds.x)
  })

  test('snapping second window to right half fills remaining space after left is resized', async () => {
    await gotoWorkspace(page)
    await setAssistGridShapeForTest(page, '2x2')
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)

    await dragToEdge(page, getDragHandle(groups.first()), 'left')

    const leftRnd = getRndWrapper(groups.first())
    const resizeHandle = leftRnd.locator('div[style*="col-resize"]').first()
    await expect(resizeHandle).toBeAttached()

    const handleBox = await resizeHandle.boundingBox()
    if (!handleBox) throw new Error('Resize handle not found')

    const startX = handleBox.x + handleBox.width / 2
    const startY = handleBox.y + handleBox.height / 2
    const viewport = page.viewportSize()!
    const halfW = Math.round(viewport.width / 2)
    await page.mouse.move(startX, startY)
    await page.mouse.down()
    await page.mouse.move(halfW - 150, startY, { steps: 10 })
    await page.mouse.up()
    await waitForWindowBoundsStable(page, groups.first())

    const resizedLeftBounds = await getWindowBounds(groups.first())
    const leftRightEdge = resizedLeftBounds.x + resizedLeftBounds.width

    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)
    await dragToEdge(page, getDragHandle(groups.nth(1)), 'right')

    const boundsA = await getWindowBounds(groups.first())
    const boundsB = await getWindowBounds(groups.nth(1))
    const rightBounds = boundsA.x < boundsB.x ? boundsB : boundsA

    expect(rightBounds.x + rightBounds.width).toBeGreaterThan(viewport.width - 25)
    expect(rightBounds.x).toBeGreaterThan(leftRightEdge - 200)
    expect(rightBounds.width).toBeGreaterThan(120)
  })

  test('snapping second window to bottom quarter fills remaining space after top-left is resized', async () => {
    await gotoWorkspace(page)
    await setAssistGridShapeForTest(page, '2x2')
    await openBrowserWindow(page)

    const groups = getWindowGroups(page)

    await dragToEdge(page, getDragHandle(groups.first()), 'top-left')

    const topRnd = getRndWrapper(groups.first())
    const resizeHandle = topRnd.locator('div[style*="row-resize"]').first()
    await expect(resizeHandle).toBeAttached()

    const handleBox = await resizeHandle.boundingBox()
    if (!handleBox) throw new Error('Resize handle not found')

    const viewport = page.viewportSize()!
    const containerH = viewport.height - TASKBAR_HEIGHT
    const halfH = Math.round(containerH / 2)

    const startX = handleBox.x + handleBox.width / 2
    const startY = handleBox.y + handleBox.height / 2
    await page.mouse.move(startX, startY)
    await page.mouse.down()
    await page.mouse.move(startX, halfH - 80, { steps: 10 })
    await page.mouse.up()
    await waitForWindowBoundsStable(page, groups.first())

    const resizedTopBounds = await getWindowBounds(groups.first())
    const topBottomEdge = resizedTopBounds.y + resizedTopBounds.height

    await groups.nth(1).dispatchEvent('mousedown')
    await page.waitForTimeout(30)
    await dragToEdge(page, getDragHandle(groups.nth(1)), 'bottom-left')

    const boundsA = await getWindowBounds(groups.first())
    const boundsB = await getWindowBounds(groups.nth(1))
    const bottomBounds = boundsA.y < boundsB.y ? boundsB : boundsA

    expect(bottomBounds.y + bottomBounds.height).toBeGreaterThan(containerH - 35)
    expect(bottomBounds.y).toBeGreaterThan(topBottomEdge - 200)
    expect(bottomBounds.height).toBeGreaterThan(120)
  })
})
