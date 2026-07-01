import { test, expect } from '@playwright/test'

test.describe('Folder Navigation', () => {
  test('shows all top-level folders at root', async ({ page }) => {
    await page.goto('/')
    const table = page.locator('table')
    await Promise.all(
      ['Videos', 'Music', 'Images', 'Documents', 'Notes', 'SharedContent', 'EmptyFolder'].map(
        (folder) => expect(table.getByText(folder, { exact: true })).toBeVisible(),
      ),
    )
  })

  test('navigates into a folder on click', async ({ page }) => {
    await page.goto('/')
    await page.locator('table').getByText('Videos', { exact: true }).click()
    await page.waitForURL(/dir=Videos/)
    await expect(page.locator('table').getByText('sample.mp4')).toBeVisible()
  })

  test('navigates back via breadcrumbs', async ({ page }) => {
    await page.goto(`/?dir=${encodeURIComponent('Notes/subfolder')}`)
    await expect(page.locator('table').getByText('nested-note.md')).toBeVisible()
    await page.getByRole('button', { name: 'Notes', exact: true }).click()
    await page.waitForURL(/dir=Notes(?:&|$)/)
    await expect(page.locator('table').getByText('welcome.md')).toBeVisible()
  })

  test('breadcrumb folder context menu includes Set icon and workspace actions', async ({
    page,
  }) => {
    await page.goto(`/?dir=${encodeURIComponent('Notes/subfolder')}`)
    await expect(page.locator('table').getByText('nested-note.md')).toBeVisible()
    await page.locator('[data-breadcrumb-path="Notes"]').click({ button: 'right' })
    await expect(page.getByTestId('breadcrumb-menu-set-icon')).toBeVisible()
    await expect(page.getByTestId('breadcrumb-menu-open-workspace')).toBeVisible()
    await expect(page.getByTestId('breadcrumb-menu-open-new-tab')).toBeVisible()
  })

  test('navigates to parent using ".." row', async ({ page }) => {
    await page.goto('/?dir=Videos')
    await page.locator('table').getByText('..').first().click()
    await expect(page.locator('table').getByText('Videos', { exact: true })).toBeVisible()
  })

  test('navigates into nested folders', async ({ page }) => {
    await page.goto('/?dir=Notes')
    await page.locator('table').getByText('subfolder', { exact: true }).first().click()
    await page.waitForURL(/dir=Notes.*subfolder/)
    await expect(page.locator('table').getByText('nested-note.md')).toBeVisible()
  })

  test('switches to grid view and back', async ({ page }) => {
    await page.goto('/?dir=Videos')
    await expect(page.locator('table')).toBeVisible()

    await page.locator('button:has(.lucide-layout-grid)').click()
    await expect(page.locator('table')).not.toBeVisible()
    await expect(page.getByText('sample.mp4')).toBeVisible()

    await page.locator('button:has(.lucide-list)').click()
    await expect(page.locator('table')).toBeVisible()
  })

  test('shows empty state for empty folder', async ({ page }) => {
    await page.goto('/?dir=EmptyFolder')
    await expect(page.getByText('..')).toBeVisible()
    await expect(page.getByTestId('directory-empty')).toBeVisible()
    await expect(page.getByText('This folder is empty')).toBeVisible()
    const dataRows = page.locator('table tbody tr')
    await expect(dataRows).toHaveCount(2)
  })

  test('favorites a file and sees it in Favorites virtual folder', async ({ page }) => {
    await page.goto('/?dir=Documents')
    const row = page.locator('table tr').filter({ hasText: 'readme.txt' })
    await row.hover()
    await Promise.all([
      page.waitForResponse(
        (resp) => resp.url().includes('/api/settings/favorite') && resp.status() === 200,
      ),
      row.locator('button[title="Add to favorites"]').click(),
    ])

    await page.goto('/?dir=Favorites')
    await expect(page.getByText('readme.txt')).toBeVisible()

    // cleanup
    const favRow = page.locator('table tr').filter({ hasText: 'readme.txt' })
    await favRow.hover()
    await Promise.all([
      page.waitForResponse(
        (resp) => resp.url().includes('/api/settings/favorite') && resp.status() === 200,
      ),
      favRow.locator('button[title="Remove from favorites"]').click(),
    ])
  })

  test('tracks views and shows Most Played', async ({ page }) => {
    const res = await page.request.post('/api/stats/views', {
      data: { filePath: 'Documents/readme.txt' },
    })
    expect(res.ok()).toBeTruthy()

    await page.goto('/?dir=Most Played')
    await expect(page.locator('table').getByText('readme.txt')).toBeVisible()
  })

  test('loads folder from direct URL', async ({ page }) => {
    await page.goto('/?dir=Music')
    await expect(page.locator('table').getByText('track.mp3')).toBeVisible()
  })

  test('shows file metadata in list view', async ({ page }) => {
    await page.goto('/?dir=Documents')
    const row = page.locator('table tr').filter({ hasText: 'readme.txt' })
    await expect(row).toBeVisible()
    // File size cell should contain a number
    await expect(row).toContainText(/\d+\s*(B|KB|MB)/)
  })

  test('increments view count when opening a file in list view', async ({ page }) => {
    await page.goto('/?dir=Documents')
    await page.locator('table').getByText('readme.txt', { exact: true }).click()
    await expect(page).toHaveURL(/viewing=/)
    const row = page.locator('table tr').filter({ hasText: 'readme.txt' })
    await expect(row.locator('[data-testid=file-view-count]')).toHaveText(/[1-9]/)
  })

  test('shows virtual folders at root', async ({ page }) => {
    await page.goto('/')
    await expect(page.getByText('Favorites')).toBeVisible()
    await expect(page.getByText('Most Played')).toBeVisible()
  })

  test('context menu Open in Workspace targets workspace with folder dir', async ({ page }) => {
    await page.goto('/')
    await page.locator('table').getByText('Documents', { exact: true }).click({ button: 'right' })
    await expect(
      page.locator('[data-slot="context-menu-item"]').getByText('Open in Workspace'),
    ).toBeVisible()

    await page.evaluate(() => {
      const w = window as Window & { __workspaceMenuOpenUrl?: string }
      w.open = (url?: string | URL) => {
        w.__workspaceMenuOpenUrl = typeof url === 'string' ? url : String(url ?? '')
        return null
      }
    })
    await page.locator('[data-slot="context-menu-item"]').getByText('Open in Workspace').click()
    const captured = await page.evaluate(
      () => (window as Window & { __workspaceMenuOpenUrl?: string }).__workspaceMenuOpenUrl ?? '',
    )
    expect(captured).toMatch(/\/workspace\?dir=Documents(?:&|$)/)
  })

  test('favorites a folder via context menu and removes from Favorites', async ({ page }) => {
    await page.goto('/')
    await page.locator('table').getByText('Notes', { exact: true }).click({ button: 'right' })
    await Promise.all([
      page.waitForResponse(
        (resp) => resp.url().includes('/api/settings/favorite') && resp.status() === 200,
      ),
      page.locator('[data-slot="context-menu-item"]').getByText('Favorite').click(),
    ])

    await page.goto('/?dir=Favorites')
    await expect(page.getByText('Notes').first()).toBeVisible()

    await page.locator('table').getByText('Notes', { exact: true }).click({ button: 'right' })
    await Promise.all([
      page.waitForResponse(
        (resp) => resp.url().includes('/api/settings/favorite') && resp.status() === 200,
      ),
      page.locator('[data-slot="context-menu-item"]').getByText('Unfavorite').click(),
    ])
  })

  test('unsupported file from URL shows dialog and close clears viewing', async ({ page }) => {
    await page.goto('/?dir=Documents')
    await page.locator('table').getByText('unsupported.xyz').click()
    await page.waitForURL(/viewing=/)
    await expect(page.getByRole('heading', { name: 'Unsupported File Type' })).toBeVisible()
    await page.getByRole('button', { name: 'Close' }).click()
    await expect(page).not.toHaveURL(/viewing=/)
    await expect(page.locator('table').getByText('unsupported.xyz')).toBeVisible()
  })
})
