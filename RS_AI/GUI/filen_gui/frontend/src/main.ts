// filen_gui frontend — Phase 1 scaffold.
// Placeholder bootstrap: wires up nav tabs, sidebar, and the transfer drawer.
// Real state management (store.ts) and Tauri command bindings land in later phases.
import { ThemeManager } from "./themes/ThemeManager";
import { t, setLanguage, applyLanguage } from "./i18n";
import { transferManager } from "./features/transferManager";
import { TransferDrawer } from "./components/TransferDrawer";

// ── Runtime i18n (docs/i18n-and-themes.md §2) ─────────────────────────────
// Phase 1: dùng `lang` từ <html> (vi), hot-switch `setLanguage()` + `applyLanguage()`
// qua `[data-lang-id]`. Phase 2+: lưu `current_language` trong settings.json.
function initI18n(): void {
  const htmlLang = document.documentElement.lang || "en";
  setLanguage(htmlLang);
  applyLanguage();
  console.log(`[i18n] lang → ${htmlLang}`);
}

declare global {
  interface Window {
    /** Tra cứu chuỗi UI — expose toàn cục cho các module/QA khác. */
    t: typeof t;
  }
}

// Expose `t()` global (đúng spec: mọi text hiển thị qua `t(key)`).
window.t = t;

// ── Runtime theming (docs/themes-runtime.md §2) ────────────────────────────
// Phase 1: apply default Neon theme. Phase 2+: đọc `active_theme` từ
// settings.json + hot-reload qua Rust watcher (`themes:changed` event).
function initTheming(): void {
  const tm = new ThemeManager();
  tm.onHotReload((entry) => {
    console.log(`[themes] hot-reload → ${entry?.slug ?? "default"}`);
  });
  tm.loadAll()
    .then((entries) => {
      console.log(`[themes] loaded ${entries.length} theme(s)`);
      tm.apply(null); // default Neon theme
    })
    .catch((err) => {
      console.warn("[themes] loadAll fail, dùng default", err);
      tm.apply(null);
    });
}

let currentView = "explorer";

function setActiveView(view: string, place?: string): void {
  document.querySelectorAll<HTMLButtonElement>(".nav-tab[data-view]").forEach((t) => {
    t.classList.toggle("active", t.dataset.view === view);
  });
  
  document.querySelectorAll<HTMLButtonElement>(".side-item").forEach((i) => {
    i.classList.remove("active");
  });
  
  if (place) {
    const placeBtn = document.querySelector<HTMLButtonElement>(`.side-item[data-place="${place}"]`);
    if (placeBtn) placeBtn.classList.add("active");
  } else {
    const viewBtn = document.querySelector<HTMLButtonElement>(`.side-item[data-view="${view}"]`);
    if (viewBtn) viewBtn.classList.add("active");
  }
}

async function switchView(view: string, place?: string): Promise<void> {
  const central = document.getElementById('central');
  if (!central) return;
  const prev = currentView;
  currentView = view;
  setActiveView(view, place);
  central.innerHTML = '';
  if (view === 'explorer') {
    let explorer = (window as any).__explorer;
    if (!explorer) {
      const { DualPaneExplorer } = await import('./components/DualPaneExplorer');
      explorer = new DualPaneExplorer();
      (window as any).__explorer = explorer;
    }
    central.appendChild(explorer.getElement());
    return;
  }
  let el: HTMLDivElement;
  if (view === 'recents') {
    const { RecentsView } = await import('./components/RecentsView');
    el = new RecentsView().getElement();
  } else if (view === 'sync') {
    const { SyncPairsView } = await import('./components/SyncPairsView');
    el = new SyncPairsView().getElement();
  } else if (view === 'servers') {
    const { ServersDashboard } = await import('./components/ServersDashboard');
    el = new ServersDashboard().getElement();
  } else {
    return;
  }
  central.appendChild(el);
  console.log(`[view] ${prev} → ${view}`);
}

function bindNavTabs(): void {
  const tabs = document.querySelectorAll<HTMLButtonElement>(".nav-tab[data-view]");
  tabs.forEach((tab) => {
    tab.addEventListener("click", () => {
      switchView(tab.dataset.view!);
    });
  });
}

