import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['../tests/frontend/unit/**/*.test.ts'],
    exclude: ['../tests/frontend/e2e/**', 'node_modules/**', 'dist/**'],
    environment: 'node',
  },
});
