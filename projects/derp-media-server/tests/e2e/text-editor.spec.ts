import { test, expect } from '@playwright/test'

test.describe('Text Editor', () => {
  test('opens text viewer when clicking a text file', async ({ page }) => {
    await page.goto('/?dir=Documents')
    await page.locator('table').getByText('readme.txt').click()
    await page.waitForURL(/viewing=/)
    await expect(page.getByText('This is a test readme file')).toBeVisible()
  })

  test('renders markdown headings and formatting', async ({ page }) => {
    await page.goto(`/?dir=Documents&viewing=${encodeURIComponent('Documents/notes.md')}`)
    await expect(page.locator('h1:has-text("Test Notes")')).toBeVisible()
    await expect(page.locator('strong:has-text("markdown")')).toBeVisible()
    await expect(page.locator('a[href="https://example.com"]')).toBeVisible()
  })

  test('markdown image click opens fullscreen overlay', async ({ page }) => {
    await page.goto(`/?dir=Documents&viewing=${encodeURIComponent('Documents/image-note.md')}`)
    const img = page.locator('img[alt="photo"]')
    await expect(img).toBeVisible()
    await img.click()
    const overlay = page.locator('[role="dialog"][aria-label="View image fullscreen"]')
    await expect(overlay).toBeVisible()
  })

  test('does not show edit button for non-editable folders', async ({ page }) => {
    await page.goto(`/?dir=Documents&viewing=${encodeURIComponent('Documents/readme.txt')}`)
    await expect(page.getByText('This is a test readme file')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Edit' })).not.toBeVisible()
  })

  test('auto-enters edit mode in editable folders', async ({ page }) => {
    await page.goto(`/?dir=Notes&viewing=${encodeURIComponent('Notes/todo.md')}`)
    await expect(page.locator('textarea')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Read only' })).toBeVisible()
  })

  test('shows textarea with file content in edit mode', async ({ page }) => {
    await page.goto(`/?dir=Notes&viewing=${encodeURIComponent('Notes/todo.md')}`)
    await expect(page.locator('textarea')).toBeVisible()
    const content = await page.locator('textarea').inputValue()
    expect(content).toContain('Todo List')
  })

  test('saves edits and persists changes', async ({ page }) => {
    await page.goto(`/?dir=Notes&viewing=${encodeURIComponent('Notes/todo.md')}`)
    const textarea = page.locator('textarea')
    const closeButton = page.locator('button[title="Close"]')
    await expect(textarea).toBeVisible()
    await textarea.fill('# Updated Todo\n\n- Brand new item\n')
    // Blur triggers immediate auto-save; wait for the save request instead of sleeping.
    await Promise.all([
      page.waitForResponse(
        (resp) => resp.url().includes('/api/files/edit') && resp.status() === 200,
      ),
      closeButton.focus(),
    ])

    // Close and reopen
    await closeButton.click()
    await page.locator('table').getByText('todo.md').click()
    await expect(page.locator('textarea')).toBeVisible()
    const content = await page.locator('textarea').inputValue()
    expect(content).toContain('Updated Todo')
    expect(content).toContain('Brand new item')

    // Restore original content
    await page
      .locator('textarea')
      .fill('# Todo List\n\n- [ ] First task\n- [ ] Second task\n- [x] Done task\n')
    await Promise.all([
      page.waitForResponse(
        (resp) => resp.url().includes('/api/files/edit') && resp.status() === 200,
      ),
      closeButton.focus(),
    ])
  })

  test('closes text viewer returns to file list', async ({ page }) => {
    await page.goto(`/?dir=Documents&viewing=${encodeURIComponent('Documents/readme.txt')}`)
    await expect(page.getByText('This is a test readme file')).toBeVisible()
    await page.locator('button[title="Close"]').click()
    await expect(page).not.toHaveURL(/viewing=/)
    await expect(page.locator('table').getByText('readme.txt')).toBeVisible()
  })

  test('displays JSON files', async ({ page }) => {
    await page.goto(`/?dir=Documents&viewing=${encodeURIComponent('Documents/data.json')}`)
    await expect(page).toHaveURL(/viewing=.*data\.json/)
    await expect(page.locator('button[title="Close"]')).toBeVisible()
    await expect(page.getByText('"name"')).toBeVisible()
    await expect(page.getByText('"test"')).toBeVisible()
  })

  test('copy-to-clipboard button exists', async ({ page }) => {
    await page.goto(`/?dir=Documents&viewing=${encodeURIComponent('Documents/readme.txt')}`)
    await expect(page.locator('button[title="Copy to clipboard"]')).toBeVisible()
  })

  test('switches between edit and read-only mode', async ({ page }) => {
    await page.goto(`/?dir=Notes&viewing=${encodeURIComponent('Notes/todo.md')}`)
    await expect(page.locator('textarea')).toBeVisible()

    await page.getByRole('button', { name: 'Read only' }).click()
    await expect(page.locator('textarea')).not.toBeVisible()
    await expect(page.locator('h1:has-text("Todo List")')).toBeVisible()

    await page.getByRole('button', { name: 'Edit' }).click()
    await expect(page.locator('textarea')).toBeVisible()
  })

  test('with auto-save off, blur does not persist edits', async ({ page }) => {
    const pathEnc = encodeURIComponent('Notes/autosave-parity.txt')
    await page.goto(`/?dir=Notes&viewing=${pathEnc}`)
    const textarea = page.locator('textarea')
    await expect(textarea).toBeVisible()

    let editResponses = 0
    page.on('response', (resp) => {
      if (resp.url().includes('/api/files/edit') && resp.status() === 200) editResponses += 1
    })

    await Promise.all([
      page.waitForResponse((r) => r.url().includes('/api/settings/autoSave') && r.status() === 200),
      page.getByRole('button', { name: 'Auto-save' }).click(),
    ])
    await textarea.fill('autosave off probe\n\nunique-string-xyz-123\n')

    await page.locator('button[title="Close"]').focus()
    await page.waitForTimeout(2300)

    expect(editResponses).toBe(0)

    await page.locator('button[title="Close"]').click()
    await page.locator('table').getByText('autosave-parity.txt').click()
    await expect(textarea).toBeVisible()
    expect(await textarea.inputValue()).not.toContain('unique-string-xyz-123')

    await textarea.fill('Autosave parity initial content for e2e only.\n')
    await Promise.all([
      page.waitForResponse(
        (resp) => resp.url().includes('/api/files/edit') && resp.status() === 200,
      ),
      page.getByRole('button', { name: 'Save', exact: true }).click(),
    ])
  })
})
