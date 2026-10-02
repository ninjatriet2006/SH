import { convertFileSrc } from "@tauri-apps/api/core";
import type { FontId, Language, Theme } from "./contract";

export type Messages = Record<string, string | Record<string, string>>;

export interface ThemeResource {
  id: Theme;
  tokens: Record<string, string>;
  light_tokens?: Record<string, string>;
}

export interface ResourcePaths {
  languages: Record<Language, string | null>;
  themes: Record<Theme, string | null>;
  fonts: { primary: string | null };
}

declare global { interface Window { __UNIVERSE_MANAGER_RESOURCES__?: ResourcePaths } }

const fallbackMessages: Record<Language, Messages> = {
  en: {
    app: { title: "Universe Manager" },
    nav: { dashboard: "Dashboard", config: "Config", scan: "Scan Apps", manager: "App Manager", add: "Add App", search: "Search", debug: "Debug Logs", settings: "Settings" },
    settings: { language: "Language", theme: "Theme", font: "Font", system: "System", light: "Light", dark: "Dark", system_default: "System default" },
    dashboard: { managed_apps: "Managed apps", custom_apps: "Custom apps", managed_dir: "Managed directory", not_configured: "Not configured", getting_started: "Getting Started with Universe Manager", getting_started_desc: "No managed directory configured. Please select or enter your applications directory to start scanning.", ready_to_scan: "Ready to scan applications", ready_to_scan_desc: "Managed directory is configured. Click Scan Apps to detect your applications automatically.", apps_ready: "All applications are ready to manage and launch." },
    config: { managed_dir: "Managed directory", managed_dir_desc: "Enter an absolute path or click 'Select…' to choose via file dialog.", select: "Select…", save: "Save config" },
    scan: { description: "Scan the managed directory or inspect an app selected from a source directory.", need_config_desc: "Please set up a managed directory in Config before scanning applications.", scan_btn: "Scan managed apps", detect_btn: "Select & detect source…" },
    manager: { start: "Start", stop: "Stop" },
    search: { placeholder: "App name", search_btn: "Search" },
    table: { name: "Name", version: "Version", type: "Type", actions: "Actions", id: "ID", source: "Source" },
    status: { no_apps: "No applications.", no_results: "No results.", starting: "Starting…", saved: "Saved" },
    detection: { appimage: "AppImage", executables: "Executables", icons: "Icons", desktop: "Desktop templates" },
    confirm: { start: "Start {name}?", stop: "Stop {name}?" },
  },
  vi: {
    app: { title: "Universe Manager" },
    nav: { dashboard: "Tổng quan", config: "Cấu hình", scan: "Quét ứng dụng", manager: "Quản lý ứng dụng", add: "Thêm ứng dụng", search: "Tìm kiếm", debug: "Nhật ký", settings: "Cài đặt" },
    settings: { language: "Ngôn ngữ", theme: "Giao diện", font: "Phông chữ", system: "Hệ thống", light: "Sáng", dark: "Tối", system_default: "Mặc định hệ thống" },
    dashboard: { managed_apps: "Ứng dụng quản lý", custom_apps: "Ứng dụng tùy chỉnh", managed_dir: "Thư mục quản lý", not_configured: "Chưa cấu hình", getting_started: "Bắt đầu sử dụng Universe Manager", getting_started_desc: "Bạn chưa chọn thư mục quản lý. Hãy chọn hoặc nhập đường dẫn thư mục chứa ứng dụng của bạn để bắt đầu quét.", ready_to_scan: "Sẵn sàng quét ứng dụng", ready_to_scan_desc: "Thư mục quản lý đã sẵn sàng. Hãy bấm Quét ứng dụng để tự động phát hiện các phần mềm.", apps_ready: "Tất cả ứng dụng đã sẵn sàng quản lý và khởi chạy." },
    config: { managed_dir: "Thư mục quản lý", managed_dir_desc: "Nhập đường dẫn tuyệt đối hoặc bấm 'Chọn…' để mở hộp thoại chọn thư mục.", select: "Chọn…", save: "Lưu cấu hình" },
    scan: { description: "Quét thư mục quản lý hoặc kiểm tra ứng dụng từ thư mục nguồn.", need_config_desc: "Vui lòng thiết lập thư mục quản lý trong trang Cấu hình trước khi thực hiện quét ứng dụng.", scan_btn: "Quét ứng dụng", detect_btn: "Chọn & kiểm tra nguồn…" },
    manager: { start: "Khởi động", stop: "Dừng" },
    search: { placeholder: "Tên ứng dụng", search_btn: "Tìm kiếm" },
    table: { name: "Tên", version: "Phiên bản", type: "Loại", actions: "Thao tác", id: "ID", source: "Nguồn" },
    status: { no_apps: "Không có ứng dụng.", no_results: "Không có kết quả.", starting: "Đang khởi động…", saved: "Đã lưu" },
    detection: { appimage: "AppImage", executables: "Tệp thực thi", icons: "Biểu tượng", desktop: "Mẫu desktop" },
    confirm: { start: "Khởi động {name}?", stop: "Dừng {name}?" },
  },
};

