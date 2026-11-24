// AI-assisted (OpenBot agent)
import { defineConfig } from '../../../../frontend/node_modules/@playwright/test/index.mjs';
export default defineConfig({
  testDir: process.env.SHIELD_TEST_DIR,
  fullyParallel: false, retries: 0, workers: 1, timeout: 30_000,
  forbidOnly: true,
  reporter: [['list'], ['json', { outputFile: process.env.SHIELD_EVIDENCE + '/playwright.json' }]],
  outputDir: process.env.SHIELD_EVIDENCE + '/browser',
  use: { baseURL: process.env.SHIELD_URL, trace: 'on', screenshot: 'on' },
  projects: [
    { name: 'desktop-chromium', use: { browserName: 'chromium', viewport: { width: 1280, height: 800 } } },
    { name: 'mobile-chromium', use: { browserName: 'chromium', viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true } }
  ]
});
