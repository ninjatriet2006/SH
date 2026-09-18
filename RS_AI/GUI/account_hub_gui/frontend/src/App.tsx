import React, { useEffect } from 'react';
import { Routes, Route, NavLink } from 'react-router-dom';
import { LayoutDashboard, CheckCircle2, Mail, Globe, Settings as SettingsIcon, Sliders, KeyRound } from 'lucide-react';
import { DashboardPage } from './pages/DashboardPage';
import { RegistrationsPage } from './pages/RegistrationsPage';
import { EmailsPage } from './pages/EmailsPage';
import { WebsitesPage } from './pages/WebsitesPage';
import { CriteriaPage } from './pages/CriteriaPage';
import { LoginMethodsPage } from './pages/LoginMethodsPage';
import { SettingsPage } from './pages/SettingsPage';
import { useAppStore } from './store';
import { useTranslation } from './i18n';

export const App: React.FC = () => {
  const { loadData, isLoading, error } = useAppStore();
  const { t } = useTranslation();

  useEffect(() => {
    loadData();
  }, [loadData]);

  if (isLoading) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh', color: '#fff' }}>
        Đang tải dữ liệu Account Hub...
      </div>
    );
  }

  return (
    <div className="app-container">
      {/* Sidebar Navigation */}
      <nav className="sidebar">
        <div className="sidebar-brand">
          <Globe size={24} color="#3b82f6" />
          <span>Account Hub</span>
        </div>

        <NavLink to="/" className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`} end>
          <LayoutDashboard size={18} /> {t('nav.dashboard')}
        </NavLink>

        <NavLink to="/registrations" className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}>
          <CheckCircle2 size={18} /> Đăng Ký (Live/Die)
        </NavLink>

        <NavLink to="/emails" className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}>
          <Mail size={18} /> {t('nav.emails')}
        </NavLink>

        <NavLink to="/websites" className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}>
          <Globe size={18} /> {t('nav.websites')}
        </NavLink>

        <NavLink to="/criteria" className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}>
          <Sliders size={18} /> Tiêu chí
        </NavLink>

        <NavLink to="/login-methods" className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}>
          <KeyRound size={18} /> Đăng nhập
        </NavLink>

        <div style={{ flex: 1 }} />

        <NavLink to="/settings" className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}>
          <SettingsIcon size={18} /> {t('nav.settings')}
        </NavLink>
      </nav>

      {/* Main Content Area */}
      <main className="main-content">
        {error && (
          <div style={{ background: 'rgba(239, 68, 68, 0.2)', border: '1px solid #ef4444', padding: '0.8rem', borderRadius: '8px', marginBottom: '1rem', color: '#ef4444' }}>
            Lỗi kết nối Backend: {error}
          </div>
        )}
        <Routes>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/registrations" element={<RegistrationsPage />} />
          <Route path="/emails" element={<EmailsPage />} />
          <Route path="/websites" element={<WebsitesPage />} />
          <Route path="/criteria" element={<CriteriaPage />} />
          <Route path="/login-methods" element={<LoginMethodsPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  );
};
