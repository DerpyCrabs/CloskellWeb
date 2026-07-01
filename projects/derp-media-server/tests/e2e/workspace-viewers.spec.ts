import { test, expect, type BrowserContext, type Page } from '@playwright/test'
import {
  getWindowGroups,
  gotoWorkspace,
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

function getBrowserContent(page: Page) {
  return getWindowGroups(page).first().locator('.workspace-window-content')
}

async function openFileFromBrowser(page: Page, folder: string, fileName: string) {
  const content = getBrowserContent(page)
  await content.getByText(folder, { exact: true }).click()
  const fileRow = content.locator('table').getByText(fileName)
  await expect(fileRow).toBeVisible()
  await fileRow.click()
  await expect(getWindowGroups(page)).toHaveCount(2)
  return getWindowGroups(page).nth(1).locator('.workspace-window-content')
}

test.describe('Workspace File Browser', () => {
  test('navigation: breadcrumbs, parent row, and nested folders', async () => {
    await test.step('navigates back via breadcrumbs', async () => {
      await gotoWorkspace(page)
      const content = getBrowserContent(page)
      await content.getByText('Documents', { exact: true }).click()
      await expect(content.getByText('readme.txt')).toBeVisible()
      await content.getByRole('button', { name: 'Home' }).click()
      const table = content.locator('table')
      await Promise.all(
        ['Videos', 'Music', 'Images', 'Documents'].map((folder) =>
          expect(table.getByText(folder, { exact: true })).toBeVisible(),
        ),
      )
    })

    await test.step('navigates to parent using ".." row', async () => {
      await gotoWorkspace(page)
      const content = getBrowserContent(page)
      await content.getByText('Videos', { exact: true }).click()
      await expect(content.getByText('sample.mp4')).toBeVisible()
      await content.getByText('..').first().click()
      await expect(content.getByText('Videos', { exact: true })).toBeVisible()
    })

    await test.step('navigates into nested folders', async () => {
      await gotoWorkspace(page)
      const content = getBrowserContent(page)
      await content.getByText('Notes', { exact: true }).click()
      const subfolderRow = content.locator('table').getByText('subfolder', { exact: true })
      await expect(subfolderRow).toBeVisible()
      await subfolderRow.click()
      await expect(content.locator('table').getByText('nested-note.md')).toBeVisible()
    })
  })

  test('context menus: row Set icon and breadcrumb Set icon', async () => {
    await test.step('workspace browser row context menu includes Set icon', async () => {
      await gotoWorkspace(page)
      const content = getBrowserContent(page)
      await content
        .locator('table')
        .getByText('Documents', { exact: true })
        .click({ button: 'right' })
      await expect(
        page.locator('[data-slot="file-row-context-menu"]').getByText('Set icon'),
      ).toBeVisible()
    })

    await test.step('workspace breadcrumb folder context menu includes Set icon', async () => {
      await gotoWorkspace(page)
      const content = getBrowserContent(page)
      await content.getByText('Notes', { exact: true }).click()
      await content.locator('table').getByText('subfolder', { exact: true }).click()
      await expect(content.locator('table').getByText('nested-note.md')).toBeVisible()
      let notesBreadcrumb = content.locator('[data-breadcrumb-path="Notes"]')
      if ((await notesBreadcrumb.count()) === 0) {
        await content.locator('[data-breadcrumb-segment="path-picker"]').click()
        notesBreadcrumb = page
          .getByTestId('breadcrumb-path-menu')
          .locator('[data-breadcrumb-path="Notes"]')
      }
      await notesBreadcrumb.dispatchEvent('contextmenu')
      await expect(page.getByTestId('breadcrumb-menu-set-icon')).toBeVisible()
    })
  })

  test('workspace browser shows KB recent strip, search, and inline create in Notes', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    const notesRow = content.locator('table').getByText('Notes', { exact: true })
    await expect(notesRow).toBeVisible()
    await notesRow.click()
    await expect(content.getByTestId('kb-recent-strip')).toBeVisible()
    await expect(content.getByRole('button', { name: 'Open search' })).toBeVisible()
    await content.getByRole('button', { name: 'Open search' }).click()
    await expect(page.getByPlaceholder('Search notes...')).toBeVisible()
    await expect(content.locator('table')).toBeVisible()
    const dashedActions = content.locator('button.border-dashed')
    await expect(dashedActions.filter({ hasText: 'New file' })).toBeVisible()
    await expect(dashedActions.filter({ hasText: 'New folder' })).toBeVisible()
  })

  test('switches to grid view and back', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)

    await content.getByText('Videos', { exact: true }).click()
    await expect(content.getByText('sample.mp4')).toBeVisible()

    const listBtn = content.locator('button:has(.lucide-list)')
    if (
      await content
        .locator('table')
        .isVisible()
        .catch(() => false)
    ) {
      // Already in list mode
    } else {
      await listBtn.click()
    }
    await expect(content.locator('table')).toBeVisible()

    await content.locator('button:has(.lucide-layout-grid)').click()
    await expect(content.locator('table')).not.toBeVisible()
    await expect(content.getByText('sample.mp4')).toBeVisible()

    await listBtn.click()
    await expect(content.locator('table')).toBeVisible()
  })

  test('shows file metadata in list view', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)

    await content.getByText('Documents', { exact: true }).click()
    const row = content.locator('table tr').filter({ hasText: 'readme.txt' })
    await expect(row).toBeVisible()
    await expect(row).toContainText(/\d+\s*(B|KB|MB)/)
  })

  test('unsupported file dialog is contained inside file browser window', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await content.getByText('Documents', { exact: true }).click()
    await content.locator('table').getByText('unsupported.xyz').click()
    await page.waitForTimeout(100)

    const dialogMessage = page.getByText('This file type cannot be previewed.')
    await expect(dialogMessage).toBeVisible()
    const windowGroup = getWindowGroups(page).first()
    await expect(windowGroup.locator('text=This file type cannot be previewed.')).toBeVisible()
  })
})

