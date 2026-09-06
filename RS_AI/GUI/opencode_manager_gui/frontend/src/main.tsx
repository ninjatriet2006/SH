import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { HashRouter } from 'react-router-dom'
import './index.css'
import App from './App.tsx'

// HashRouter (không phải BrowserRouter): trong Tauri, frontend được nạp qua
// giao thức tuỳ biến nên điều hướng theo đường dẫn thật sẽ 404 khi tải lại.
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <HashRouter>
      <App />
    </HashRouter>
  </StrictMode>,
)
