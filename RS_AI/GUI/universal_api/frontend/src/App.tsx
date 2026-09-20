import { Routes, Route, NavLink } from 'react-router-dom';
import { useEffect, useState } from 'react';
import { Users, Settings as SettingsIcon, Clock, Shield, Activity, Radio, Plug } from 'lucide-react';
import { DashboardPage } from './pages/DashboardPage';
import { AccountsPage } from './pages/AccountsPage';
import { ConfigPage } from './pages/ConfigPage';
import { ProvidersPage } from './pages/ProvidersPage';
import { ZedAccountsPage } from './pages/ZedAccountsPage';
import { ZedConfigPage } from './pages/ZedConfigPage';
import { SchedulerPage } from './pages/SchedulerPage';
import { SettingsPage } from './pages/SettingsPage';
import { TrafficLogsPage } from './pages/TrafficLogsPage';
import { useSettingsStore } from './store/useSettingsStore';
import { useThemeStore } from './store/useThemeStore';
import { useFontStore } from './store/useFontStore';
import { useTranslation } from './utils/i18n';

const SIDEBAR_STORAGE_KEY = 'universal-api:sidebar-width';
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
                    Universal API
                </h2>
                <div className="nav-group">{t('sidebar.group_common')}</div>
                <NavLink to="/" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`} end>
                    <Activity size={20} /> {t('sidebar.dashboard')}
                </NavLink>
                <NavLink to="/traffic" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Radio size={20} /> {t('sidebar.traffic')}
                </NavLink>
                <NavLink to="/settings" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <SettingsIcon size={20} /> {t('sidebar.settings')}
                </NavLink>
                <div className="nav-group">{t('sidebar.group_codebuddy')}</div>
                <NavLink to="/accounts" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Users size={20} /> {t('sidebar.accounts')}
                </NavLink>
                <NavLink to="/scheduler" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Clock size={20} /> {t('sidebar.scheduler')}
                </NavLink>
                <NavLink to="/config" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Shield size={20} /> {t('sidebar.config')}
                </NavLink>
                <div className="nav-group">{t('sidebar.group_antipi')}</div>
                <NavLink to="/providers" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Plug size={20} /> {t('sidebar.providers')}
                </NavLink>
                <div className="nav-group">{t('sidebar.group_zed')}</div>
                <NavLink to="/zed/accounts" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Users size={20} /> {t('providers.account_section')}
                </NavLink>
                <NavLink to="/zed/config" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Shield size={20} /> {t('providers.config_section')}
                </NavLink>
            </nav>
            <main className="main-content">
                <Routes>
                    <Route path="/" element={<DashboardPage />} />
                    <Route path="/accounts" element={<AccountsPage />} />
                    <Route path="/traffic" element={<TrafficLogsPage />} />
                    <Route path="/providers" element={<ProvidersPage />} />
                    <Route path="/zed/accounts" element={<ZedAccountsPage />} />
                    <Route path="/zed/config" element={<ZedConfigPage />} />
                    <Route path="/scheduler" element={<SchedulerPage />} />
                    <Route path="/config" element={<ConfigPage />} />
                    <Route path="/settings" element={<SettingsPage />} />
                </Routes>
            </main>
        </div>
    );
}

export default App;
