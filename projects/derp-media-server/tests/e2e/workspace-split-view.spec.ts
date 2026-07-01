import { test, expect, type BrowserContext, type Page } from '@playwright/test'
import {
  gotoWorkspace,
  openBrowserWindow,
  getWindowGroups,
  getDragHandle,
  waitForWindowBoundsStable,
  dragFromTo,
  WORKSPACE_VISIBLE_WINDOW_GROUP,
} from './workspace-layout-helpers'
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

async function mergeSecondBrowserIntoFirst(p: Page) {
  const groups = getWindowGroups(p)
  const handleB = getDragHandle(groups.nth(1))
  const boxB = await handleB.boundingBox()
  const boxA = await getDragHandle(groups.first()).boundingBox()
  if (!boxB || !boxA) throw new Error('Handles not visible')

  await dragFromTo(
    p,
    boxB.x + boxB.width / 2,
    boxB.y + boxB.height / 2,
    boxA.x + boxA.width / 2,
    boxA.y + 16,
  )
  await waitForWindowBoundsStable(p, getWindowGroups(p).first())
  await expect(getWindowGroups(p)).toHaveCount(1)
}

async function chooseWorkspaceOpenTarget(p: Page, label: 'New tab' | 'New window') {
  const dialog = p.getByRole('dialog').filter({ has: p.getByRole('heading', { name: 'Settings' }) })
  await dialog.getByRole('button', { name: label }).click()
  await p.keyboard.press('Escape')
}

