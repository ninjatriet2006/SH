/*
[INTEGRITY NOTES]
Mục đích: Điểm vào chính (entry point) cho logic giao diện rcloneGUI.
Trách nhiệm: Khởi tạo UI, load ngôn ngữ (i18n), thiết lập sự kiện các tab, sidebar.
Các module tương tác: /bridge/remote_api.ts, /bridge/lang_api.ts
*/

import { RemotesManager } from './features/remotesManager.ts';
import { MountManager } from './features/mountManager';
import { DualPaneExplorer } from './components/DualPaneExplorer.ts';
import { MenuBar } from './components/MenuBar';
import { TransferDrawer } from './components/TransferDrawer';
import { DebugView } from './components/DebugView.ts';
import { RecentsView } from './components/RecentsView';
import { Sidebar } from './components/Sidebar';

// Nhúng CSS thông qua Vite bundler
import '../../themes/tokens.css';
import '../../themes/style.css';

// Từ điển được đọc lúc CHẠY từ thư mục `langs/` cạnh binary (qua backend), thay
// vì `import viLang from '../../langs/vi.json'` như trước. Cách cũ nhúng cứng
// một file vào bundle nên: thêm/sửa bản dịch phải build lại, `langs/` trong
// release chỉ là file chết, và app không chạy được với bộ ngôn ngữ không có `vi`.
import { resolveLanguage, applyLanguage, observeLanguage } from './features/i18n';
import { appState, normalizeSettings, saveSettings } from './store';
import { getAvailableFonts, getAvailableThemes, type FontInfo, type ThemeInfo } from '../../bridge/appearance_api';
import { getAvailableLangs } from '../../bridge/lang_api';
import { applyFont, applyTheme } from './features/appearance';

let remotesManager: RemotesManager | null = null;
let mountManager: MountManager | null = null;
let debugView: DebugView | null = null;
let recentsView: RecentsView | null = null;
let sidebar: Sidebar | null = null;

// Từ điển hiện tại; giữ ở module scope để `update_language_ui()` dùng lại khi
// các view được tạo động sau lúc khởi tạo.
let currentLangData: Record<string, string> = {};

/**
 * Cập nhật UI text dựa trên `data-lang-id` (quy tắc ID Linking).
 */
function update_language_ui(root: HTMLElement = document.body) {
  applyLanguage(currentLangData, root);
}

/**
 * Hàm load ngôn ngữ. `preferred` để rỗng thì dùng file đầu tiên có thật.
 */
async function loadLanguage(preferred: string | null = null) {
  const { code, data } = await resolveLanguage(preferred);
  currentLangData = data;
  if (code) document.documentElement.lang = code;
  update_language_ui();
}

async function initSettings() {
  const [languages, themes, fonts] = await Promise.all([
    getAvailableLangs(),
    getAvailableThemes(),
    getAvailableFonts(),
  ]);
  const settings = appState.settings!;
  const normalized = normalizeSettings(
    settings,
    languages,
    themes.map((item) => item.id),
    fonts.map((item) => item.id),
  );
  const theme = themes.find((item) => item.id === settings.theme) ?? themes[0];
  const font = fonts.find((item) => item.id === settings.font) ?? fonts[0];
  if (theme) applyTheme(theme);
  if (font) applyFont(font);
  if (normalized) saveSettings();

  const fill = (id: string, values: Array<{ id: string; name: string }>, selected: string) => {
    const select = document.getElementById(id) as HTMLSelectElement | null;
    if (!select) return;
    select.replaceChildren(...values.map(({ id, name }) => new Option(name, id)));
    select.value = values.some((item) => item.id === selected) ? selected : values[0]?.id ?? '';
  };
  fill('settings-language', languages.map((id) => ({ id, name: id })), settings.language);
  fill('settings-theme', themes, theme?.id ?? '');
  fill('settings-font', fonts, font?.id ?? '');

  const bind = <T extends ThemeInfo | FontInfo>(id: string, values: T[], apply: (value: T) => void, key: 'theme' | 'font') => {
    document.getElementById(id)?.addEventListener('change', (event) => {
      const value = values.find((item) => item.id === (event.target as HTMLSelectElement).value);
      if (!value) return;
      settings[key] = value.id;
      apply(value);
      saveSettings();
    });
  };
  bind('settings-theme', themes, applyTheme, 'theme');
  bind('settings-font', fonts, applyFont, 'font');
  document.getElementById('settings-language')?.addEventListener('change', async (event) => {
    settings.language = (event.target as HTMLSelectElement).value;
    saveSettings();
    await loadLanguage(settings.language);
  });
}

/**
 * Thiết lập các sự kiện giao diện (UI Events).
 */
