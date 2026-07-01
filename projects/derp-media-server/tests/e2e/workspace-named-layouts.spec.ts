import { test, expect, type BrowserContext, type Page } from '@playwright/test'
import { openBrowserWindow, getWindowGroups } from '../e2e/workspace-layout-helpers'
import { createWorkspaceE2EContext } from './workspace-e2e-auth'

const batch = process.env.BATCH_ID ?? 'local'

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

test.describe('workspace named layout presets', () => {
  test('save preset, layout dirty revert, hydrate via preset URL', async () => {
    const ws = `e2e-named-layout-${batch}-${Date.now()}`
    await page.goto(`/workspace?ws=${ws}`)
    await expect(page.getByTestId('workspace-named-layout-trigger')).toBeEnabled()
    await expect(getWindowGroups(page)).toHaveCount(1)

    await page.getByTestId('workspace-named-layout-trigger').click()
    await page.getByRole('menuitem', { name: 'Save current layout…' }).click()
    await expect(page.getByRole('dialog', { name: 'Save layout' })).toBeVisible()

    const respPromise = page.waitForResponse(
      (r) =>
        r.url().includes('/api/settings/workspaceLayoutPresets') && r.request().method() === 'POST',
    )
    await page.getByPlaceholder('e.g. Review + browser').fill(`Batch ${batch} named layout`)
    const saveBtn = page
      .getByRole('dialog', { name: 'Save layout' })
      .getByRole('button', { name: 'Save', exact: true })
    await expect(saveBtn).toBeEnabled()
    await saveBtn.click()
    const saveResp = await respPromise
    expect(saveResp.ok()).toBeTruthy()
    await expect(page.getByRole('dialog', { name: 'Save layout' })).toBeHidden()
    const saveBody = (await saveResp.json()) as { workspaceLayoutPresets: { id: string }[] }
    const presetId = saveBody.workspaceLayoutPresets.at(-1)?.id
    expect(presetId).toBeTruthy()

    await expect(page).not.toHaveURL(/[?&]preset=/)

    await openBrowserWindow(page)
    await expect(getWindowGroups(page)).toHaveCount(2)

    await page.getByTestId('workspace-named-layout-trigger').click()
    await expect(page.getByRole('menuitem', { name: 'Revert to baseline' })).toBeVisible()
    await page.getByRole('menuitem', { name: 'Revert to baseline' }).click()
    await expect(getWindowGroups(page)).toHaveCount(1)

    const ws2 = `e2e-named-layout-2-${batch}-${Date.now()}`
    await page.goto(`/workspace?ws=${ws2}&preset=${presetId}`)
    await expect(getWindowGroups(page)).toHaveCount(1)
    await expect(page).not.toHaveURL(/[?&]preset=/)
  })

  test('update saved preset snapshot from current windows', async () => {
    const ws = `e2e-update-layout-${batch}-${Date.now()}`
    await page.goto(`/workspace?ws=${ws}`)
    await expect(getWindowGroups(page)).toHaveCount(1)

    await page.getByTestId('workspace-named-layout-trigger').click()
    await page.getByRole('menuitem', { name: 'Save current layout…' }).click()
    await page.getByPlaceholder('e.g. Review + browser').fill(`Update preset ${batch}`)
    await page
      .getByRole('dialog', { name: 'Save layout' })
      .getByRole('button', { name: 'Save' })
      .click()
    await expect(page.getByRole('dialog', { name: 'Save layout' })).toBeHidden()

    await openBrowserWindow(page)
    await expect(getWindowGroups(page)).toHaveCount(2)

    await page.getByTestId('workspace-named-layout-trigger').click()
    const updateLabel = new RegExp(`Update layout.*Update preset ${batch}.*from current windows`)
    const [upd] = await Promise.all([
      page.waitForResponse(
        (r) =>
          r.url().includes('/api/settings/workspaceLayoutPresets') &&
          r.request().method() === 'POST',
      ),
      page
        .locator('[data-workspace-layout-menu]')
        .getByRole('button', { name: updateLabel })
        .click(),
    ])
    expect(upd.ok()).toBeTruthy()

    const ws2 = `e2e-update-layout-2-${batch}-${Date.now()}`
    const presetId = (
      (await upd.json()) as { workspaceLayoutPresets: { id: string; name: string }[] }
    ).workspaceLayoutPresets.find((p) => p.name === `Update preset ${batch}`)?.id
    expect(presetId).toBeTruthy()

    await page.goto(`/workspace?ws=${ws2}&preset=${presetId}`)
    await expect(getWindowGroups(page)).toHaveCount(2)
    await expect(page).not.toHaveURL(/[?&]preset=/)
  })
})
