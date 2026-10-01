import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  resolve: {
    alias: {
      '@tauri-apps/api': path.resolve(__dirname, './node_modules/@tauri-apps/api')
    }
  },
  server: {
    fs: {
      allow: ['..']
    }
  }
})