function setupEvents() {
  // Tabs Navigation
  const tabs = document.querySelectorAll('.nav-tab');
  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      // 1. Cập nhật class active cho tab
      // 2. Lấy tên view cần chuyển
      const viewName = tab.getAttribute('data-view') || 'explorer';
      const targetView = document.getElementById(`view-${viewName}`);
      if (!targetView) {
        console.warn(`View chưa được triển khai: ${viewName}`);
        return;
      }
      document.querySelector('.nav-tab.active')?.classList.remove('active');
      tab.classList.add('active');
      console.log('Chuyển sang view:', viewName);
      
      // 3. Ẩn tất cả các view
      const views = document.querySelectorAll('.app-view');
      views.forEach(v => {
        v.classList.remove('active');
        (v as HTMLElement).style.display = 'none';
      });
      
      // 4. Hiển thị view được chọn
      targetView.classList.add('active');
      targetView.style.display = viewName === 'explorer' ? 'grid' : 'flex';
      
      // 5. Logic riêng từng trang
      if (viewName === 'remotes') {
        if (!remotesManager) {
          remotesManager = new RemotesManager();
          remotesManager.init();
        }
        remotesManager.renderList();
      } else if (viewName === 'mounts') {
        if (!mountManager) {
          mountManager = new MountManager();
        }
      } else if (viewName === 'activity') {
        // Nhật ký được ghi liên tục bởi logActivity() nên phải render lại
        // mỗi lần mở tab để thấy bản ghi mới nhất.
        if (!recentsView) {
          recentsView = new RecentsView();
          targetView.appendChild(recentsView.getElement());
        } else {
          recentsView.render();
        }
      } else if (viewName === 'debug') {
        if (!debugView) {
          debugView = new DebugView();
        }
      }
    });
  });

  // Toggle Transfer Drawer
  const drawerToggle = document.getElementById('drawer-toggle');
  const drawer = document.getElementById('transfer-drawer');
  drawerToggle?.addEventListener('click', () => {
    drawer?.classList.toggle('open');
  });

  // Nút ☰: thu gọn sidebar thành dải icon (không ẩn hẳn) — giống sidebar dọc Firefox.
  const bodyRowEl = document.querySelector('.body-row') as HTMLElement | null;
  document.getElementById('sidebar-toggle')?.addEventListener('click', () => {
    const collapsed = !(sidebar?.isCollapsed() ?? false);
    sidebar?.setCollapsed(collapsed);
    bodyRowEl?.classList.toggle('sidebar-collapsed', collapsed);
    // Bỏ chiều rộng do kéo tay để lớp .sidebar-collapsed áp dụng được.
    if (bodyRowEl) bodyRowEl.style.gridTemplateColumns = '';
  });

  // Resize Sidebar
  const resizer = document.getElementById('sidebar-resizer');
  const bodyRow = bodyRowEl;
  
  let isResizing = false;
  resizer?.addEventListener('mousedown', () => {
    isResizing = true;
    document.body.style.cursor = 'col-resize';
  });
  
  document.addEventListener('mousemove', (e) => {
    if (!isResizing) return;
    if (sidebar?.isCollapsed()) return; // Đang thu gọn thì không kéo giãn
    const newWidth = Math.max(150, Math.min(e.clientX, 400));
    if (bodyRow) {
      bodyRow.style.gridTemplateColumns = `${newWidth}px 4px 1fr`;
    }
  });
  
  document.addEventListener('mouseup', () => {
    if (isResizing) {
      isResizing = false;
      document.body.style.cursor = 'default';
    }
  });
}

// Chạy khởi tạo khi load xong DOM
document.addEventListener('DOMContentLoaded', async () => {
  // Tạo Explorer trước khi gắn các listener tương tác của ứng dụng.
  const explorerContainer = document.getElementById('view-explorer');
  if (explorerContainer) {
    explorerContainer.innerHTML = '';
    const dualPane = new DualPaneExplorer();
    explorerContainer.appendChild(dualPane.container);

    // MenuBar và TreeView chỉ nhận đúng tập lệnh chúng cần, không dùng biến toàn cục.
    const menubarContainer = document.getElementById('menubar-container');
    if (menubarContainer) {
      menubarContainer.appendChild(new MenuBar(dualPane.commands).getElement());
    }

    // Sidebar: Truy cập nhanh (XDG) + Đã ghim + Cây thư mục.
    const sidebarEl = document.getElementById('sidebar');
    if (sidebarEl) {
      sidebar = new Sidebar({
        onSelect: (path) => dualPane.commands.navigateActive(path),
      });
      sidebarEl.appendChild(sidebar.getElement());
      await sidebar.init();
    }
  }

  // Ngôn ngữ đã lưu (rỗng = chưa chọn) → `resolveLanguage` rớt về file đầu
  // tiên thực có trong `langs/`. Không truyền mã cứng ở đây.
  await loadLanguage(appState.settings?.language || null);
  observeLanguage(() => currentLangData);
  await initSettings();
  setupEvents();
  new TransferDrawer();
  console.log('rcloneGUI khởi tạo thành công!');
});
