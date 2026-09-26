/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái và tác vụ của Dual-Pane File Explorer (2 khung trái - phải).
- Trách nhiệm: Nạp danh sách file, quản lý đường dẫn, lịch sử điều hướng, lựa chọn, sắp xếp, tìm kiếm.
- Tương tác: Dùng `files_bridge.ts` và `events_bridge.ts`.
*/

import { create } from 'zustand';
import { subscribeLocalDirChanged } from '../../../bridge/events_bridge';
import { getHomeDir, getUserPlaces, listFiles } from '../../../bridge/files_bridge';
import type { FileItem, UserPlace } from '../../../bridge/types';
import { getParentPath } from '../utils/formatters';

export type PaneType = 'left' | 'right';
export type SortKey = 'name' | 'size' | 'mod_time';
export type SortDir = 'asc' | 'desc';

export interface PaneState {
  path: string;
  files: FileItem[];
  selectedIds: Set<string>;
  history: string[];
  historyIndex: number;
  sortKey: SortKey;
  sortDir: SortDir;
  searchQuery: string;
  isLoading: boolean;
}

interface ExplorerStore {
  left: PaneState;
  right: PaneState;
  activePane: PaneType;
  homeDir: string;
  userPlaces: UserPlace[];
  bookmarks: Array<{ name: string; path: string }>;

  initExplorer: () => Promise<void>;
  setActivePane: (pane: PaneType) => void;
  loadDirectory: (pane: PaneType, path: string, recordHistory?: boolean) => Promise<void>;
  navigateUp: (pane: PaneType) => Promise<void>;
  goBack: (pane: PaneType) => Promise<void>;
  goForward: (pane: PaneType) => Promise<void>;
  setSort: (pane: PaneType, sortKey: SortKey) => void;
  toggleSelect: (pane: PaneType, uuid: string, multi?: boolean) => void;
  selectAll: (pane: PaneType) => void;
  clearSelection: (pane: PaneType) => void;
  setSearchQuery: (pane: PaneType, query: string) => void;
  toggleBookmark: (name: string, path: string) => void;
  refreshPane: (pane: PaneType) => Promise<void>;
  refreshActive: () => Promise<void>;
}

const createInitialPaneState = (path = ''): PaneState => ({
  path,
  files: [],
  selectedIds: new Set<string>(),
  history: path ? [path] : [],
  historyIndex: 0,
  sortKey: 'name',
  sortDir: 'asc',
  searchQuery: '',
  isLoading: false,
});

