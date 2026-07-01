import { test, expect } from '@playwright/test'

test.describe('File Download', () => {
  test('context menu shows Download for files', async ({ page }) => {
    await page.goto('/?dir=Documents')
    await page.locator('table tr').filter({ hasText: 'readme.txt' }).click({ button: 'right' })
    await expect(
      page.locator('[data-slot="context-menu-item"]').getByText('Download', { exact: true }),
    ).toBeVisible()
  })

  test('downloads a single file via context menu', async ({ page }) => {
    await page.goto('/?dir=Documents')

    const downloadPromise = page.waitForEvent('download')
    await page.locator('table tr').filter({ hasText: 'readme.txt' }).click({ button: 'right' })
    const downloadItem = page.locator('[data-slot="context-menu-item"]').getByText('Download', {
      exact: true,
    })
    await expect(downloadItem).toBeVisible()
    await downloadItem.click({ noWaitAfter: true })
    const download = await downloadPromise

    expect(download.suggestedFilename()).toBe('readme.txt')
  })

  test('context menu shows "Download as ZIP" for folders', async ({ page }) => {
    await page.goto('/')
    await page.locator('table tr').filter({ hasText: 'Documents' }).click({ button: 'right' })
    await expect(
      page.locator('[data-slot="context-menu-item"]').getByText('Download as ZIP'),
    ).toBeVisible()
  })

  test('downloads a folder as ZIP via context menu', async ({ page }) => {
    await page.goto('/')

    const downloadPromise = page.waitForEvent('download')
    await page.locator('table tr').filter({ hasText: 'EmptyFolder' }).click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Download as ZIP').click()
    const download = await downloadPromise

    expect(download.suggestedFilename()).toBe('EmptyFolder.zip')
  })
})
