import { defineConfig, devices } from '@playwright/test';

const webBaseURL = process.env.NVBES_WEB_BASE_URL ?? 'http://127.0.0.1:3001';

export default defineConfig({
  testDir: './e2e',
  timeout: 90_000,
  expect: {
    timeout: 15_000,
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.002,
      threshold: 0.2,
      animations: 'disabled',
    },
  },
  snapshotPathTemplate: '{testDir}/__snapshots__/{testFilePath}/{arg}{ext}',
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  workers: process.env.CI ? 2 : undefined,
  reporter: process.env.CI
    ? [
        ['line'],
        ['junit', { outputFile: 'test-results/identity-web-e2e.xml' }],
        ['html', { open: 'never', outputFolder: 'playwright-report' }],
      ]
    : [['list']],
  outputDir: 'test-results/artifacts',
  use: {
    baseURL: webBaseURL,
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
    locale: 'fr-FR',
    timezoneId: 'Europe/Paris',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      name: 'firefox',
      testIgnore: [/monkey\.spec\.ts$/u],
      use: {
        ...devices['Desktop Firefox'],
      },
    },
    {
      name: 'webkit',
      testIgnore: [/monkey\.spec\.ts$/u],
      use: {
        ...devices['Desktop Safari'],
      },
    },
    {
      name: 'mobile-chromium',
      testIgnore: [/monkey\.spec\.ts$/u],
      use: {
        ...devices['Pixel 7'],
      },
    },
    {
      name: 'mobile-webkit',
      testIgnore: [/monkey\.spec\.ts$/u],
      use: {
        ...devices['iPhone 15'],
      },
    },
  ],
});
