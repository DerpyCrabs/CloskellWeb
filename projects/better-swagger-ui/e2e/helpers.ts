import type { Page, Route } from '@playwright/test'

export const FIXTURE_PATH = '/fixtures'

export function specPath(path: string) {
  return `${FIXTURE_PATH}${path.startsWith('/') ? path : `/${path}`}`
}

export async function absoluteFixtureUrl(page: Page, path: string) {
  const origin = new URL(page.url()).origin
  return `${origin}${specPath(path)}`
}

export async function loadSpec(page: Page, url: string) {
  await page.goto('/')
  await page.getByTestId('url-input').waitFor({ timeout: 10_000 })

  const target = url.startsWith('http')
    ? url
    : url.startsWith('/')
      ? `${new URL(page.url()).origin}${url}`
      : await absoluteFixtureUrl(page, url)

  await page.getByTestId('url-input').fill(target)
  await page.getByTestId('load-form').evaluate((form) => {
    ;(form as HTMLFormElement).requestSubmit()
  })
  await page.getByTestId('api-title').waitFor({ timeout: 15_000 })
}

export function operationLocator(page: Page, opId: string) {
  return page.getByTestId(`operation-${opId}`)
}

export async function expandOperation(page: Page, opId: string) {
  let op = operationLocator(page, opId)
  if (!(await op.isVisible().catch(() => false))) {
    const sections = page.locator('[data-testid^="tag-section-"]')
    const count = await sections.count()
    for (let i = 0; i < count; i++) {
      const section = sections.nth(i)
      if (await op.isVisible().catch(() => false)) break
      await section.getByRole('button').first().click()
    }
  }

  op = operationLocator(page, opId)
  await op.getByRole('button').first().click()
  await op.getByText('Parameters', { exact: true }).waitFor()
}

export async function openTryItOut(page: Page, opId: string) {
  const op = operationLocator(page, opId)
  await op.getByTestId('try-it-out').click()
}

export async function executeTryItOut(page: Page, opId: string) {
  const op = operationLocator(page, opId)
  await op.getByTestId('execute').click()
}

export async function mockApi(
  page: Page,
  handler: (route: Route) => Promise<void> | void,
) {
  await page.unroute('**/fixtures/mock-api/**')
  await page.route('**/fixtures/mock-api/**', handler)
}

export function specUrl(path: string) {
  return specPath(path.startsWith('/openapi/') ? path : `/openapi/${path}`)
}

export async function clearAuthStorage(page: Page) {
  await page.goto('/')
  await page.evaluate(() => {
    for (const key of Object.keys(localStorage)) {
      if (key.startsWith('better-swagger-auth:')) {
        localStorage.removeItem(key)
      }
    }
  })
}

export async function clearThemeStorage(page: Page) {
  await page.addInitScript(() => {
    localStorage.removeItem('better-swagger-theme')
  })
}
