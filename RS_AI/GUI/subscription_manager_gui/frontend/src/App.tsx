/*
[INTEGRITY NOTES]
- Mục đích: Root Component quản lý Layout và Routing.
- Trách nhiệm: Hiển thị Sidebar, chuyển hướng URL tới các trang tương ứng.
- Tương tác: Dùng `react-router-dom` để tạo Routes.
*/

import { Routes, Route, NavLink } from 'react-router-dom';
import { LayoutDashboard, Users, PackageSearch, History } from 'lucide-react';
import { DashboardPage } from './pages/DashboardPage';
import { UserManagementPage } from './pages/UserManagementPage';
import { PackageManagementPage } from './pages/PackageManagementPage';
import { TransactionManagementPage } from './pages/TransactionManagementPage';
import { SettingsPage } from './pages/SettingsPage';
import { useSettingsStore } from './store/useSettingsStore';
import { useThemeStore } from './store/useThemeStore';
import { useFontStore } from './store/useFontStore';
import { useTranslation } from './utils/i18n';
import { useEffect } from 'react';
import { Settings as SettingsIcon } from 'lucide-react';

function App() {
  const { initSettings, isLoading } = useSettingsStore();
  const { initThemes, isLoading: isThemeLoading } = useThemeStore();
  const { initFonts, isLoading: isFontLoading } = useFontStore();
  const { t } = useTranslation();

  useEffect(() => {
    const initAll = async () => {
      await initSettings();
      await initThemes();
      await initFonts();
    };
    initAll();
  }, []);

  if (isLoading || isThemeLoading || isFontLoading) {
    return <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh', color: 'var(--text-primary)' }}>Loading App...</div>;
  }

  return (
    <div className="app-layout">
      {/* Cột bên trái: Menu điều hướng */}
      <nav className="sidebar">
        <h2 style={{ color: 'var(--primary)', marginBottom: '2rem', paddingLeft: '1rem' }}>
          SubManager
        </h2>
        
        <NavLink 
          to="/" 
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
          end
        >
          <LayoutDashboard size={20} /> {t('sidebar.dashboard')}
        </NavLink>
        
        <NavLink 
          to="/users" 
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
        >
          <Users size={20} /> {t('sidebar.users')}
        </NavLink>
        
        <NavLink 
          to="/packages" 
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
        >
          <PackageSearch size={20} /> {t('sidebar.packages')}
        </NavLink>

        <NavLink 
          to="/transactions" 
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
        >
          <History size={20} /> {t('sidebar.transactions')}
        </NavLink>

        <NavLink 
          to="/settings" 
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
        >
          <SettingsIcon size={20} /> {t('sidebar.settings')}
        </NavLink>
      </nav>

      {/* Cột bên phải: Nội dung trang chính */}
      <main className="main-content">
        <Routes>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/users" element={<UserManagementPage />} />
          <Route path="/packages" element={<PackageManagementPage />} />
          <Route path="/transactions" element={<TransactionManagementPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  );
}

export default App;