test.describe('Workspace clipboard paste', () => {
  test('paste in Notes creates file and opens viewer with clipboard text', async () => {
    await gotoWorkspace(page)
    const content = getBrowserContent(page)
    await content.locator('table').getByText('Notes', { exact: true }).click()
    const dropZone = content.getByTestId('workspace-upload-drop-zone')
    await dropZone.focus()
    const marker = `ws paste e2e ${Date.now()}`
    await page.evaluate(async (text) => {
      await navigator.clipboard.write([
        new ClipboardItem({
          'text/plain': new Blob([text], { type: 'text/plain' }),
        }),
      ])
    }, marker)
    await page.keyboard.press('Control+v')
    await expect(page.getByRole('heading', { name: /Paste Text/i })).toBeVisible()
    await page.getByRole('button', { name: 'Paste' }).click()
    await expect(getWindowGroups(page)).toHaveCount(2)
    const viewer = getWindowGroups(page).nth(1).locator('.workspace-window-content')
    await expect(viewer.locator('textarea')).toBeVisible({ timeout: 10_000 })
    await expect(viewer.locator('textarea')).toHaveValue(marker)
  })
})

test.describe('Workspace Image Viewer', () => {
  test('image viewer: controls, fit, counter, and keyboard navigation', async () => {
    await gotoWorkspace(page)
    const viewer = await openFileFromBrowser(page, 'Images', 'photo.jpg')

    await test.step('shows image and zoom controls', async () => {
      await expect(viewer.locator('img[alt="photo.jpg"]')).toBeVisible()
      await expect(viewer.locator('button:has(.lucide-zoom-in)')).toBeVisible()
      await expect(viewer.locator('button:has(.lucide-zoom-out)')).toBeVisible()
      await expect(viewer.getByText('Fit')).toBeVisible()
    })

    await test.step('zooms in and out on button click', async () => {
      await expect(viewer.getByText('Fit')).toBeVisible()
      await viewer.locator('button:has(.lucide-zoom-in)').click()
      await expect(viewer.getByText('125%')).toBeVisible()
      await viewer.locator('button:has(.lucide-zoom-out)').click()
      await expect(viewer.getByText('Fit').or(viewer.getByText('100%'))).toBeVisible()
    })

    await test.step('rotates image via rotate button', async () => {
      const img = viewer.locator('img[alt="photo.jpg"]')
      await expect(img).toBeVisible()
      await viewer.locator('button:has(.lucide-rotate-cw)').click()
      const transform = await img.evaluate((el) => el.style.transform)
      expect(transform).toContain('rotate(90deg)')
    })

    await test.step('fit-to-screen resets zoom and rotation', async () => {
      await viewer.locator('button:has(.lucide-zoom-in)').click()
      await viewer.locator('button:has(.lucide-rotate-cw)').click()
      await viewer.locator('button[title="Fit to screen"]').click()
      await expect(viewer.getByText('Fit')).toBeVisible()
      const img = viewer.locator('img[alt="photo.jpg"]')
      const transform = await img.evaluate((el) => el.style.transform)
      expect(transform).toContain('rotate(0deg)')
    })

    await test.step('shows image counter and navigates to next via keyboard', async () => {
      await expect(viewer.getByText('1 of 2')).toBeVisible()
      await viewer.click()
      await page.keyboard.press('ArrowRight')
      const nextViewer = getWindowGroups(page).last().locator('.workspace-window-content')
      await expect(nextViewer.locator('img[alt="photo.png"]')).toBeVisible()
      await expect(nextViewer.getByText('2 of 2')).toBeVisible()
    })

    await test.step('navigates to previous image via keyboard from photo.png', async () => {
      await gotoWorkspace(page)
      const viewerPng = await openFileFromBrowser(page, 'Images', 'photo.png')
      await expect(viewerPng.locator('img[alt="photo.png"]')).toBeVisible()
      await viewerPng.click()
      await page.keyboard.press('ArrowLeft')
      const prevViewer = getWindowGroups(page).last().locator('.workspace-window-content')
      await expect(prevViewer.locator('img[alt="photo.jpg"]')).toBeVisible()
    })
  })

  test('image viewer: arrow keys do not navigate while a search field is focused', async () => {
    await gotoWorkspace(page)
    await openFileFromBrowser(page, 'Images', 'photo.jpg')
    const viewer = getWindowGroups(page).nth(1).locator('.workspace-window-content')
    await expect(viewer.locator('img[alt="photo.jpg"]')).toBeVisible()
    await expect(viewer.getByText('1 of 2')).toBeVisible()

    const browserGroup = getWindowGroups(page).first()
    await browserGroup.locator('[data-workspace-tab-id]').first().click()
    const browser = getBrowserContent(page)
    await browser.getByRole('button', { name: 'Home' }).click()
    await browser.getByText('Notes', { exact: true }).click()
    await browser.getByRole('button', { name: 'Open search' }).click()
    const search = page.getByPlaceholder('Search notes...')
    await expect(search).toBeVisible()
    await search.focus()
    await page.keyboard.press('ArrowRight')

    await expect(viewer.locator('img[alt="photo.jpg"]')).toBeVisible()
    await expect(viewer.getByText('1 of 2')).toBeVisible()
  })
})

