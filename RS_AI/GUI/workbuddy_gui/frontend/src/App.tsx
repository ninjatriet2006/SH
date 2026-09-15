import { Routes, Route, NavLink } from 'react-router-dom';
import { useEffect, useState } from 'react';
import { Users, Settings as SettingsIcon, Clock, Shield, Activity, LayoutDashboard } from 'lucide-react';
import { DashboardPage } from './pages/DashboardPage';
import { AccountsPage } from './pages/AccountsPage';
import { AdminPage } from './pages/AdminPage';
import { ConfigPage } from './pages/ConfigPage';
import { SchedulerPage } from './pages/SchedulerPage';
import { SettingsPage } from './pages/SettingsPage';
import { useSettingsStore } from './store/useSettingsStore';
import { useThemeStore } from './store/useThemeStore';
import { useFontStore } from './store/useFontStore';
import { useTranslation } from './utils/i18n';

const SIDEBAR_STORAGE_KEY = 'workbuddy-gui:sidebar-width';
const SIDEBAR_MIN = 180;
const SIDEBAR_MAX = 420;
const SIDEBAR_DEFAULT = 240;

function loadSidebarWidth(): number {
    const v = Number(localStorage.getItem(SIDEBAR_STORAGE_KEY));
    return Number.isFinite(v) && v >= SIDEBAR_MIN && v <= SIDEBAR_MAX ? v : SIDEBAR_DEFAULT;
}

function App() {
    const { initSettings, isLoading } = useSettingsStore();
    const { initThemes, isLoading: isThemeLoading } = useThemeStore();
    const { initFonts, isLoading: isFontLoading } = useFontStore();
    const { t } = useTranslation();
    const [sidebarWidth] = useState<number>(loadSidebarWidth);

    useEffect(() => {
        const initAll = async () => {
            try { await initSettings(); } catch (e) { console.error('initSettings failed:', e); }
            await initThemes();
            await initFonts();
        };
        initAll();
    }, []);

    if (isLoading || isThemeLoading || isFontLoading) {
        return (
            <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh', color: 'var(--text-primary)' }}>
                Loading...
            </div>
        );
    }

    return (
        <div className="app-layout" style={{ gridTemplateColumns: `${sidebarWidth}px 1fr` }}>
            <nav className="sidebar">
                <h2 style={{ color: 'var(--primary)', marginBottom: '2rem', paddingLeft: '1rem' }}>
                    WorkBuddy
                </h2>
                <NavLink to="/" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`} end>
                    <Activity size={20} /> {t('sidebar.dashboard')}
                </NavLink>
                <NavLink to="/accounts" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Users size={20} /> {t('sidebar.accounts')}
                </NavLink>
                <NavLink to="/admin" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <LayoutDashboard size={20} /> {t('sidebar.admin')}
                </NavLink>
                <NavLink to="/scheduler" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Clock size={20} /> {t('sidebar.scheduler')}
                </NavLink>
                <NavLink to="/config" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Shield size={20} /> {t('sidebar.config')}
                </NavLink>
                <NavLink to="/settings" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <SettingsIcon size={20} /> {t('sidebar.settings')}
                </NavLink>
            </nav>
            <main className="main-content">
                <Routes>
                    <Route path="/" element={<DashboardPage />} />
                    <Route path="/accounts" element={<AccountsPage />} />
                    <Route path="/admin" element={<AdminPage />} />
                    <Route path="/scheduler" element={<SchedulerPage />} />
                    <Route path="/config" element={<ConfigPage />} />
                    <Route path="/settings" element={<SettingsPage />} />
                </Routes>
            </main>
        </div>
    );
}

export default App;
