/*
[INTEGRITY NOTES]
- Mục đích: Root component — layout, điều hướng và khởi tạo cài đặt/theme.
- Trách nhiệm: Nạp cài đặt + theme trước khi render nội dung; hiển thị sidebar.
- Tương tác: các store và trang trong `pages/`.
*/

import { Routes, Route, NavLink } from 'react-router-dom';
import { useEffect } from 'react';
import { Server, ShieldAlert, Settings as SettingsIcon, Layers, Cloud } from 'lucide-react';
import { ProvidersPage } from './pages/ProvidersPage';
import { BulkAddPage } from './pages/BulkAddPage';
import { CkeyPage } from './pages/CkeyPage';
import { CleanupPage } from './pages/CleanupPage';
import { SettingsPage } from './pages/SettingsPage';
import { useSettingsStore } from './store/useSettingsStore';
import { useThemeStore } from './store/useThemeStore';
import { useTranslation } from './utils/i18n';

function App() {
    const { initSettings, isLoading } = useSettingsStore();
    const { initThemes, isLoading: isThemeLoading } = useThemeStore();
    const { t } = useTranslation();

    useEffect(() => {
        const initAll = async () => {
            // initSettings tự chữa (chọn file ngôn ngữ có thật) và chỉ ném khi
            // backend sập hẳn — bắt ở đây để app vẫn lên thay vì treo màn tải.
            try { await initSettings(); } catch (e) { console.error('initSettings thất bại:', e); }
            await initThemes();
        };
        initAll();
    }, []);

    if (isLoading || isThemeLoading) {
        return (
            <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh', color: 'var(--text-primary)' }}>
                Loading…
            </div>
        );
    }

    return (
        <div className="app-layout">
            <nav className="sidebar">
                <h2 style={{ color: 'var(--primary)', marginBottom: '2rem', paddingLeft: '1rem' }}>
                    OpenCode
                </h2>

                <NavLink to="/" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`} end>
                    <Server size={20} /> {t('sidebar.providers')}
                </NavLink>
                <NavLink to="/cleanup" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <ShieldAlert size={20} /> {t('sidebar.cleanup')}
                </NavLink>
                <NavLink to="/bulk" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Layers size={20} /> {t('sidebar.bulk')}
                </NavLink>
                <NavLink to="/ckey" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <Cloud size={20} /> {t('sidebar.ckey')}
                </NavLink>
                <NavLink to="/settings" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <SettingsIcon size={20} /> {t('sidebar.settings')}
                </NavLink>
            </nav>

            <main className="main-content">
                <Routes>
                    <Route path="/" element={<ProvidersPage />} />
                    <Route path="/cleanup" element={<CleanupPage />} />
                    <Route path="/bulk" element={<BulkAddPage />} />
                    <Route path="/ckey" element={<CkeyPage />} />
                    <Route path="/settings" element={<SettingsPage />} />
                </Routes>
            </main>
        </div>
    );
}

export default App;
