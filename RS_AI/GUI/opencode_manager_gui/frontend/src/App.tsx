/*
[INTEGRITY NOTES]
 - Mục đích: Root component — layout, điều hướng và khởi tạo cài đặt/theme/font.
- Trách nhiệm: Nạp cài đặt + theme trước khi render nội dung; hiển thị sidebar
   (có handle kéo đổi độ rộng, lưu localStorage, nhấn đúp để đặt lại).
- Tương tác: các store và trang trong `pages/`, `utils/dragResize.ts`.
*/

import { Routes, Route, NavLink } from 'react-router-dom';
import { useCallback, useEffect, useState } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { Server, ShieldAlert, Settings as SettingsIcon, Layers, Cloud, BarChart3 } from 'lucide-react';
import { ProvidersPage } from './pages/ProvidersPage';
import { BulkAddPage } from './pages/BulkAddPage';
import { CkeyPage } from './pages/CkeyPage';
import { ModelsPage } from './pages/ModelsPage';
import { CleanupPage } from './pages/CleanupPage';
import { SettingsPage } from './pages/SettingsPage';
import { useSettingsStore } from './store/useSettingsStore';
import { useThemeStore } from './store/useThemeStore';
import { useFontStore } from './store/useFontStore';
import { useTranslation } from './utils/i18n';
import { startHorizontalDrag } from './utils/dragResize';

const SIDEBAR_STORAGE_KEY = 'opencode-manager:sidebar-width';
const SIDEBAR_MIN = 180;
const SIDEBAR_MAX = 420;
const SIDEBAR_DEFAULT = 240;

/** Độ rộng lưu phải nằm trong biên cho phép; giá trị rác/biến mất → về mặc định. */
function loadSidebarWidth(): number {
    const v = Number(localStorage.getItem(SIDEBAR_STORAGE_KEY));
    return Number.isFinite(v) && v >= SIDEBAR_MIN && v <= SIDEBAR_MAX ? v : SIDEBAR_DEFAULT;
}

function App() {
    const { initSettings, isLoading } = useSettingsStore();
    const { initThemes, isLoading: isThemeLoading } = useThemeStore();
    const { initFonts, isLoading: isFontLoading } = useFontStore();
    const { t } = useTranslation();
    const [sidebarWidth, setSidebarWidth] = useState<number>(loadSidebarWidth);

    useEffect(() => {
        const initAll = async () => {
            // initSettings tự chữa (chọn file ngôn ngữ có thật) và chỉ ném khi
            // backend sập hẳn — bắt ở đây để app vẫn lên thay vì treo màn tải.
            try { await initSettings(); } catch (e) { console.error('initSettings thất bại:', e); }
            await initThemes();
            await initFonts();
        };
        initAll();
    }, []);

    const startSidebarResize = useCallback((e: ReactMouseEvent) => {
        e.preventDefault();
        const startX = e.clientX;
        const startWidth = sidebarWidth;
        let latest = startWidth;
        startHorizontalDrag(
            clientX => {
                latest = Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, Math.round(startWidth + clientX - startX)));
                setSidebarWidth(latest);
            },
            // Chỉ ghi localStorage khi thả tay — kéo không gây ghi liên tục.
            () => {
                try { localStorage.setItem(SIDEBAR_STORAGE_KEY, String(latest)); } catch { /* bỏ qua */ }
            },
        );
    }, [sidebarWidth]);

    const resetSidebarWidth = useCallback(() => {
        setSidebarWidth(SIDEBAR_DEFAULT);
        try { localStorage.setItem(SIDEBAR_STORAGE_KEY, String(SIDEBAR_DEFAULT)); } catch { /* bỏ qua */ }
    }, []);

    if (isLoading || isThemeLoading || isFontLoading) {
        return (
            <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh', color: 'var(--text-primary)' }}>
                Loading…
            </div>
        );
    }

    return (
        <div className="app-layout" style={{ gridTemplateColumns: `${sidebarWidth}px 1fr` }}>
            <nav className="sidebar">
                <div
                    className="sidebar-resizer"
                    onMouseDown={startSidebarResize}
                    onDoubleClick={resetSidebarWidth}
                    title={t('sidebar.resize_hint')}
                />
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
                <NavLink to="/models" className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}>
                    <BarChart3 size={20} /> {t('sidebar.models')}
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
                    <Route path="/models" element={<ModelsPage />} />
                    <Route path="/settings" element={<SettingsPage />} />
                </Routes>
            </main>
        </div>
    );
}

export default App;
