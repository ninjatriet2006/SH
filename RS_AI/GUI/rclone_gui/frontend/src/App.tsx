/*
[INTEGRITY NOTES]
- Mục đích: Root Layout Component điều phối Routing và khởi tạo hệ thống.
- Trách nhiệm: Nạp cấu hình giao diện (Theme/Font/Lang), khởi chạy watcher và hàng đợi job, hiển thị Sidebar và Router views.
- Tương tác: Dùng `react-router-dom`, `useAppearanceStore`, `useExplorerStore`, `useJobsStore`.
*/

import React, { useEffect } from 'react';
import { Route, Routes } from 'react-router-dom';
import { Sidebar } from './components/Sidebar';
import { ExplorerPage } from './pages/ExplorerPage';
import { MountsPage } from './pages/MountsPage';
import { RemotesPage } from './pages/RemotesPage';
import { SettingsPage } from './pages/SettingsPage';
import { TransfersPage } from './pages/TransfersPage';
import { TrashPage } from './pages/TrashPage';
import { useAppearanceStore } from './store/useAppearanceStore';
import { useExplorerStore } from './store/useExplorerStore';
import { useJobsStore } from './store/useJobsStore';

export const App: React.FC = () => {
  const initAppearance = useAppearanceStore((state) => state.initAppearance);
  const isAppearanceLoading = useAppearanceStore((state) => state.isLoading);
  const initExplorer = useExplorerStore((state) => state.initExplorer);
  const initSubscription = useJobsStore((state) => state.initSubscription);

  useEffect(() => {
    const bootstrap = async () => {
      try {
        await initAppearance();
        await initExplorer();
        await initSubscription();
      } catch (err) {
        console.error('Lỗi bootstrap app:', err);
      }
    };
    bootstrap();
  }, [initAppearance, initExplorer, initSubscription]);

  if (isAppearanceLoading) {
    return (
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100vh',
          width: '100vw',
          color: 'var(--text-secondary)',
          background: 'var(--bg-dark)',
          fontSize: '0.95rem',
        }}
      >
        Đang khởi động Rclone GUI...
      </div>
    );
  }

  return (
    <div className="app-layout">
      <Sidebar />
      <main className="main-content">
        <Routes>
          <Route path="/" element={<ExplorerPage />} />
          <Route path="/remotes" element={<RemotesPage />} />
          <Route path="/mounts" element={<MountsPage />} />
          <Route path="/transfers" element={<TransfersPage />} />
          <Route path="/trash" element={<TrashPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  );
};

export default App;
