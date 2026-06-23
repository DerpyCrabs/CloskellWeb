import { defineConfig, devices } from '@playwright/test'

const localNoProxy = ['127.0.0.1', 'localhost', '::1']
const existingNoProxy = process.env.NO_PROXY || process.env.no_proxy || ''
const noProxy = [...new Set([...existingNoProxy.split(',').filter(Boolean), ...localNoProxy])].join(',')
process.env.NO_PROXY = noProxy
process.env.no_proxy = noProxy

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  use: {
    baseURL: 'http://127.0.0.1:5180',
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'npm run dev -- --port 5180 --strictPort',
    url: 'http://127.0.0.1:5180',
    reuseExistingServer: false,
    timeout: 120_000,
  },
})
