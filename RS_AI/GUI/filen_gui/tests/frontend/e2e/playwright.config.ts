import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E for filen_gui frontend.
 * The app is a Tauri shell; in-browser the Tauri IPC is mocked via
 * addInitScript (see support/tauri-mock.ts) so the full UI flow can be driven
 * against the Vite production build served by `vite preview`.
 */
export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  workers: 1,
  timeout: 60_000,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: 'http://127.0.0.1:4173',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    command: 'npm --prefix ../../../frontend run preview -- --port 4173 --strictPort',
    url: 'http://127.0.0.1:4173',
    reuseExistingServer: true,
    timeout: 120_000,
  },
});
