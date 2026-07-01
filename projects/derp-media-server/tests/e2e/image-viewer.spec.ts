import { test, expect } from '@playwright/test'

test.describe('Image Viewer', () => {
  test('image thumbnails appear in grid view', async ({ page }) => {
    await page.goto('/?dir=Images')
    await page.locator('button:has(.lucide-layout-grid)').click()
    const card = page
      .locator('[data-testid=file-browser] .file-browser-grid [role=button]')
      .filter({
        hasText: 'photo.jpg',
      })
    const thumb = card.locator('[data-testid=file-browser-image-thumbnail]')
    await expect(thumb).toBeVisible()
    await expect(thumb).toHaveAttribute('src', /\/api\/thumbnail\//)

    const response = await page.request.get('/api/thumbnail/Images/photo.jpg')
    expect(response.headers()['content-type']).toContain('image/jpeg')

    await page.getByRole('button', { name: 'List view' }).click()
  })

  test('opens image viewer when clicking an image file', async ({ page }) => {
    await page.goto('/?dir=Images')
    await page.locator('table').getByText('photo.jpg').click()
    await expect(page.locator('img[alt="photo.jpg"]')).toBeVisible()
  })

  test('reflects viewing image in URL', async ({ page }) => {
    await page.goto('/?dir=Images')
    await page.locator('table').getByText('photo.jpg').click()
    await expect(page).toHaveURL(/viewing=Images.*photo\.jpg/)
  })

  test('shows zoom controls', async ({ page }) => {
    await page.goto('/?dir=Images&viewing=Images%2Fphoto.jpg')
    await expect(page.locator('button:has(.lucide-zoom-in)')).toBeVisible()
    await expect(page.locator('button:has(.lucide-zoom-out)')).toBeVisible()
    await expect(page.getByText('Fit')).toBeVisible()
  })

  test('zooms in and out on button click', async ({ page }) => {
    await page.goto('/?dir=Images&viewing=Images%2Fphoto.jpg')
    await expect(page.getByText('Fit')).toBeVisible()

    await page.locator('button:has(.lucide-zoom-in)').click()
    await expect(page.getByText('125%')).toBeVisible()

    await page.locator('button:has(.lucide-zoom-out)').click()
    await expect(page.getByText('Fit').or(page.getByText('100%'))).toBeVisible()
  })

  test('rotates image via rotate button', async ({ page }) => {
    await page.goto('/?dir=Images&viewing=Images%2Fphoto.jpg')
    const img = page.locator('img[alt="photo.jpg"]')
    await expect(img).toBeVisible()

    await page.locator('button:has(.lucide-rotate-cw)').click()
    const transform = await img.evaluate((el) => el.style.transform)
    expect(transform).toContain('rotate(90deg)')
  })

  test('fit-to-screen button resets zoom and rotation', async ({ page }) => {
    await page.goto('/?dir=Images&viewing=Images%2Fphoto.jpg')

    await page.locator('button:has(.lucide-zoom-in)').click()
    await page.locator('button:has(.lucide-rotate-cw)').click()

    await page.locator('button[title="Fit to screen"]').click()
    await expect(page.getByText('Fit')).toBeVisible()
    const img = page.locator('img[alt="photo.jpg"]')
    const transform = await img.evaluate((el) => el.style.transform)
    expect(transform).toContain('rotate(0deg)')
  })

  test('navigates to next image with ArrowRight key', async ({ page }) => {
    await page.goto('/?dir=Images')
    await page.locator('table').getByText('photo.jpg').click()
    await expect(page.locator('img[alt="photo.jpg"]')).toBeVisible()
    await expect(page.getByText('1 of 2')).toBeVisible()

    await page.evaluate(() =>
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true })),
    )
    await expect(page.locator('img[alt="photo.png"]')).toBeVisible()
    await expect(page.getByText('2 of 2')).toBeVisible()
  })

  test('navigates to previous image with ArrowLeft key', async ({ page }) => {
    await page.goto('/?dir=Images')
    await page.locator('table').getByText('photo.png').click()
    await expect(page.locator('img[alt="photo.png"]')).toBeVisible()

    await page.evaluate(() =>
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true })),
    )
    await expect(page.locator('img[alt="photo.jpg"]')).toBeVisible()
  })

  test('shows image counter', async ({ page }) => {
    await page.goto('/?dir=Images&viewing=Images%2Fphoto.jpg')
    await expect(page.getByText('1 of 2')).toBeVisible()
  })

  test('closing viewer returns to file list', async ({ page }) => {
    await page.goto('/?dir=Images&viewing=Images%2Fphoto.jpg')
    await expect(page.locator('img[alt="photo.jpg"]')).toBeVisible()

    await page.locator('button:has(.lucide-x)').click()
    await expect(page.locator('img[alt="photo.jpg"]')).not.toBeVisible()
    await expect(page.locator('table').getByText('photo.jpg')).toBeVisible()
    await expect(page).not.toHaveURL(/viewing=/)
  })
})