test.describe('Workspace split view', () => {
  test('enter split via tab context shows left and right panes', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)
    await mergeSecondBrowserIntoFirst(page)

    const tabStrip = page.locator('.workspace-tab-strip')
    const firstTab = tabStrip.locator('[data-workspace-tab-id]').first()
    await firstTab.click({ button: 'right' })
    await page.getByTestId('workspace-tab-menu-use-split-left').click()

    await expect(page.getByTestId('workspace-split-left-pane')).toBeVisible()
    await expect(page.getByTestId('workspace-split-right-pane')).toBeVisible()
  })

  test('exit split via left tab context restores single visible pane', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)
    await mergeSecondBrowserIntoFirst(page)

    const tabStrip = page.locator('.workspace-tab-strip')
    await tabStrip.locator('[data-workspace-tab-id]').first().click({ button: 'right' })
    await page.getByTestId('workspace-tab-menu-use-split-left').click()
    await expect(page.getByTestId('workspace-split-left-pane')).toBeVisible()

    await tabStrip.locator('[data-workspace-split-left-tab]').click({ button: 'right' })
    await page.getByTestId('workspace-tab-menu-exit-split').click()

    await expect(page.getByTestId('workspace-split-left-pane')).toHaveCount(0)
  })

  test('divider drag keeps each pane at least ~30% of split row', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)
    await mergeSecondBrowserIntoFirst(page)

    const tabStrip = page.locator('.workspace-tab-strip')
    await tabStrip.locator('[data-workspace-tab-id]').first().click({ button: 'right' })
    await page.getByTestId('workspace-tab-menu-use-split-left').click()

    const divider = page.getByTestId('workspace-split-divider')
    const divBox = await divider.boundingBox()
    if (!divBox) throw new Error('divider')

    await dragFromTo(
      page,
      divBox.x + divBox.width / 2,
      divBox.y + divBox.height / 2,
      divBox.x + divBox.width / 2 + 120,
      divBox.y + divBox.height / 2,
    )

    const left = page.getByTestId('workspace-split-left-pane')
    const right = page.getByTestId('workspace-split-right-pane')
    const lb = await left.boundingBox()
    const rb = await right.boundingBox()
    if (!lb || !rb) throw new Error('panes')
    const row = lb.width + rb.width + divBox.width
    expect(lb.width / row).toBeGreaterThanOrEqual(0.28)
    expect(rb.width / row).toBeGreaterThanOrEqual(0.28)
  })

  test('split left tab cannot be pulled into a new window', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)
    await mergeSecondBrowserIntoFirst(page)

    const tabStrip = page.locator('.workspace-tab-strip')
    await tabStrip.locator('[data-workspace-tab-id]').first().click({ button: 'right' })
    await page.getByTestId('workspace-tab-menu-use-split-left').click()

    const leftTab = tabStrip.locator('[data-workspace-split-left-tab]')
    const box = await leftTab.boundingBox()
    if (!box) throw new Error('left tab')

    await dragFromTo(
      page,
      box.x + box.width / 2,
      box.y + box.height / 2,
      box.x + box.width / 2,
      box.y + box.height / 2 + 80,
    )
    await waitForWindowBoundsStable(page, getWindowGroups(page).first())

    await expect(page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP)).toHaveCount(1)
  })

  test('right tab can still be pulled into a new window', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)
    await mergeSecondBrowserIntoFirst(page)

    const tabStrip = page.locator('.workspace-tab-strip')
    await tabStrip.locator('[data-workspace-tab-id]').first().click({ button: 'right' })
    await page.getByTestId('workspace-tab-menu-use-split-left').click()

    const rightTabs = tabStrip.locator(
      '[data-workspace-tab-id]:not([data-workspace-split-left-tab])',
    )
    const second = rightTabs.nth(0)
    const box = await second.boundingBox()
    if (!box) throw new Error('right tab')

    await dragFromTo(
      page,
      box.x + box.width / 2,
      box.y + box.height / 2,
      box.x + box.width / 2,
      box.y + box.height / 2 + 80,
    )
    await waitForWindowBoundsStable(page, getWindowGroups(page).first())

    await expect(page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP)).toHaveCount(2)
  })

  test('with New window setting, opening from left browser stays in same group as split', async () => {
    await gotoWorkspace(page)
    await page.getByRole('button', { name: 'Open settings' }).click()
    await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible()
    await chooseWorkspaceOpenTarget(page, 'New window')

    await openBrowserWindow(page)
    await mergeSecondBrowserIntoFirst(page)

    const tabStrip = page.locator('.workspace-tab-strip')
    await tabStrip.locator('[data-workspace-tab-id]').first().click({ button: 'right' })
    await page.getByTestId('workspace-tab-menu-use-split-left').click()

    const group = getWindowGroups(page).first()
    const content = group.locator('[data-testid="workspace-split-left-pane"]')
    await content.getByText('Documents', { exact: true }).click()
    await expect(content.getByText('readme.txt')).toBeVisible({ timeout: 10_000 })
    await content.getByText('readme.txt').click()

    await expect(getWindowGroups(page)).toHaveCount(1)
    await expect(page.getByTestId('workspace-split-right-pane')).toBeVisible()
  })

  test('Open in split view from file context', async () => {
    await gotoWorkspace(page)

    const group = getWindowGroups(page).first()
    const content = group.locator('[data-testid="workspace-window-visible-content"]')
    await content.getByText('Documents', { exact: true }).click()
    await expect(content.getByText('readme.txt')).toBeVisible({ timeout: 10_000 })
    await content.getByText('readme.txt').click({ button: 'right' })
    await page.getByTestId('workspace-file-menu-open-split-view').click()

    await expect(page.getByTestId('workspace-split-left-pane')).toBeVisible()
    await expect(page.getByTestId('workspace-split-right-pane')).toBeVisible()
    await expect(getWindowGroups(page)).toHaveCount(1)
  })

  test('clicking split left tab keeps right tab visually active in strip', async () => {
    await gotoWorkspace(page)
    await openBrowserWindow(page)
    await mergeSecondBrowserIntoFirst(page)

    const tabStrip = page.locator('.workspace-tab-strip')
    await tabStrip.locator('[data-workspace-tab-id]').first().click({ button: 'right' })
    await page.getByTestId('workspace-tab-menu-use-split-left').click()

    const rightTab = tabStrip
      .locator('[data-workspace-tab-id]:not([data-workspace-split-left-tab])')
      .first()
    const leftTab = tabStrip.locator('[data-workspace-split-left-tab]')
    await rightTab.click()
    await expect(rightTab).toHaveClass(/bg-background/)

    await leftTab.click()
    await expect(rightTab).toHaveClass(/bg-background/)
    await expect(leftTab).not.toHaveClass(/bg-background/)
  })
})
