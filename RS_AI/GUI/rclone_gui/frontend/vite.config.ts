/// <reference types="vitest" />
import { defineConfig } from 'vite';
import { fileURLToPath, URL } from 'node:url';

export default defineConfig({
  resolve: {
    alias: {
      '@tauri-apps/api/core': fileURLToPath(new URL('./node_modules/@tauri-apps/api/core.js', import.meta.url)),
      '@tauri-apps/api': fileURLToPath(new URL('./node_modules/@tauri-apps/api', import.meta.url))
    }
  },
  test: {
    environment: 'jsdom',
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html'],
      include: ['src/**/*.ts', '../bridge/**/*.ts']
    }
  }
});
