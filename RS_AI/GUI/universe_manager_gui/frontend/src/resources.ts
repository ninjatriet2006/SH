import { convertFileSrc } from "@tauri-apps/api/core";
import type { FontId, Language, Preferences, Theme } from "./contract";

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
    nav: { dashboard: "Dashboard", config: "Config", scan: "Scan Apps", manager: "App Manager", search: "Search", settings: "Settings" },
    settings: { language: "Language", theme: "Theme", font: "Font", system: "System", light: "Light", dark: "Dark", system_default: "System default" },
  },
  vi: {
    app: { title: "Universe Manager" },
    nav: { dashboard: "Tổng quan", config: "Cấu hình", scan: "Quét ứng dụng", manager: "Quản lý ứng dụng", search: "Tìm kiếm", settings: "Cài đặt" },
    settings: { language: "Ngôn ngữ", theme: "Giao diện", font: "Phông chữ", system: "Hệ thống", light: "Sáng", dark: "Tối", system_default: "Mặc định hệ thống" },
  },
};
const dark = { canvas: "#111827", foreground: "#e5e7eb", border: "#374151", panel: "#0b1020", panel_foreground: "#d1d5db", accent: "#2563eb", error: "#f87171" };
const light = { canvas: "#f3f4f6", foreground: "#111827", border: "#d1d5db", panel: "#ffffff", panel_foreground: "#1f2937", accent: "#2563eb", error: "#b91c1c" };

type AssetConverter = (path: string) => string;
type AssetFetcher = (input: string) => Promise<Pick<Response, "ok" | "json">>;

export async function loadBundledJson<T>(
  path: string | null | undefined,
  fallback: T,
  converter: AssetConverter = convertFileSrc,
  fetcher: AssetFetcher = fetch,
): Promise<T> {
  if (!path) return fallback;
  try {
    const response = await fetcher(converter(path));
    return response.ok ? await response.json() as T : fallback;
  } catch { return fallback; }
}

export async function loadResources(preferences: Preferences): Promise<Messages> {
  const paths = window.__UNIVERSE_MANAGER_RESOURCES__;
  const messages = await loadBundledJson(paths?.languages[preferences.language], fallbackMessages[preferences.language]);
  const fallbackTheme: ThemeResource = preferences.theme === "light"
    ? { id: "light", tokens: light }
    : preferences.theme === "dark" ? { id: "dark", tokens: dark } : { id: "system", tokens: dark, light_tokens: light };
  const theme = await loadBundledJson(paths?.themes[preferences.theme], fallbackTheme);
  applyTheme(theme, preferences.font_id, paths?.fonts.primary);
  document.documentElement.lang = preferences.language;
  return messages;
}

export function applyTheme(theme: ThemeResource, font: FontId, fontPath?: string | null, converter: AssetConverter = convertFileSrc): void {
  const prefersLight = matchMedia("(prefers-color-scheme: light)").matches;
  const tokens = theme.id === "system" && prefersLight && theme.light_tokens ? theme.light_tokens : theme.tokens;
  for (const [name, value] of Object.entries(tokens)) document.documentElement.style.setProperty(`--${name.replaceAll("_", "-")}`, value);
  document.documentElement.style.setProperty("--font", "system-ui, sans-serif");
  if (font === "dejavusans" && fontPath) {
    try {
      const face = new FontFace("Universe DejaVu", `url(${JSON.stringify(converter(fontPath))})`);
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
  return typeof value === "object" && typeof value[item] === "string" ? value[item] : key;
}
