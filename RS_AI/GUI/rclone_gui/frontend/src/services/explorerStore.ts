/*
[INTEGRITY NOTES]
- Mục đích: explorerStore.ts — Trạng thái riêng biệt cho trình khám phá file (tách khỏi store.ts toàn cục).
- Trách nhiệm: Quản lý leftPath/rightPath/leftFiles/rightFiles, độ rộng cột, lịch sử điều hướng, lựa chọn file (selection) cho Dual Pane.
- Tương tác: Các component (DualPaneExplorer) sẽ đọc/ghi qua đây thay vì dùng `appState.explorer`. Lấy và lưu từ localStorage.
*/
import type { FileItem } from '../store';
import type { SortKey, SortDir } from '../features/sort';

export type Pane = 'left' | 'right';

/** Cột có thể kéo để đổi width (cột Modified cuối flex còn lại). */
export type PaneColKey = 'name' | 'type' | 'size' | 'date' | 'permissions' | 'owner' | 'group';

export interface PaneColWidths {
  name: number;
  type: number;
  size: number;
  date: number;
  permissions: number;
  owner: number;
  group: number;
}

export interface ExplorerSelection {
  pane: Pane;
  name: string;
  path: string;
  is_dir: boolean;
}

export interface ExplorerState {
  leftPath: string;
  rightPath: string;
  leftFiles: FileItem[];
  rightFiles: FileItem[];
  leftSelection: ExplorerSelection[];
  rightSelection: ExplorerSelection[];
  activePane: Pane;
  leftSortKey: SortKey;
  leftSortDir: SortDir;
  rightSortKey: SortKey;
  rightSortDir: SortDir;
  leftColWidths: PaneColWidths;
  rightColWidths: PaneColWidths;
  leftHistoryBack: string[];
  leftHistoryForward: string[];
  rightHistoryBack: string[];
  rightHistoryForward: string[];
  leftVisibleCols: string[];
  rightVisibleCols: string[];
}

const state: ExplorerState = {
  leftPath: '/',
  rightPath: '/',
  leftFiles: [],
  rightFiles: [],
  leftSelection: [],
  rightSelection: [],
  activePane: 'left',
  leftSortKey: 'name',
  leftSortDir: 'asc',
  rightSortKey: 'name',
  rightSortDir: 'asc',
  leftColWidths: { name: 300, type: 80, size: 80, date: 150, permissions: 100, owner: 100, group: 100 },
  rightColWidths: { name: 300, type: 80, size: 80, date: 150, permissions: 100, owner: 100, group: 100 },
  leftHistoryBack: [],
  leftHistoryForward: [],
  rightHistoryBack: [],
  rightHistoryForward: [],
  leftVisibleCols: ['name', 'type', 'size', 'date'],
  rightVisibleCols: ['name', 'type', 'size', 'date'],
};

// Khôi phục từ localStorage (tương tự như bookmarks/activityLog trong store.ts)
try {
  const savedState = localStorage.getItem('filen_explorer_state');
  if (savedState) {
    const parsed = JSON.parse(savedState);
    if (parsed.leftColWidths) state.leftColWidths = { ...state.leftColWidths, ...parsed.leftColWidths };
    if (parsed.rightColWidths) state.rightColWidths = { ...state.rightColWidths, ...parsed.rightColWidths };
    if (parsed.leftVisibleCols) state.leftVisibleCols = parsed.leftVisibleCols;
    if (parsed.rightVisibleCols) state.rightVisibleCols = parsed.rightVisibleCols;
  }
} catch (e) {
  console.warn('Failed to parse explorer state', e);
}

function saveExplorerState() {
  localStorage.setItem('filen_explorer_state', JSON.stringify({
    leftColWidths: state.leftColWidths,
    rightColWidths: state.rightColWidths,
    leftVisibleCols: state.leftVisibleCols,
    rightVisibleCols: state.rightVisibleCols,
  }));
}

export function getExplorerState(): ExplorerState {
  return state;
}

// ── Đường dẫn (Path) ────────────────────────────────────────────────────────
export function getPanePath(pane: Pane): string {
  return pane === 'left' ? state.leftPath : state.rightPath;
}

export function setPanePath(pane: Pane, path: string): void {
  if (pane === 'left') {
    state.leftPath = path;
  } else {
    state.rightPath = path;
  }
}

