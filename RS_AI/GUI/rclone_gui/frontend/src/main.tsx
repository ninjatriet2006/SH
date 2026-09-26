/*
[INTEGRITY NOTES]
- Mục đích: Entry point chính của ứng dụng React.
- Trách nhiệm: Gắn App Component vào root DOM với StrictMode và HashRouter.
- Tương tác: Import `./index.css` và `./App.tsx`.
*/

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { HashRouter } from 'react-router-dom';
import App from './App';
import './index.css';

const rootElement = document.getElementById('root');
if (rootElement) {
  createRoot(rootElement).render(
    <StrictMode>
      <HashRouter>
        <App />
      </HashRouter>
    </StrictMode>,
  );
}
