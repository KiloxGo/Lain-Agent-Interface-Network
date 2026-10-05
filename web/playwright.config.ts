import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/e2e', fullyParallel: false, workers: 1, timeout: 90_000,
  reporter: 'list', outputDir: 'test-results',
  use: { browserName: 'chromium', viewport: { width: 1440, height: 1000 }, trace: 'off', screenshot: 'off' },
});