// ── Lịch sử điều hướng (History) ────────────────────────────────────────────
export function canPaneGoBack(pane: Pane): boolean {
  return pane === 'left' ? state.leftHistoryBack.length > 0 : state.rightHistoryBack.length > 0;
}

export function canPaneGoForward(pane: Pane): boolean {
  return pane === 'left' ? state.leftHistoryForward.length > 0 : state.rightHistoryForward.length > 0;
}

export function pushPaneHistory(pane: Pane, path: string): void {
  const current = getPanePath(pane);
  if (current === path) return; // Không lưu lịch sử trùng lặp nếu đang đứng yên tại chỗ
  if (pane === 'left') {
    state.leftHistoryBack.push(current);
    state.leftHistoryForward = [];
  } else {
    state.rightHistoryBack.push(current);
    state.rightHistoryForward = [];
  }
  setPanePath(pane, path);
}

export function popPaneBack(pane: Pane): string | null {
  const current = getPanePath(pane);
  let prev: string | undefined;
  if (pane === 'left') {
    prev = state.leftHistoryBack.pop();
    if (prev) state.leftHistoryForward.push(current);
  } else {
    prev = state.rightHistoryBack.pop();
    if (prev) state.rightHistoryForward.push(current);
  }
  if (prev) setPanePath(pane, prev);
  return prev || null;
}

export function popPaneForward(pane: Pane): string | null {
  const current = getPanePath(pane);
  let next: string | undefined;
  if (pane === 'left') {
    next = state.leftHistoryForward.pop();
    if (next) state.leftHistoryBack.push(current);
  } else {
    next = state.rightHistoryForward.pop();
    if (next) state.rightHistoryBack.push(current);
  }
  if (next) setPanePath(pane, next);
  return next || null;
}

// ── Danh sách File (Files) ──────────────────────────────────────────────────
export function getPaneFiles(pane: Pane): FileItem[] {
  return pane === 'left' ? state.leftFiles : state.rightFiles;
}

export function setPaneFiles(pane: Pane, files: FileItem[]): void {
  if (pane === 'left') {
    state.leftFiles = files;
  } else {
    state.rightFiles = files;
  }
}

// ── Selection (danh sách — hỗ trợ rubber-band chọn nhiều) ──────────────────
export function getPaneSelection(pane: Pane): ExplorerSelection[] {
  return pane === 'left' ? state.leftSelection : state.rightSelection;
}

export function setPaneSelection(pane: Pane, sels: ExplorerSelection[]): void {
  if (pane === 'left') {
    state.leftSelection = sels;
  } else {
    state.rightSelection = sels;
  }
}

export function clearPaneSelection(pane: Pane): void {
  setPaneSelection(pane, []);
}

// ── Khung thao tác hiện tại (Active Pane) ───────────────────────────────────
export function getActivePane(): Pane {
  return state.activePane;
}

export function setActivePane(pane: Pane): void {
  state.activePane = pane;
}

// ── Cấu hình sắp xếp (Sort - riêng từng pane) ──────────────────────────────
export function getPaneSortKey(pane: Pane): SortKey {
  return pane === 'left' ? state.leftSortKey : state.rightSortKey;
}

export function getPaneSortDir(pane: Pane): SortDir {
  return pane === 'left' ? state.leftSortDir : state.rightSortDir;
}

export function setPaneSort(pane: Pane, key: SortKey, dir: SortDir): void {
  if (pane === 'left') {
    state.leftSortKey = key;
    state.leftSortDir = dir;
  } else {
    state.rightSortKey = key;
    state.rightSortDir = dir;
  }
}

// ── Độ rộng cột hiển thị (Column Widths - riêng từng pane) ─────────────────
export function getPaneColWidths(pane: Pane): PaneColWidths {
  return pane === 'left' ? state.leftColWidths : state.rightColWidths;
}

export function setPaneColWidth(pane: Pane, key: PaneColKey, width: number): void {
  const target = pane === 'left' ? state.leftColWidths : state.rightColWidths;
  target[key] = width;
  saveExplorerState();
}

// ── Trạng thái hiển thị cột (Visible Columns - riêng từng pane) ─────────────
export function getPaneVisibleCols(pane: Pane): string[] {
  return pane === 'left' ? state.leftVisibleCols : state.rightVisibleCols;
}

export function setPaneVisibleCols(pane: Pane, cols: string[]): void {
  if (pane === 'left') {
    state.leftVisibleCols = cols;
  } else {
    state.rightVisibleCols = cols;
  }
  saveExplorerState();
}