import { defineConfig, devices } from '@playwright/test'
import path from 'path'

process.env.NO_PROXY =
  (process.env.NO_PROXY ? process.env.NO_PROXY + ',' : '') + 'localhost,127.0.0.1'

const batchId = process.env.BATCH_ID
const port = batchId ? 9200 + parseInt(batchId) : 5973
const configFile = batchId
  ? `tests/fixtures/test-config-${batchId}.jsonc`
  : 'tests/fixtures/test-config.jsonc'
const authSessionFile = batchId ? `session-${batchId}.json` : 'session.json'
const authStoragePath = path.join(__dirname, 'tests/fixtures/.auth', authSessionFile)

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  // Batches use isolated ports + media dirs; parallelize by file (not within-file).
  workers: batchId ? 4 : 1,
  reporter: [['line'], ['html', { open: 'never' }]],
  timeout: 15_000,
  expect: { timeout: 10_000 },
  use: {
    baseURL: `http://localhost:${port}`,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'auth-setup',
      testDir: './tests/fixtures',
      testMatch: /auth-setup\.ts/,
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'login',
      testMatch: /login\.spec\.ts/,
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        storageState: authStoragePath,
      },
      dependencies: ['auth-setup'],
      testIgnore: /login\.spec\.ts/,
    },
  ],
  globalSetup: './tests/fixtures/setup.ts',
  globalTeardown: './tests/fixtures/teardown.ts',
  webServer: {
    command: 'npm run build:closkell && node .closkell/build/server/main.mjs',
    url: `http://localhost:${port}`,
    reuseExistingServer: !process.env.CI && !process.env.BATCH_ID,
    timeout: 120_000,
    env: {
      NODE_ENV: 'test',
      PORT: String(port),
      CONFIG_PATH: configFile,
      NO_PROXY: 'localhost,127.0.0.1',
    },
  },
})
