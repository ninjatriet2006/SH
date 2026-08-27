/*
[INTEGRITY NOTES]
- Mục đích: Khai báo cấu trúc dữ liệu chung (Interfaces) và trạng thái toàn cục cơ bản của ứng dụng.
- Trách nhiệm: Lưu trữ và khôi phục nhật ký hoạt động (activity log), dấu trang (bookmarks), cài đặt chung từ localStorage.
- Tương tác: Được import bởi hầu hết các modules và components để ghi log, theo dõi cấu hình.
*/

// Định nghĩa cấu trúc dữ liệu trạng thái của ứng dụng
export interface FileItem {
  uuid: string;
  name: string;
  is_dir: boolean;
  size: number;
  mod_time: string;
  /** Loại hiển thị: "Folder" hoặc extension viết hoa (PDF, PNG…). */
  file_type?: string | null;
  owner?: string;
  group?: string;
  permissions?: string;
}

export interface ActivityItem {
  id: string;
  timestamp: number;
  action: string;
  details: string;
}

export interface BookmarkItem {
  name: string;
  path: string;
}

export interface AppSettings {
  showHiddenFiles: boolean;
  language: string;
  theme: string;
}

export interface AppState {
  auth?: {
    user?: string;
    twoFAEnabled?: boolean;
  };
  explorer?: {
    leftPath?: string;
    rightPath?: string;
    leftFiles?: FileItem[];
    rightFiles?: FileItem[];
  };
  activityLog?: ActivityItem[];
  bookmarks?: BookmarkItem[];
  settings?: AppSettings;
}

// Trạng thái biến đổi toàn cục (Global mutable state - giữ đơn giản cho phiên bản hiện tại)
export const appState: AppState = {};

// Khôi phục nhật ký hoạt động (activity log) từ localStorage
try {
  const savedLog = localStorage.getItem('filen_activity_log');
  if (savedLog) {
    appState.activityLog = JSON.parse(savedLog);
  }
} catch (e) {
  console.warn('Failed to parse activity log', e);
}

if (!appState.activityLog) {
  appState.activityLog = [];
}

try {
  const savedBookmarks = localStorage.getItem('filen_bookmarks');
  if (savedBookmarks) {
    appState.bookmarks = JSON.parse(savedBookmarks);
  }
} catch (e) {
  console.warn('Failed to parse bookmarks', e);
}

if (!appState.bookmarks) {
  appState.bookmarks = [];
}

/** Ghi log hành động và lưu vào localStorage (tối đa 200 bản ghi). */
export function logActivity(action: string, details: string) {
  const item: ActivityItem = {
    id: Math.random().toString(36).substring(2, 9),
    timestamp: Date.now(),
    action,
    details
  };
  
  if (!appState.activityLog) appState.activityLog = [];
  appState.activityLog.unshift(item); // Thêm vào đầu
  
  // Giới hạn 200 bản ghi
  if (appState.activityLog.length > 200) {
    appState.activityLog.pop();
  }
  
  try {
    localStorage.setItem('filen_activity_log', JSON.stringify(appState.activityLog));
  } catch (e) {
    console.warn('Failed to save activity log', e);
  }
}

export function isBookmarked(path: string): boolean {
  return appState.bookmarks?.some(b => b.path === path) ?? false;
}

export function toggleBookmark(name: string, path: string) {
  if (!appState.bookmarks) appState.bookmarks = [];
  
  if (isBookmarked(path)) {
    appState.bookmarks = appState.bookmarks.filter(b => b.path !== path);
  } else {
    appState.bookmarks.push({ name, path });
  }
  
  try {
    localStorage.setItem('filen_bookmarks', JSON.stringify(appState.bookmarks));
  } catch (e) {
    console.warn('Failed to save bookmarks', e);
  }
  
  // Kích hoạt sự kiện (Dispatch event) để cập nhật lại giao diện (UI)
  window.dispatchEvent(new Event('bookmarks-updated'));
}

export function saveSettings() {
  if (appState.settings) {
    try {
      localStorage.setItem('filen_settings', JSON.stringify(appState.settings));
    } catch (e) {
      console.warn('Failed to save settings', e);
    }
  }
}

