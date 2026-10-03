import { Routes, Route, NavLink } from 'react-router-dom';
import { useEffect, useState } from 'react';
import {
    Activity,
    Cpu,
    Radio,
    Settings as SettingsIcon,
    Shield,
    Rocket,
    LayoutDashboard,
    Sliders,
} from 'lucide-react';
import { DashboardPage } from './pages/DashboardPage';
import { InstancesPage } from './pages/InstancesPage';
import { CodebuddyCnPage } from './pages/platforms/CodebuddyCnPage';
import { CodebuddyGlobalPage } from './pages/platforms/CodebuddyGlobalPage';
import { AccountsPage } from './pages/AccountsPage';
import { ApiRelayPage } from './pages/ApiRelayPage';
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
import { useProfileStore } from './store/useProfileStore';
import { useTranslation } from './utils/i18n';

// Cockpit Platform Native Icons
import codebuddyIcon from './assets/icons/codebuddy.png';
import zedIcon from './assets/icons/zed.png';

const SIDEBAR_STORAGE_KEY = 'universal-api:sidebar-width';
const SIDEBAR_MIN = 200;
const SIDEBAR_MAX = 360;
const SIDEBAR_DEFAULT = 250;

function loadSidebarWidth(): number {
    const v = Number(localStorage.getItem(SIDEBAR_STORAGE_KEY));
    return Number.isFinite(v) && v >= SIDEBAR_MIN && v <= SIDEBAR_MAX ? v : SIDEBAR_DEFAULT;
}

function App() {
    const { initSettings, isLoading } = useSettingsStore();
    const { initThemes, isLoading: isThemeLoading } = useThemeStore();
    const { initFonts, isLoading: isFontLoading } = useFontStore();
    const { runningInstances, fetchProfiles } = useProfileStore();
    const { t } = useTranslation();
    const [sidebarWidth] = useState<number>(loadSidebarWidth);

    useEffect(() => {
        const initAll = async () => {
            try { await initSettings(); } catch (e) { console.error('initSettings failed:', e); }
            await initThemes();
            await initFonts();
            await fetchProfiles();
        };
        initAll();
    }, []);

    if (isLoading || isThemeLoading || isFontLoading) {
        return (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', justifyContent: 'center', alignItems: 'center', height: '100vh', color: 'var(--text-primary)' }}>
                <Rocket className="spin" size={32} color="var(--primary)" />
                <div style={{ fontSize: '0.9rem', color: 'var(--text-secondary)' }}>Đang khởi tạo Universe Cockpit...</div>
            </div>
        );
    }

    const runningCount = runningInstances.length;

    return (
        <div className="app-layout" style={{ gridTemplateColumns: `${sidebarWidth}px 1fr` }}>
            <nav className="sidebar">
                {/* Brand Header */}
                <div className="sidebar-header">
                    <div className="sidebar-brand-icon">
                        <Rocket size={20} />
                    </div>
                    <div className="sidebar-brand-text">
                        <h2>Cockpit Tools</h2>
                        <span>AI IDE Account & Profile Manager</span>
                    </div>
                </div>

                {/* Nav Links */}
                <div className="sidebar-scroll">
                    <div className="nav-group">Tổng quan & Điều khiển</div>
                    <NavLink to="/" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`} end>
                        <LayoutDashboard size={18} /> {t('sidebar.dashboard') || 'Tổng quan'}
                    </NavLink>
                    <NavLink to="/instances" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                        <Cpu size={18} /> Môi trường & Profiles
                        {runningCount > 0 && (
                            <span className="badge badge-success" style={{ marginLeft: 'auto', fontSize: '0.65rem', padding: '0.15rem 0.4rem' }}>
                                {runningCount}
                            </span>
                        )}
                    </NavLink>

                    <div className="nav-group">Nền tảng Quản lý</div>
                    <NavLink to="/platforms/codebuddy-cn" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                        <img src={codebuddyIcon} alt="" className="nav-item-icon" /> CodeBuddy CN
                    </NavLink>
                    <NavLink to="/platforms/codebuddy-global" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                        <img src={codebuddyIcon} alt="" className="nav-item-icon" /> CodeBuddy Global
                    </NavLink>
                    <NavLink to="/zed/accounts" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                        <img src={zedIcon} alt="" className="nav-item-icon" /> Zed Cloud
                    </NavLink>
                    <NavLink to="/accounts" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                        <Shield size={18} /> Kho Vault Accounts
                    </NavLink>

                    <div className="nav-group">Hệ thống & Tiện ích</div>
                    <NavLink to="/api-relay" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                        <Radio size={18} /> OpenAI Relay Gateway
                    </NavLink>
                    <NavLink to="/traffic" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                        <Activity size={18} /> {t('sidebar.traffic') || 'Traffic & Logs'}
                    </NavLink>
                    <NavLink to="/config" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                        <Sliders size={18} /> Cấu hình Hệ thống
                    </NavLink>
                    <NavLink to="/settings" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                        <SettingsIcon size={18} /> {t('sidebar.settings') || 'Cài đặt'}
                    </NavLink>
                </div>

                {/* Footer status */}
                <div className="sidebar-footer">
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.72rem', color: 'var(--text-secondary)' }}>
                        <span className={`pulse-dot ${runningCount > 0 ? '' : 'paused'}`} style={{
                            background: runningCount > 0 ? 'var(--success)' : 'var(--text-muted)',
                            boxShadow: runningCount > 0 ? '0 0 8px var(--success)' : 'none'
                        }} />
                        <span>{runningCount > 0 ? `${runningCount} Instance active` : 'Chưa có Instance'}</span>
                    </div>
                    <span style={{ fontSize: '0.65rem', color: 'var(--text-muted)' }}>v1.3.65</span>
                </div>
            </nav>

            <main className="main-content">
                <Routes>
                    <Route path="/" element={<DashboardPage />} />
                    <Route path="/instances" element={<InstancesPage />} />
                    <Route path="/platforms/codebuddy-cn" element={<CodebuddyCnPage />} />
                    <Route path="/platforms/codebuddy-global" element={<CodebuddyGlobalPage />} />
                    <Route path="/accounts" element={<AccountsPage />} />
                    <Route path="/api-relay" element={<ApiRelayPage />} />
                    <Route path="/zed/accounts" element={<ZedAccountsPage />} />
                    <Route path="/zed/config" element={<ZedConfigPage />} />
                    <Route path="/traffic" element={<TrafficLogsPage />} />
                    <Route path="/providers" element={<ProvidersPage />} />
                    <Route path="/scheduler" element={<SchedulerPage />} />
                    <Route path="/config" element={<ConfigPage />} />
                    <Route path="/settings" element={<SettingsPage />} />
                </Routes>
            </main>
        </div>
    );
}

export default App;