function bindSidebar(): void {
  const sidebarToggle = document.getElementById('sidebar-toggle');
  const sidebar = document.getElementById('sidebar');
  if (sidebarToggle && sidebar) {
    sidebarToggle.addEventListener('click', () => {
      sidebar.classList.toggle('collapsed');
    });
  }
  const items = document.querySelectorAll<HTMLButtonElement>(".side-item[data-view]");
  items.forEach((item) => {
    item.addEventListener("click", () => {
      switchView(item.dataset.view!);
    });
  });

  const places = document.querySelectorAll<HTMLButtonElement>(".side-item[data-place]");
  places.forEach((item) => {
    item.addEventListener("click", async () => {
      const place = item.dataset.place;
      await switchView('explorer', place);
      let targetPath = '/';
      try {
        const { homeDir, desktopDir, documentDir, downloadDir } = await import('@tauri-apps/api/path');
        switch (place) {
          case 'home': targetPath = await homeDir(); break;
          case 'desktop': targetPath = await desktopDir(); break;
          case 'documents': targetPath = await documentDir(); break;
          case 'downloads': targetPath = await downloadDir(); break;
          case 'trash://local': targetPath = 'trash://local'; break;
          case 'trash://remote': targetPath = 'trash://remote'; break;
        }
      } catch (e) {
        console.warn("Failed to resolve path for", place, e);
      }
      const explorer = (window as any).__explorer;
      if (explorer) {
        if (place === 'trash://remote') {
          explorer.loadPane('right', targetPath);
        } else {
          explorer.loadPane('left', targetPath);
        }
      }
    });
  });

  renderBookmarks();
  window.addEventListener('bookmarks-updated', renderBookmarks);

  document.getElementById('btn-login')?.addEventListener('click', () => {
    import('./components/AuthModal').then(({ AuthModal }) => {
      new AuthModal().open();
    });
  });

  document.getElementById('btn-settings')?.addEventListener('click', () => {
    import('./components/SettingsModal').then(({ SettingsModal }) => {
      new SettingsModal().open();
    });
  });

  document.getElementById('btn-edit-bookmarks')?.addEventListener('click', () => {
    import('./components/BookmarkManagerModal').then(({ BookmarkManagerModal }) => {
      new BookmarkManagerModal().open();
    });
  });

  window.addEventListener('filen-bookmarks-changed', () => {
    renderBookmarks();
  });

  document.getElementById("btn-logout")?.addEventListener("click", async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      await invoke('auth_logout_terminal', {});
    } catch (_) {}
    // clear state
    const { appState } = await import('./store');
    appState.auth = undefined;
    // update UI
    const logoutBtn = document.getElementById('btn-logout') as HTMLButtonElement;
    logoutBtn.disabled = true;
    // re‑enable login button after logout
    const loginBtn = document.getElementById('btn-login') as HTMLButtonElement;
    if (loginBtn) {
      loginBtn.disabled = false;
      loginBtn.classList.remove('disabled');
    }
    logoutBtn.classList.add('disabled');
    const accountPill = document.getElementById('account-pill');
    if (accountPill) accountPill.textContent = '👤 Chưa đăng nhập';
    // reload cloud pane (now logged out → placeholder)
    const explorer = (window as any).__explorer;
    if (explorer) explorer.loadPane('right', '/');
  });
}

async function renderTreeView() {
  const treeContainer = document.getElementById('sidebar-tree-list');
  if (treeContainer) {
    const { homeDir } = await import('@tauri-apps/api/path');
    const rootPath = await homeDir().catch(() => '/home');
    const { TreeView } = await import('./components/TreeView');
    const tree = new TreeView(rootPath);
    treeContainer.appendChild(tree.getElement());
  }
}

async function renderBookmarks() {
  const listEl = document.getElementById('sidebar-bookmarks-list');
  if (!listEl) return;
  listEl.innerHTML = '';
  
  const { appState, toggleBookmark } = await import('./store');
  const bookmarks = appState.bookmarks || [];
  
  bookmarks.forEach(b => {
    const btn = document.createElement('button');
    btn.className = 'side-item bookmark-item';
    
    const textSpan = document.createElement('span');
    textSpan.className = 'bookmark-text';
    textSpan.textContent = `📁 ${b.name}`;
    
    const delBtn = document.createElement('span');
    delBtn.className = 'bookmark-del';
    delBtn.textContent = '❌';
    delBtn.title = 'Bỏ ghim';
    
    btn.appendChild(textSpan);
    btn.appendChild(delBtn);
    
    textSpan.addEventListener('click', async (e) => {
      e.stopPropagation();
      await switchView('explorer');
      const explorer = (window as any).__explorer;
      if (explorer) explorer.loadPane('left', b.path);
    });
    
    delBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      toggleBookmark(b.name, b.path);
    });
    
    listEl.appendChild(btn);
  });
}

// ── Mount explorer ngay khi khởi động (local pane KHÔNG cần đăng nhập) ─────
async function mountExplorer(): Promise<void> {
  const central = document.getElementById('central');
  if (!central) return;
  // tránh mount 2 lần
  if (central.querySelector('.dual-pane-explorer')) return;
  let explorer = (window as any).__explorer;
  if (!explorer) {
    const { DualPaneExplorer } = await import('./components/DualPaneExplorer');
    explorer = new DualPaneExplorer();
    (window as any).__explorer = explorer;
  }
  central.innerHTML = '';
  central.appendChild(explorer.getElement());
}

