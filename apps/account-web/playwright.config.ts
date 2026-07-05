import { defineConfig, devices } from '@playwright/test';

const webBaseURL = process.env.NVBES_WEB_BASE_URL;
if (!webBaseURL) {
  throw new Error('NVBES_WEB_BASE_URL is required for account-web browser tests.');
}

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
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
      },
    },
  ],
});
