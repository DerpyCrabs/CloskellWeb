import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  maxFailures: 1,
  retries: process.env.CI ? 1 : 0,
  timeout: 20000,
  workers: 1,
  use: {
    baseURL: "http://127.0.0.1:4184",
    trace: "on-first-retry"
  },
  webServer: {
    command: "vite --host 127.0.0.1 --port 4184 --strictPort",
    url: "http://127.0.0.1:4184",
    reuseExistingServer: !process.env.CI,
    timeout: 30000
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] }
    }
  ]
});
