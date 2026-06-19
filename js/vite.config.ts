/// <reference types="vitest/config" />
import { defineConfig } from 'vitest/config';

// The wasm-pack `--target web` output loads its module via
// `new URL('..._bg.wasm', import.meta.url)`, which Vite handles natively as an
// asset — no wasm plugin needed for dev, build, preview, or Vitest browser mode.
export default defineConfig({
  test: {
    // Vitest unit/integration suites only. Playwright E2E specs live under
    // tests/e2e and are run by `playwright test`, not Vitest.
    include: ['tests/**/*.test.ts'],
    exclude: ['tests/e2e/**', 'node_modules/**'],
    browser: {
      // The `--target web` build initializes WASM by fetching the .wasm over
      // HTTP, so the public API can only be exercised in a real browser.
      enabled: true,
      provider: 'playwright',
      headless: true,
      instances: [{ browser: 'chromium' }],
    },
  },
});
