// AI-assisted (OpenBot agent)
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  retries: 0,
  workers: 1,
  timeout: 30_000,
  reporter: [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL: 'http://127.0.0.1:3100',
    trace: 'retain-on-failure',
    screenshot: 'on'
  },
  projects: [
    {
      name: 'desktop-chromium',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1280, height: 800 }
      }
    },
    {
      name: 'mobile-chromium',
      use: {
        ...devices['Pixel 7'],
        viewport: { width: 390, height: 844 }
      }
    }
  ],
  webServer: {
    command:
      'sh -c \'export FIELDSHIFT_ROOT="${FIELDSHIFT_ROOT:-$(cd .. && pwd)}"; export FIELDSHIFT_STATIC_DIR="${FIELDSHIFT_STATIC_DIR:-$(pwd)/build}"; export BIND_ADDRESS="127.0.0.1:3100"; export FRONTEND_ORIGIN="http://127.0.0.1:3100"; if [ -x ../backend/target/release/fieldshift-api ]; then exec ../backend/target/release/fieldshift-api; elif [ -x /tmp/fs-target-api/debug/fieldshift-api ]; then exec /tmp/fs-target-api/debug/fieldshift-api; else exec cargo run --manifest-path ../backend/Cargo.toml -p fieldshift-api; fi\'',
    url: 'http://127.0.0.1:3100/v1/health',
    reuseExistingServer: false,
    timeout: 60_000
  }
});