test.describe('Workspace PDF Viewer', () => {
  test('PDF viewer embed and toolbar actions', async () => {
    await gotoWorkspace(page)
    const viewer = await openFileFromBrowser(page, 'Documents', 'sample.pdf')
    await test.step('opens PDF embed', async () => {
      await expect(viewer.locator('embed[type="application/pdf"]')).toBeVisible()
    })
    await test.step('shows download and open-in-new-tab buttons', async () => {
      await expect(viewer.locator('button[title="Download"]')).toBeVisible()
      await expect(viewer.locator('button[title="Open in new tab"]')).toBeVisible()
    })
  })
})

test.describe('Workspace Text Viewer', () => {
  test('readme.txt: content, metadata, and read-only toolbar', async () => {
    await gotoWorkspace(page)
    const viewer = await openFileFromBrowser(page, 'Documents', 'readme.txt')
    await test.step('displays text content', async () => {
      await expect(viewer.getByText('This is a test readme file')).toBeVisible()
    })
    await test.step('shows file type and line count', async () => {
      await expect(viewer.getByText('TXT')).toBeVisible()
      await expect(viewer.getByText(/\d+ lines/)).toBeVisible()
    })
    await test.step('does not show edit button for non-editable folders', async () => {
      await expect(viewer.getByRole('button', { name: 'Edit' })).not.toBeVisible()
    })
    await test.step('copy button exists', async () => {
      await expect(viewer.locator('button[title="Copy to clipboard"]')).toBeVisible()
    })
  })

  test('displays JSON files', async () => {
    await gotoWorkspace(page)
    const viewer = await openFileFromBrowser(page, 'Documents', 'data.json')
    await expect(viewer.getByText('"name"')).toBeVisible()
    await expect(viewer.getByText('"test"')).toBeVisible()
  })

  test('renders markdown headings and formatting', async () => {
    await gotoWorkspace(page)
    const viewer = await openFileFromBrowser(page, 'Documents', 'notes.md')
    await expect(viewer.locator('h1:has-text("Test Notes")')).toBeVisible()
    await expect(viewer.locator('strong:has-text("markdown")')).toBeVisible()
  })

  test('markdown image fullscreen overlay open and close paths', async () => {
    await gotoWorkspace(page)
    const viewer = await openFileFromBrowser(page, 'Documents', 'image-note.md')
    const overlay = viewer.locator('[role="dialog"][aria-label="View image fullscreen"]')

    await test.step('opens fullscreen from image click', async () => {
      const img = viewer.locator('img[alt="photo"]')
      await expect(img).toBeVisible()
      await img.click()
      await expect(overlay).toBeVisible()
    })

    await test.step('closes on Escape', async () => {
      await page.keyboard.press('Escape')
      await expect(overlay).not.toBeVisible()
    })

    await test.step('closes on backdrop click', async () => {
      await viewer.locator('img[alt="photo"]').click()
      await expect(overlay).toBeVisible()
      await overlay.click({ position: { x: 10, y: 10 } })
      await expect(overlay).not.toBeVisible()
    })

    await test.step('closes on close button', async () => {
      await viewer.locator('img[alt="photo"]').click()
      await expect(overlay).toBeVisible()
      await overlay.getByRole('button', { name: 'Close' }).click()
      await expect(overlay).not.toBeVisible()
    })
  })

  test('Notes todo.md: edit mode, content, and read-only toggle', async () => {
    await gotoWorkspace(page)
    const viewer = await openFileFromBrowser(page, 'Notes', 'todo.md')

    await test.step('auto-enters edit mode in editable folders', async () => {
      await expect(viewer.locator('textarea')).toBeVisible()
      await expect(viewer.getByRole('button', { name: 'Read only' })).toBeVisible()
    })

    await test.step('shows textarea with file content', async () => {
      const content = await viewer.locator('textarea').inputValue()
      expect(content).toContain('Todo List')
    })

    await test.step('switches between edit and read-only mode', async () => {
      await viewer.getByRole('button', { name: 'Read only' }).click()
      await expect(viewer.locator('textarea')).not.toBeVisible()
      await expect(viewer.locator('h1:has-text("Todo List")')).toBeVisible()
      await viewer.getByRole('button', { name: 'Edit' }).click()
      await expect(viewer.locator('textarea')).toBeVisible()
    })
  })

  test('saves edits and persists changes', async () => {
    await gotoWorkspace(page)
    const viewer = await openFileFromBrowser(page, 'Notes', 'todo.md')
    const textarea = viewer.locator('textarea')
    await expect(textarea).toBeVisible({ timeout: 10_000 })

    await textarea.fill('# Updated Todo\n\n- Brand new item\n')

    const viewerWindow = page
      .locator(WORKSPACE_VISIBLE_WINDOW_GROUP)
      .filter({ has: page.locator('.workspace-window-content').locator('textarea') })
      .first()
    await Promise.all([
      page.waitForResponse(
        (resp) => resp.url().includes('/api/files/edit') && resp.status() === 200,
      ),
      viewerWindow.locator('[data-testid="window-drag-handle"]').click(),
    ])

    const closeBtn = viewerWindow.locator('.workspace-window-buttons button:has(.lucide-x)')
    await closeBtn.click()
    await expect(getWindowGroups(page)).toHaveCount(1)

    const content = getBrowserContent(page)
    await content.locator('table').getByText('todo.md').click()
    await expect(getWindowGroups(page)).toHaveCount(2)

    const newViewer = page
      .locator(WORKSPACE_VISIBLE_WINDOW_GROUP)
      .filter({ has: page.locator('.workspace-window-content textarea') })
      .first()
      .locator('.workspace-window-content')
    await expect(newViewer.locator('textarea')).toBeVisible({ timeout: 10_000 })
    const saved = await newViewer.locator('textarea').inputValue()
    expect(saved).toContain('Updated Todo')
    expect(saved).toContain('Brand new item')

    await newViewer
      .locator('textarea')
      .fill('# Todo List\n\n- [ ] First task\n- [ ] Second task\n- [x] Done task\n')
    await Promise.all([
      page.waitForResponse(
        (resp) => resp.url().includes('/api/files/edit') && resp.status() === 200,
      ),
      getWindowGroups(page).nth(1).locator('[data-testid="window-drag-handle"]').click(),
    ])
  })
})
