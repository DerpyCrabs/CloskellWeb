import { test, expect } from '@playwright/test'

test.describe('Editable Folders', () => {
  test('creates a new folder via dialog', async ({ page }) => {
    await page.goto('/?dir=Notes')
    await page.locator('button[title="Create new folder"]').click()
    await page.locator('input[placeholder="Folder name"]').fill('test-folder')
    await page.getByRole('button', { name: 'Create' }).click()
    await expect(page.locator('table').getByText('test-folder')).toBeVisible()
  })

  test('creates a new file via dialog', async ({ page }) => {
    await page.goto('/?dir=Notes')
    await page.locator('button[title="Create new file"]').click()
    await page.locator('input[placeholder*="File name"]').fill('test-note.md')
    await page.getByRole('button', { name: 'Create' }).click()
    // Close auto-opened text viewer
    await page.locator('button[title="Close"]').click()
    await expect(page.locator('table').getByText('test-note.md')).toBeVisible()
  })

  test('renames a file via context menu', async ({ page }) => {
    await page.goto('/?dir=Notes')
    await page.locator('button[title="Create new file"]').click()
    await page.locator('input[placeholder*="File name"]').fill('rename-me.md')
    await page.getByRole('button', { name: 'Create' }).click()
    // Close auto-opened text viewer
    await page.locator('button[title="Close"]').click()
    await expect(page.locator('table').getByText('rename-me.md')).toBeVisible()

    await page.locator('table tr').filter({ hasText: 'rename-me.md' }).click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Rename').click()

    const nameInput = page.locator('input[placeholder="New name"]')
    await nameInput.clear()
    await nameInput.fill('renamed-file.md')
    await Promise.all([
      page.waitForResponse(
        (resp) => resp.url().includes('/api/files/rename') && resp.status() === 200,
      ),
      page.getByRole('dialog').getByRole('button', { name: 'Rename', exact: true }).click(),
    ])

    await expect(page.locator('table').getByText('renamed-file.md')).toBeVisible()
    await expect(page.locator('table').getByText('rename-me.md')).not.toBeVisible()
  })

  test('renames a folder via context menu', async ({ page }) => {
    await page.goto('/?dir=Notes')
    await page.locator('table tr').filter({ hasText: 'test-folder' }).click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Rename').click()

    const nameInput = page.locator('input[placeholder="New name"]')
    await nameInput.clear()
    await nameInput.fill('renamed-folder')
    await page.getByRole('dialog').getByRole('button', { name: 'Rename', exact: true }).click()

    await expect(page.locator('table').getByText('renamed-folder')).toBeVisible()
  })

  test('deletes a file via context menu', async ({ page }) => {
    await page.goto('/?dir=Notes')
    await page.locator('table tr').filter({ hasText: 'renamed-file.md' }).click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Delete').click()

    await page.getByRole('button', { name: /Delete/i }).click()
    await expect(page.locator('table').getByText('renamed-file.md')).not.toBeVisible()
  })

  test('deletes a folder via context menu', async ({ page }) => {
    await page.goto('/?dir=Notes')
    await page.locator('table tr').filter({ hasText: 'renamed-folder' }).click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Delete').click()

    await page.getByRole('button', { name: /Delete/i }).click()
    await expect(page.locator('table').getByText('renamed-folder')).not.toBeVisible()
  })

  test('moves a file via context menu', async ({ page }) => {
    await page.goto('/?dir=SharedContent')
    await page.locator('button[title="Create new file"]').click()
    await page.locator('input[placeholder*="File name"]').fill('move-me.txt')
    await page.getByRole('button', { name: 'Create' }).click()
    // Close auto-opened text viewer
    await page.locator('button[title="Close"]').click()
    await expect(page.locator('table').getByText('move-me.txt')).toBeVisible()

    await page.locator('table tr').filter({ hasText: 'move-me.txt' }).click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Move to...').click()

    await page.locator('[role="dialog"]').getByText('subfolder').click()
    await page.getByRole('button', { name: /Move/i }).click()

    await expect(page.locator('table').getByText('move-me.txt')).not.toBeVisible()

    await page.locator('table').getByText('subfolder').first().click()
    await expect(page.locator('table').getByText('move-me.txt')).toBeVisible()
  })

  test('copies a file via context menu', async ({ page }) => {
    await page.goto('/?dir=SharedContent')
    await page.locator('table tr').filter({ hasText: 'public-doc.txt' }).click({ button: 'right' })
    const copyItem = page.locator('[data-slot="context-menu-item"]').getByText('Copy to...')
    await expect(copyItem).toBeVisible()
    await copyItem.click({ noWaitAfter: true })

    await expect(page.locator('[role="dialog"]')).toBeVisible()
    await page.locator('[role="dialog"]').getByText('subfolder').click()
    await page.getByRole('button', { name: /Copy/i }).click()

    await expect(page.locator('table').getByText('public-doc.txt')).toBeVisible()

    await page.locator('table').getByText('subfolder').first().click()
    await expect(page.locator('table').getByText('public-doc.txt')).toBeVisible()
  })

  test('does not show edit options in non-editable folders', async ({ page }) => {
    await page.goto('/?dir=Documents')
    await expect(page.locator('button[title="Create new folder"]')).not.toBeVisible()
    await expect(page.locator('button[title="Create new file"]')).not.toBeVisible()

    await page.locator('table tr').filter({ hasText: 'readme.txt' }).click({ button: 'right' })
    await expect(
      page.locator('[data-slot="context-menu-item"]').getByText('Rename'),
    ).not.toBeVisible()
    await expect(
      page.locator('[data-slot="context-menu-item"]').getByText('Delete'),
    ).not.toBeVisible()
  })
})
