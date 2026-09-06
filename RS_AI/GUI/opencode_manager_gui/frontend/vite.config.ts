import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@tauri-apps/api': path.resolve(__dirname, './node_modules/@tauri-apps/api')
    }
  },
  server: {
    // Cổng khác subscription_manager (5173) để chạy song song hai app dev được.
    port: 5174,
    strictPort: true,
    fs: {
      allow: ['..']
    }
  }
})
