import { test, expect, type Page } from '@playwright/test'

const DEEP_DIR = 'Notes/seg-a/seg-b/seg-c/breadcrumb-deep'

async function narrowBreadcrumbSlot(page: Page, px: number | null) {
  await page.getByTestId('breadcrumb-slot').evaluate((el, w) => {
    const h = el as HTMLElement
    if (w == null) {
      for (const p of [
        'box-sizing',
        'width',
        'max-width',
        'min-width',
        'flex',
        'flex-wrap',
        'overflow',
      ]) {
        h.style.removeProperty(p)
      }
      return
    }
    h.style.setProperty('box-sizing', 'border-box', 'important')
    h.style.setProperty('width', `${w}px`, 'important')
    h.style.setProperty('max-width', `${w}px`, 'important')
    h.style.setProperty('min-width', '0', 'important')
    h.style.setProperty('flex', `0 0 ${w}px`, 'important')
    h.style.setProperty('flex-wrap', 'nowrap', 'important')
    h.style.setProperty('overflow', 'hidden', 'important')
  }, px)
}

test.describe('Breadcrumbs', () => {
  test.afterEach(async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    try {
      await narrowBreadcrumbSlot(page, null)
    } catch {
      // closed page
    }
  })

  test('deep path: full inline trail when wide; no path ellipsis', async ({ page }) => {
    await page.goto(`/?dir=${encodeURIComponent(DEEP_DIR)}`)
    await expect(page.locator('table').getByText('chain-readme.txt')).toBeVisible()

    const bar = page.getByTestId('breadcrumb-bar')
    await expect(bar).toHaveAttribute('data-breadcrumb-layout', 'inline')
    await expect(bar).not.toHaveAttribute('data-breadcrumb-path-ellipsis')
    await expect(bar.getByRole('button', { name: 'Home', exact: true })).toBeVisible()
    for (const name of ['Notes', 'seg-a', 'seg-b', 'seg-c', 'breadcrumb-deep']) {
      await expect(bar.getByRole('button', { name, exact: true })).toBeVisible()
    }
  })

  test('deep path: narrower inline keeps Home + … + parent + current', async ({ page }) => {
    await page.goto(`/?dir=${encodeURIComponent(DEEP_DIR)}`)
    await expect(page.locator('table').getByText('chain-readme.txt')).toBeVisible()

    await narrowBreadcrumbSlot(page, 520)
    const bar = page.getByTestId('breadcrumb-bar')
    await expect(bar).toHaveAttribute('data-breadcrumb-layout', 'inline')
    await expect(bar).toHaveAttribute('data-breadcrumb-path-ellipsis')
    await expect(bar.getByRole('button', { name: 'Home', exact: true })).toBeVisible()
    await expect(bar.getByTestId('breadcrumb-ellipsis').first()).toBeVisible()
    await expect(bar.getByRole('button', { name: 'Notes', exact: true })).toHaveCount(0)
    await expect(bar.getByRole('button', { name: 'seg-a', exact: true })).toHaveCount(0)
    await expect(bar.getByRole('button', { name: 'seg-b', exact: true })).toHaveCount(0)
    await expect(bar.getByRole('button', { name: 'seg-c', exact: true })).toBeVisible()
    await expect(bar.getByRole('button', { name: 'breadcrumb-deep', exact: true })).toBeVisible()
  })

  test('path ellipsis opens portaled path menu (like compact)', async ({ page }) => {
    await page.goto(`/?dir=${encodeURIComponent(DEEP_DIR)}`)
    await expect(page.locator('table').getByText('chain-readme.txt')).toBeVisible()

    await narrowBreadcrumbSlot(page, 520)
    const bar = page.getByTestId('breadcrumb-bar')
    await expect(page.getByTestId('breadcrumb-path-menu')).toHaveCount(0)
    await bar.getByTestId('breadcrumb-ellipsis').first().click()
    const menu = page.getByTestId('breadcrumb-path-menu')
    await expect(menu).toBeVisible()
    await expect(menu.getByRole('menuitem', { name: 'seg-a', exact: true })).toBeVisible()
    await expect(menu.getByRole('menuitem', { name: 'Home', exact: true })).toHaveCount(0)
    await expect(menu.getByRole('menuitem', { name: 'seg-c', exact: true })).toHaveCount(0)
    await expect(menu.getByRole('menuitem', { name: 'breadcrumb-deep', exact: true })).toHaveCount(
      0,
    )
    await bar.getByTestId('breadcrumb-ellipsis').first().click()
    await expect(page.getByTestId('breadcrumb-path-menu')).toHaveCount(0)
    await expect(bar.getByRole('button', { name: 'seg-a', exact: true })).toHaveCount(0)
  })

  test('tighter inline: Home + … + current only (parent reachable via ellipsis menu)', async ({
    page,
  }) => {
    await page.goto(`/?dir=${encodeURIComponent(DEEP_DIR)}`)
    await expect(page.locator('table').getByText('chain-readme.txt')).toBeVisible()

    await narrowBreadcrumbSlot(page, 340)
    const bar = page.getByTestId('breadcrumb-bar')
    await expect(bar).toHaveAttribute('data-breadcrumb-layout', 'inline')
    await expect(bar).toHaveAttribute('data-breadcrumb-path-ellipsis')
    await expect(bar.getByRole('button', { name: 'Home', exact: true })).toBeVisible()
    await expect(bar.getByRole('button', { name: 'breadcrumb-deep', exact: true })).toBeVisible()
    await expect(bar.getByRole('button', { name: 'seg-c', exact: true })).toHaveCount(0)
  })

  test('very narrow: compact trigger is muted (not primary) and opens portaled menu', async ({
    page,
  }) => {
    await page.goto(`/?dir=${encodeURIComponent(DEEP_DIR)}`)
    await expect(page.locator('table').getByText('chain-readme.txt')).toBeVisible()

    await narrowBreadcrumbSlot(page, 120)
    const bar = page.getByTestId('breadcrumb-bar')
    await expect(bar).toHaveAttribute('data-breadcrumb-layout', 'compact')
    const trigger = bar.locator('[data-breadcrumb-segment="path-picker"]')
    await expect(trigger).toBeVisible()
    await expect(trigger).toHaveAttribute('class', /bg-muted/)
    await expect(trigger).not.toHaveAttribute('class', /bg-primary/)

    await expect(page.getByTestId('breadcrumb-path-menu')).toHaveCount(0)
    await trigger.click()
    const menu = page.getByTestId('breadcrumb-path-menu')
    await expect(menu).toBeVisible()
    await expect(menu.getByRole('menuitem', { name: 'Home', exact: true })).toBeVisible()
    await expect(menu.getByRole('menuitem', { name: 'Notes', exact: true })).toBeVisible()
    await expect(menu.getByRole('menuitem', { name: 'breadcrumb-deep', exact: true })).toBeVisible()
  })

  test('compact menu navigates to an ancestor', async ({ page }) => {
    await page.goto(`/?dir=${encodeURIComponent(DEEP_DIR)}`)
    await expect(page.locator('table').getByText('chain-readme.txt')).toBeVisible()

    await narrowBreadcrumbSlot(page, 120)
    await page
      .getByTestId('breadcrumb-bar')
      .locator('[data-breadcrumb-segment="path-picker"]')
      .click()
    await page.getByTestId('breadcrumb-path-menu').locator('[data-breadcrumb-path="Notes"]').click()

    await page.waitForURL(/dir=Notes(?:&|$)/)
    await expect(page.locator('table').getByText('welcome.md')).toBeVisible()
  })

  test('three-segment path: wide enough slot shows Home + both folders inline', async ({
    page,
  }) => {
    await page.goto(`/?dir=${encodeURIComponent('Notes/subfolder')}`)
    await expect(page.locator('table').getByText('nested-note.md')).toBeVisible()

    await narrowBreadcrumbSlot(page, 420)
    const bar = page.getByTestId('breadcrumb-bar')
    await expect(bar).toHaveAttribute('data-breadcrumb-layout', 'inline')
    await expect(bar).not.toHaveAttribute('data-breadcrumb-path-ellipsis')
    await expect(bar.getByRole('button', { name: 'Home', exact: true })).toBeVisible()
    await expect(bar.getByRole('button', { name: 'Notes', exact: true })).toBeVisible()
    await expect(bar.getByRole('button', { name: 'subfolder', exact: true })).toBeVisible()
  })

  test('three-segment path: narrow slot uses compact', async ({ page }) => {
    await page.goto(`/?dir=${encodeURIComponent('Notes/subfolder')}`)
    await expect(page.locator('table').getByText('nested-note.md')).toBeVisible()

    await narrowBreadcrumbSlot(page, 200)
    const bar = page.getByTestId('breadcrumb-bar')
    await expect(bar).toHaveAttribute('data-breadcrumb-layout', 'compact')
    await expect(bar.locator('[data-breadcrumb-segment="path-picker"]')).toContainText('subfolder')
  })

  test('two-segment path: compact when slot too small', async ({ page }) => {
    await page.goto(`/?dir=${encodeURIComponent('Notes')}`)
    await expect(page.locator('table').getByText('welcome.md')).toBeVisible()

    await narrowBreadcrumbSlot(page, 56)
    const bar = page.getByTestId('breadcrumb-bar')
    await expect(bar).toHaveAttribute('data-breadcrumb-layout', 'compact')
    await expect(bar.locator('[data-breadcrumb-segment="path-picker"]')).toContainText('Notes')
  })

  test('clicking outside closes compact path menu', async ({ page }) => {
    await page.goto(`/?dir=${encodeURIComponent(DEEP_DIR)}`)
    await expect(page.locator('table').getByText('chain-readme.txt')).toBeVisible()

    await narrowBreadcrumbSlot(page, 120)
    await page
      .getByTestId('breadcrumb-bar')
      .locator('[data-breadcrumb-segment="path-picker"]')
      .click()
    await expect(page.getByTestId('breadcrumb-path-menu')).toBeVisible()

    await page.mouse.click(80, 420)
    await expect(page.getByTestId('breadcrumb-path-menu')).toHaveCount(0)
  })

  test('compact path menu: crumb context menu stacks above path menu', async ({ page }) => {
    await page.goto(`/?dir=${encodeURIComponent(DEEP_DIR)}`)
    await expect(page.locator('table').getByText('chain-readme.txt')).toBeVisible()

    await narrowBreadcrumbSlot(page, 120)
    await page
      .getByTestId('breadcrumb-bar')
      .locator('[data-breadcrumb-segment="path-picker"]')
      .click()
    const pathMenu = page.getByTestId('breadcrumb-path-menu')
    await expect(pathMenu).toBeVisible()

    await pathMenu.getByRole('menuitem', { name: 'Notes', exact: true }).click({ button: 'right' })
    const ctxMenu = page.locator('[data-slot="breadcrumb-context-menu"]')
    await expect(ctxMenu).toBeVisible()

    const contextOnTop = await page.evaluate(() => {
      const ctx = document.querySelector('[data-slot="breadcrumb-context-menu"]')
      if (!ctx) return false
      const r = ctx.getBoundingClientRect()
      const x = r.left + Math.min(r.width / 2, 80)
      const y = r.top + Math.min(r.height / 2, 80)
      const top = document.elementFromPoint(x, y)
      return top != null && top.closest('[data-slot="breadcrumb-context-menu"]') !== null
    })
    expect(contextOnTop).toBe(true)
  })
})