export const useExplorerStore = create<ExplorerStore>((set, get) => {
  // Lấy bookmarks từ localStorage nếu có
  let initialBookmarks: Array<{ name: string; path: string }> = [];
  try {
    const raw = localStorage.getItem('rclonegui_bookmarks');
    if (raw) initialBookmarks = JSON.parse(raw);
  } catch {
    initialBookmarks = [];
  }

  return {
    left: createInitialPaneState(),
    right: createInitialPaneState(),
    activePane: 'left',
    homeDir: '',
    userPlaces: [],
    bookmarks: initialBookmarks,

    initExplorer: async () => {
      try {
        const [home, places] = await Promise.all([
          getHomeDir().catch(() => '/'),
          getUserPlaces().catch(() => []),
        ]);

        set({ homeDir: home, userPlaces: places });

        // Tải thư mục mặc định cho 2 pane
        await Promise.all([
          get().loadDirectory('left', home, true),
          get().loadDirectory('right', home, true),
        ]);

        // Đăng ký lắng nghe sự kiện local-dir-changed từ backend
        await subscribeLocalDirChanged(() => {
          const { left, right } = get();
          // Nếu pane nào là local (không chứa ::) thì làm mới
          if (left.path && !left.path.includes('::')) {
            get().refreshPane('left');
          }
          if (right.path && !right.path.includes('::')) {
            get().refreshPane('right');
          }
        });
      } catch (err) {
        console.error('Lỗi initExplorer:', err);
      }
    },

    setActivePane: (pane: PaneType) => {
      set({ activePane: pane });
    },

    loadDirectory: async (pane: PaneType, path: string, recordHistory = true) => {
      if (!path) return;
      const targetState = get()[pane];

      set((state) => ({
        [pane]: { ...state[pane], isLoading: true },
      }));

      try {
        const fileList = await listFiles(path, pane);

        // Cập nhật lịch sử
        let newHistory = targetState.history;
        let newIndex = targetState.historyIndex;

        if (recordHistory) {
          if (newHistory.length === 0 || newHistory[newIndex] !== path) {
            newHistory = newHistory.slice(0, newIndex + 1).concat(path);
            newIndex = newHistory.length - 1;
          }
        }

        set((state) => ({
          [pane]: {
            ...state[pane],
            path,
            files: fileList,
            selectedIds: new Set<string>(),
            history: newHistory,
            historyIndex: newIndex,
            isLoading: false,
          },
        }));
      } catch (err) {
        console.error(`Lỗi tải thư mục [${pane}] ${path}:`, err);
        set((state) => ({
          [pane]: { ...state[pane], isLoading: false },
        }));
      }
    },

    navigateUp: async (pane: PaneType) => {
      const currentPath = get()[pane].path;
      const parent = getParentPath(currentPath);
      if (parent && parent !== currentPath) {
        await get().loadDirectory(pane, parent, true);
      }
    },

    goBack: async (pane: PaneType) => {
      const state = get()[pane];
      if (state.historyIndex > 0) {
        const newIndex = state.historyIndex - 1;
        const targetPath = state.history[newIndex];
        set((s) => ({
          [pane]: { ...s[pane], historyIndex: newIndex },
        }));
        await get().loadDirectory(pane, targetPath, false);
      }
    },

    goForward: async (pane: PaneType) => {
      const state = get()[pane];
      if (state.historyIndex < state.history.length - 1) {
        const newIndex = state.historyIndex + 1;
        const targetPath = state.history[newIndex];
        set((s) => ({
          [pane]: { ...s[pane], historyIndex: newIndex },
        }));
        await get().loadDirectory(pane, targetPath, false);
      }
    },

    setSort: (pane: PaneType, sortKey: SortKey) => {
      const current = get()[pane];
      const sortDir = current.sortKey === sortKey && current.sortDir === 'asc' ? 'desc' : 'asc';
      set((state) => ({
        [pane]: { ...state[pane], sortKey, sortDir },
      }));
    },

    toggleSelect: (pane: PaneType, uuid: string, multi = false) => {
      set((state) => {
        const paneState = state[pane];
        const newSelected = new Set(multi ? paneState.selectedIds : []);
        if (newSelected.has(uuid)) {
          newSelected.delete(uuid);
        } else {
          newSelected.add(uuid);
        }
        return {
          [pane]: { ...paneState, selectedIds: newSelected },
          activePane: pane,
        };
      });
    },

    selectAll: (pane: PaneType) => {
      set((state) => {
        const paneState = state[pane];
        const allIds = new Set(paneState.files.map((f) => f.uuid));
        return {
          [pane]: { ...paneState, selectedIds: allIds },
          activePane: pane,
        };
      });
    },

    clearSelection: (pane: PaneType) => {
      set((state) => ({
        [pane]: { ...state[pane], selectedIds: new Set<string>() },
      }));
    },

    setSearchQuery: (pane: PaneType, query: string) => {
      set((state) => ({
        [pane]: { ...state[pane], searchQuery: query },
      }));
    },

    toggleBookmark: (name: string, path: string) => {
      const current = get().bookmarks;
      const exists = current.some((b) => b.path === path);
      let updated: Array<{ name: string; path: string }>;
      if (exists) {
        updated = current.filter((b) => b.path !== path);
      } else {
        updated = [...current, { name, path }];
      }
      set({ bookmarks: updated });
      try {
        localStorage.setItem('rclonegui_bookmarks', JSON.stringify(updated));
      } catch (e) {
        console.error('Lỗi lưu bookmarks:', e);
      }
    },

    refreshPane: async (pane: PaneType) => {
      const path = get()[pane].path;
      if (path) {
        await get().loadDirectory(pane, path, false);
      }
    },

    refreshActive: async () => {
      const active = get().activePane;
      await get().refreshPane(active);
    },
  };
});
