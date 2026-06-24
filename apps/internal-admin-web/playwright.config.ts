import { defineConfig, devices } from '@playwright/test';

const webBaseURL = process.env.NVBES_INTERNAL_ADMIN_WEB_BASE_URL ?? 'http://127.0.0.1:5178';

export default defineConfig({
  testDir: './e2e',
  timeout: 90_000,
  expect: {
    timeout: 15_000,
  },
  fullyParallel: false,
  workers: 1,
  reporter: [['list']],
  use: {
    baseURL: webBaseURL,
    trace: 'on-first-retry',
  },
  webServer: {
    command: 'pnpm --dir apps/internal-admin-web dev',
    cwd: '../..',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    url: webBaseURL,
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
      },
    },
  ],
});