const dark: Record<string, string> = {
  canvas: "#0f172a",
  canvas_end: "#020617",
  foreground: "#f8fafc",
  border: "rgba(255, 255, 255, 0.1)",
  panel: "rgba(30, 41, 59, 0.7)",
  "panel-foreground": "#94a3b8",
  "sidebar-bg": "rgba(15, 23, 42, 0.85)",
  accent: "#6366f1",
  "accent-hover": "#818cf8",
  error: "#f87171",
  "error-bg": "rgba(239, 68, 68, 0.15)",
  "error-border": "rgba(239, 68, 68, 0.35)",
  "input-bg": "rgba(15, 23, 42, 0.6)",
};
const light: Record<string, string> = {
  canvas: "#f8fafc",
  canvas_end: "#e2e8f0",
  foreground: "#0f172a",
  border: "rgba(0, 0, 0, 0.12)",
  panel: "rgba(255, 255, 255, 0.88)",
  "panel-foreground": "#475569",
  "sidebar-bg": "rgba(255, 255, 255, 0.95)",
  accent: "#4f46e5",
  "accent-hover": "#6366f1",
  error: "#dc2626",
  "error-bg": "rgba(220, 38, 38, 0.1)",
  "error-border": "rgba(220, 38, 38, 0.3)",
  "input-bg": "#ffffff",
};

export async function loadBundledJson<T>(
  path: string | null | undefined, fallback: T,
): Promise<T> {
  if (!path) return fallback;
  try {
    const response = await fetch(convertFileSrc(path));
    return response.ok ? await response.json() as T : fallback;
  } catch { return fallback; }
}

export async function loadMessages(language: Language): Promise<Messages> {
  const paths = window.__UNIVERSE_MANAGER_RESOURCES__;
  const fallback = fallbackMessages[language] ?? fallbackMessages.en;
  const loaded = await loadBundledJson<Messages>(paths?.languages[language], fallback);
  const merged: Messages = { ...fallback };
  for (const [k, v] of Object.entries(loaded)) {
    if (typeof v === "object" && v !== null && typeof merged[k] === "object" && merged[k] !== null) {
      merged[k] = { ...(merged[k] as Record<string, string>), ...v };
    } else {
      merged[k] = v;
    }
  }
  return merged;
}

export async function loadAndApplyTheme(theme: Theme, fontId: FontId): Promise<void> {
  const paths = window.__UNIVERSE_MANAGER_RESOURCES__;
  const fallbackTheme: ThemeResource = theme === "light"
    ? { id: "light", tokens: light }
    : theme === "dark" ? { id: "dark", tokens: dark } : { id: "system", tokens: dark, light_tokens: light };
  const themeData = await loadBundledJson(paths?.themes[theme], fallbackTheme);
  applyTheme(themeData, fontId, paths?.fonts.primary);
}

function applyTheme(theme: ThemeResource, font: FontId, fontPath?: string | null): void {
  const prefersLight = matchMedia("(prefers-color-scheme: light)").matches;
  const isLight = theme.id === "light" || (theme.id === "system" && prefersLight);
  const tokens = isLight && theme.light_tokens ? theme.light_tokens : theme.tokens;
  for (const [name, value] of Object.entries(tokens)) {
    document.documentElement.style.setProperty(`--${name.replaceAll("_", "-")}`, value);
  }
  document.documentElement.style.colorScheme = isLight ? "light" : "dark";
  document.documentElement.setAttribute("data-theme", isLight ? "light" : "dark");
  if (font === "dejavusans" && fontPath) {
    try {
      const face = new FontFace("Universe DejaVu", `url(${JSON.stringify(convertFileSrc(fontPath))})`);
      void face.load().then((loaded) => {
        document.fonts.add(loaded);
        document.documentElement.style.setProperty("--font", '"Universe DejaVu", system-ui, sans-serif');
      }).catch(() => undefined);
    } catch { /* Keep the system-default font. */ }
  }
}

export function translate(messages: Messages, key: string): string {
  const [group, item] = key.split(".");
  const value = group && item ? messages[group] : undefined;
  if (typeof value === "object" && typeof value[item] === "string") {
    return value[item];
  }
  const enGroup = group ? fallbackMessages.en[group] : undefined;
  if (typeof enGroup === "object" && item && typeof enGroup[item] === "string") {
    return enGroup[item];
  }
  return key;
}

