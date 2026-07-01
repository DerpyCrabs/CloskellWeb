import { test, expect } from '@playwright/test'

test.describe('Managing Shares', () => {
  test('creates a share for a file', async ({ page }) => {
    await page.goto('/?dir=Documents')
    await page.locator('table tr').filter({ hasText: 'readme.txt' }).click({ button: 'right' })
    const shareItem = page.locator('[data-slot="context-menu-item"]').getByText('Share')
    await expect(shareItem).toBeVisible()
    await shareItem.click({ noWaitAfter: true })

    await expect(page.getByRole('heading', { name: 'Share Links' })).toBeVisible()
    await page.getByRole('button', { name: 'Create New Share Link' }).click()

    const createBtn = page.getByRole('button', { name: /^Create$/i })
    if (await createBtn.isVisible()) {
      await createBtn.click()
    }

    await expect(page.locator('input[readonly]').first()).toHaveValue(/\/share\//)
  })

  test('creates a share for a folder', async ({ page }) => {
    await page.goto('/')
    await page.locator('table tr').filter({ hasText: 'SharedContent' }).click({ button: 'right' })
    await page.locator('[data-slot="context-menu-item"]').getByText('Share').click()

    await expect(page.getByRole('heading', { name: 'Share Links' })).toBeVisible()
    await page.getByRole('button', { name: 'Create New Share Link' }).click()

    const createBtn = page.getByRole('button', { name: /^Create$/i })
    if (await createBtn.isVisible()) {
      await createBtn.click()
    }

    await expect(page.locator('input[readonly]').first()).toHaveValue(/\/share\//)
  })

  test('creates an editable share with restriction options', async ({ page }) => {
    await page.goto('/')
    await page.locator('table tr').filter({ hasText: 'SharedContent' }).click({ button: 'right' })
    await page
      .locator('[data-slot="context-menu-item"]')
      .getByText(/Share|Manage Share/)
      .click()

    await page.getByRole('button', { name: 'Create New Share Link' }).click()

    const editableCheckbox = page.getByText('Allow editing')
    await expect(editableCheckbox).toBeVisible()
    await editableCheckbox.click()

    await expect(page.getByText('Allow uploads')).toBeVisible()

    const createBtn = page.getByRole('button', { name: /^Create$/i })
    await createBtn.click()

    await expect(page.locator('input[readonly]').first()).toHaveValue(/\/share\//)
  })

  test('share link is displayed and copyable', async ({ page }) => {
    await page.goto('/?dir=Documents')
    await page.locator('table tr').filter({ hasText: 'readme.txt' }).click({ button: 'right' })
    await page
      .locator('[data-slot="context-menu-item"]')
      .getByText(/Share|Manage Share/)
      .click()

    const urlInput = page.locator('input[readonly]').first()
    await expect(urlInput).toBeVisible()
    const value = await urlInput.inputValue()
    expect(value).toContain('/share/')

    await expect(page.locator('button[title="Copy link"]').first()).toBeVisible()
  })

  test('views shares in Shares virtual folder', async ({ page }) => {
    await page.goto('/?dir=Shares')
    const table = page.locator('table')
    await expect(table).toBeVisible()
  })

  test('revokes a share', async ({ page }) => {
    const res = await page.request.post('/api/shares', {
      data: { path: 'Documents/data.json', isDirectory: false },
    })
    const json = await res.json()
    const share = json.share

    await page.goto('/?dir=Shares')
    await expect(page.locator('table')).toBeVisible()

    const row = page.locator('table tr').filter({ hasText: 'data.json' })
    await row.click({ button: 'right' })
    const revokeItem = page.locator('[data-slot="context-menu-item"]').getByText('Revoke Share')
    await expect(revokeItem).toBeVisible()
    await revokeItem.click({ noWaitAfter: true })

    await Promise.all([
      page.waitForResponse((resp) => resp.url().includes('/api/shares/delete')),
      page.getByRole('button', { name: /Revoke/i }).click(),
    ])

    const shareRes = await page.request.get(`/api/share/${share.token}/info`)
    expect(shareRes.ok()).toBeFalsy()
  })

  test('share restrictions disable delete option', async ({ page }) => {
    const res = await page.request.post('/api/shares', {
      data: {
        path: 'SharedContent',
        isDirectory: true,
        editable: true,
        restrictions: { allowDelete: false, allowUpload: true, allowEdit: true },
      },
    })
    const json = await res.json()
    const share = json.share

    const shareUrl = share.passcode
      ? `/share/${share.token}?p=${encodeURIComponent(share.passcode)}`
      : `/share/${share.token}`

    await page.goto(shareUrl)
    await expect(page.getByText('public-doc.txt')).toBeVisible()

    await page.locator('table tr').filter({ hasText: 'public-doc.txt' }).click({ button: 'right' })
    await expect(
      page.locator('[data-slot="context-menu-item"]').getByText('Delete'),
    ).not.toBeVisible()
  })
})
