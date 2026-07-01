import { expect, type Locator, type Page } from '@playwright/test'

const TASKBAR_HEIGHT = 32

const WORKSPACE_VISIBLE_WINDOW_GROUP = '[data-window-group]:not([data-workspace-window-minimized])'

export async function gotoWorkspace(page: Page) {
  await page.goto('/workspace')
  await expect(page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP).first()).toBeVisible()
}

export async function openBrowserWindow(page: Page) {
  const countBefore = await page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP).count()
  await page.locator('button[title="Open browser window"]').click()
  await expect(page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP)).toHaveCount(countBefore + 1)
}

export function getWindowGroups(page: Page) {
  return page.locator(WORKSPACE_VISIBLE_WINDOW_GROUP)
}

export function getVisibleContent(windowGroup: Locator) {
  return windowGroup.locator('[data-testid="workspace-window-visible-content"]')
}

export function getDragHandle(windowGroup: Locator) {
  return windowGroup.locator('[data-testid="window-drag-handle"]')
}

export async function dragFromTo(
  page: Page,
  fromX: number,
  fromY: number,
  toX: number,
  toY: number,
  steps = 10,
) {
  await page.mouse.move(fromX, fromY)
  await page.mouse.down()
  await page.mouse.move(toX, toY, { steps })
  await page.mouse.up()
}

export async function dragToEdge(page: Page, handle: Locator, target: 'left' | 'right') {
  const viewport = page.viewportSize()!
  const box = await handle.boundingBox()
  if (!box) throw new Error('Handle not visible')

  const startX = box.x + box.width / 2
  const startY = box.y + box.height / 2
  const containerHeight = viewport.height - TASKBAR_HEIGHT
  const endX = target === 'left' ? 5 : viewport.width - 5
  const endY = containerHeight / 2

  await dragFromTo(page, startX, startY, endX, endY)
  await page.waitForTimeout(30)
}

export async function navigateToSharedContent(content: Locator) {
  await expect(content.getByText('SharedContent', { exact: true })).toBeVisible()
  await content.getByText('SharedContent', { exact: true }).click()
  await expect(content.getByText('public-doc.txt')).toBeVisible({ timeout: 5_000 })
}

export async function createTempFile(page: Page, content: Locator, fileName: string) {
  await content.locator('button[title="Create new file"]').scrollIntoViewIfNeeded()
  await content.locator('button[title="Create new file"]').click()
  const dialog = page.locator('[role="dialog"]').filter({
    has: page.locator('input[placeholder*="File name"]'),
  })
  await expect(dialog).toBeVisible()
  await dialog.locator('input[placeholder*="File name"]').fill(fileName)
  await dialog.getByRole('button', { name: 'Create' }).click()
  await expect(dialog).toBeHidden()
  await expect(content.getByText(fileName)).toBeVisible({ timeout: 5_000 })
}

export async function deleteFileViaContextMenu(page: Page, content: Locator, fileName: string) {
  await content.locator('tr').filter({ hasText: fileName }).click({ button: 'right' })
  await page.locator('[data-slot="context-menu-item"]').getByText('Delete').click()
  await page.getByRole('button', { name: /Delete/i }).click()
  await expect(content.locator('tr').filter({ hasText: fileName })).not.toBeVisible({
    timeout: 5_000,
  })
}

/** Synthetic DnD so `dragstart` listeners (e.g. `setFileDragData`) populate `DataTransfer`. */
export async function html5DragDrop(source: Locator, target: Locator) {
  const srcHandle = await source.elementHandle()
  const tgtHandle = await target.elementHandle()
  if (!srcHandle || !tgtHandle) throw new Error('Element handles not found')

  await source.page().evaluate(
    ([src, tgt]) => {
      const dt = new DataTransfer()
      src!.dispatchEvent(
        new DragEvent('dragstart', { dataTransfer: dt, bubbles: true, cancelable: true }),
      )
      tgt!.dispatchEvent(
        new DragEvent('dragenter', { dataTransfer: dt, bubbles: true, cancelable: true }),
      )
      tgt!.dispatchEvent(
        new DragEvent('dragover', { dataTransfer: dt, bubbles: true, cancelable: true }),
      )
      tgt!.dispatchEvent(
        new DragEvent('drop', { dataTransfer: dt, bubbles: true, cancelable: true }),
      )
      src!.dispatchEvent(new DragEvent('dragend', { dataTransfer: dt, bubbles: true }))
    },
    [srcHandle, tgtHandle],
  )
}
