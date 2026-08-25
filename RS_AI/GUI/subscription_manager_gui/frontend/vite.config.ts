import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
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
