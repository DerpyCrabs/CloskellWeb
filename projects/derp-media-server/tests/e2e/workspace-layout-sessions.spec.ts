import { test, expect, type BrowserContext } from '@playwright/test'
import { WORKSPACE_VISIBLE_WINDOW_GROUP } from './workspace-layout-helpers'
import { createWorkspaceE2EContext } from './workspace-e2e-auth'

let sharedContext: BrowserContext

test.beforeAll(async ({ browser }) => {
  sharedContext = await createWorkspaceE2EContext(browser)
})

test.afterAll(async () => {
  await sharedContext.close()
})

test.describe('workspace layout sessions', () => {
  test('isolates window drafts per ws query param', async () => {
    const wsA = 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa'
    const wsB = 'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb'
    const pageA = await sharedContext.newPage()
    const pageB = await sharedContext.newPage()
    await pageA.goto(`/workspace?ws=${wsA}`)
    await pageB.goto(`/workspace?ws=${wsB}`)
    await expect(pageA.locator(WORKSPACE_VISIBLE_WINDOW_GROUP).first()).toBeVisible()
    await expect(pageB.locator(WORKSPACE_VISIBLE_WINDOW_GROUP).first()).toBeVisible()

    await expect(pageA.locator(WORKSPACE_VISIBLE_WINDOW_GROUP)).toHaveCount(1)
    await expect(pageB.locator(WORKSPACE_VISIBLE_WINDOW_GROUP)).toHaveCount(1)

    await pageA.getByTitle('Open browser window').click()
    await expect(pageA.locator(WORKSPACE_VISIBLE_WINDOW_GROUP)).toHaveCount(2)
    await expect(pageB.locator(WORKSPACE_VISIBLE_WINDOW_GROUP)).toHaveCount(1)

    await pageA.close()
    await pageB.close()
  })
})