// ── Khôi phục session: filen đã đăng nhập sẵn thì bật UI tương ứng ─────────
async function restoreSession(): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  const { appState } = await import('./store');
  try {
    const email = await invoke<string | null>('auth_whoami_terminal', {});
    if (email) {
      appState.auth = { user: email };
      const loginBtn = document.getElementById('btn-login') as HTMLButtonElement;
      if (loginBtn) { loginBtn.disabled = true; loginBtn.classList.add('disabled'); }
      const logoutBtn = document.getElementById('btn-logout') as HTMLButtonElement;
      if (logoutBtn) { logoutBtn.disabled = false; logoutBtn.classList.remove('disabled'); }
      const accountPill = document.getElementById('account-pill');
      if (accountPill) accountPill.textContent = `👤 ${email}`;
      console.log(`[session] restored → ${email}`);
      
      const explorer = (window as any).__explorer;
      if (explorer) {
        explorer.loadPane('right', appState.explorer?.rightPath ?? '/');
      }
    } else {
      console.log('[session] chưa đăng nhập — cloud pane sẽ hiện placeholder');
    }
  } catch (e) {
    console.warn('[session] auth_whoami fail', e);
  }
}

function bindDrawer(): void {
  const toggle = document.getElementById("drawer-toggle");
  const body = document.getElementById("drawer-body");
  const drawer = document.getElementById("transfer-drawer");
  if (!toggle || !body || !drawer) return;

  // Note: display state is managed by TransferDrawer now
  const label = document.getElementById("drawer-label");
  if (label) {
    label.textContent = t("drawer_open");
  }

  // Make toggle act as a vertical resizer handle
  toggle.style.cursor = 'row-resize';

  let isResizing = false;
  toggle.addEventListener('mousedown', (e) => {
    isResizing = true;
    document.body.style.cursor = 'row-resize';
    document.body.style.userSelect = 'none';
    e.preventDefault();
  });

  document.addEventListener('mousemove', (e) => {
    if (!isResizing) return;
    const newHeight = window.innerHeight - e.clientY;
    const toggleHeight = toggle.getBoundingClientRect().height;
    const minHeight = toggleHeight + 20; // Some content visible
    const maxHeight = window.innerHeight / 2;
    if (newHeight > minHeight && newHeight < maxHeight) {
      body.style.maxHeight = 'none';
      body.style.height = `${newHeight - toggleHeight}px`;
    }
  });

  document.addEventListener('mouseup', () => {
    if (isResizing) {
      isResizing = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }
  });
}

function bindSidebarResizer(): void {
  const resizer = document.getElementById('sidebar-resizer');
  const bodyRow = document.querySelector('.body-row') as HTMLElement;
  if (!resizer || !bodyRow) return;

  let isResizing = false;
  resizer.addEventListener('mousedown', (e) => {
    isResizing = true;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    e.preventDefault();
  });

  document.addEventListener('mousemove', (e) => {
    if (!isResizing) return;
    const bodyRowRect = bodyRow.getBoundingClientRect();
    const newWidth = e.clientX - bodyRowRect.left;
    const minWidth = 100;
    const maxWidth = 500;
    if (newWidth > minWidth && newWidth < maxWidth) {
      bodyRow.style.gridTemplateColumns = `${newWidth}px 4px 1fr`;
    }
  });

  document.addEventListener('mouseup', () => {
    if (isResizing) {
      isResizing = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    }
  });
}

import { appState } from "./store";
import { MenuBar } from './components/MenuBar';
import { NeonButton, NeonInput, NeonTable, NeonModal, NeonProgressBar } from "./components";

async function main(): Promise<void> {
  initI18n();
  initTheming();
  bindNavTabs();
  bindSidebar();
  bindSidebarResizer();
  bindDrawer();
  await mountExplorer(); // load explorer tab
  await renderBookmarks();
  await renderTreeView();
  await restoreSession();

  // Khởi tạo MenuBar
  const menubarContainer = document.getElementById('menubar-container');
  if (menubarContainer) {
    const menuBar = new MenuBar();
    menubarContainer.appendChild(menuBar.getElement());
  }

  // Khởi tạo Transfer Manager và Drawer
  transferManager.init();
  new TransferDrawer();

  // Local pane hiển thị ngay, không cần đăng nhập; cloud pane tự khôi phục nếu đã có session
  mountExplorer();
  restoreSession();
}

main();
// Dummy usage to satisfy imports
console.log(appState);
new NeonButton('Test');
new NeonInput();
new NeonTable();
new NeonModal();
new NeonProgressBar();
