import { convertFileSrc } from "@tauri-apps/api/core";

export type LanguageId = "en" | "vi";
export type ThemeId = "system" | "light" | "dark";
export type FontId = "system-default" | "dejavusans";
export type Dictionary = { [key: string]: string | Dictionary };
export interface ThemeAsset { id: ThemeId; tokens: Record<string, string>; light_tokens?: Record<string, string> }
export interface ResourcePaths {
  languages: Record<LanguageId, string>;
  themes: Record<ThemeId, string>;
  fonts: { primary: string | null };
}

export const FALLBACK_LANGUAGE: LanguageId = "en";
export const FALLBACK_THEME: ThemeId = "dark";
export const FALLBACK_FONT: FontId = "system-default";
const THEME_TOKENS = ["canvas", "foreground", "border", "panel", "panel_foreground", "accent", "error"];

function resourcePath(path: string, section: "langs" | "themes" | "fonts", extension: string): string {
  const normalized = path.replaceAll("\\", "/");
  const marker = `/${section}/`;
  const index = normalized.lastIndexOf(marker);
  const relative = index >= 0 ? normalized.slice(index + 1) : normalized;
  if (relative.startsWith("/") || relative.split("/").some((part) => !part || part === "." || part === "..") ||
      !relative.startsWith(`${section}/`) || !relative.endsWith(extension)) {
    throw new Error(`invalid ${section} resource path`);
  }
  return path;
}

function dictionary(value: unknown): Dictionary {
  if (!value || Array.isArray(value) || typeof value !== "object") throw new Error("invalid language asset");
  for (const child of Object.values(value)) if (typeof child !== "string") dictionary(child);
  return value as Dictionary;
}

function theme(value: unknown, expected: ThemeId): ThemeAsset {
  if (!value || typeof value !== "object") throw new Error("invalid theme asset");
  const asset = value as Partial<ThemeAsset>;
  if (asset.id !== expected || !asset.tokens || THEME_TOKENS.some((key) => typeof asset.tokens?.[key] !== "string")) throw new Error("invalid theme tokens");
  if (expected === "system" && (!asset.light_tokens || THEME_TOKENS.some((key) => typeof asset.light_tokens?.[key] !== "string"))) throw new Error("invalid system theme");
  return asset as ThemeAsset;
}

async function json(path: string): Promise<unknown> {
  const response = await fetch(convertFileSrc(path));
  // WebKit reports successful Tauri custom-protocol responses as status 0.
  // Parsing still validates that an opened resource contains valid JSON.
  if (!response.ok && response.status !== 0) throw new Error(`resource response ${response.status}`);
  return response.json();
}

export function recursiveKeys(value: Dictionary, prefix = ""): string[] {
  return Object.entries(value).flatMap(([key, child]) => {
    const full = prefix ? `${prefix}.${key}` : key;
    return typeof child === "string" ? [full] : recursiveKeys(child, full);
  }).sort();
}

export class ResourceLoader {
  constructor(private readonly paths: ResourcePaths) {}

  async loadLanguage(id: LanguageId): Promise<{ id: LanguageId; dictionary: Dictionary }> {
    let fallback: Dictionary;
    try { fallback = dictionary(await json(resourcePath(this.paths.languages[FALLBACK_LANGUAGE], "langs", ".json"))); }
    catch { return { id: FALLBACK_LANGUAGE, dictionary: {} }; }
    if (id === FALLBACK_LANGUAGE) return { id, dictionary: fallback };
    try {
      const selected = dictionary(await json(resourcePath(this.paths.languages[id], "langs", ".json")));
      if (recursiveKeys(selected).join("\n") !== recursiveKeys(fallback).join("\n")) throw new Error("language parity mismatch");
      return { id, dictionary: selected };
    } catch { return { id: FALLBACK_LANGUAGE, dictionary: fallback }; }
  }

  async loadTheme(id: ThemeId): Promise<ThemeAsset | null> {
    try { return theme(await json(resourcePath(this.paths.themes[id], "themes", ".json")), id); }
    catch {
      try { return theme(await json(resourcePath(this.paths.themes[FALLBACK_THEME], "themes", ".json")), FALLBACK_THEME); }
      catch { return null; }
    }
  }

  fontUrl(id: FontId): string | null {
    if (id !== "dejavusans" || !this.paths.fonts.primary) return null;
    try { return convertFileSrc(resourcePath(this.paths.fonts.primary, "fonts", ".ttf")); }
    catch { return null; }
  }
}

export function translation(dictionary: Dictionary, key: string, values: Record<string, string | number> = {}): string {
  let value: string | Dictionary | undefined = dictionary;
  for (const part of key.split(".")) value = typeof value === "object" ? value[part] : undefined;
  if (typeof value !== "string") return key;
  return Object.entries(values).reduce((text, [name, replacement]) => text.replaceAll(`{${name}}`, String(replacement)), value);
}

export function applyTheme(asset: ThemeAsset | null, light = matchMedia("(prefers-color-scheme: light)").matches): ThemeId {
  const tokens = asset?.id === "system" && light ? asset.light_tokens : asset?.tokens;
  for (const key of THEME_TOKENS) document.documentElement.style.setProperty(`--${key.replaceAll("_", "-")}`, tokens?.[key] ?? "");
  const id = asset?.id ?? FALLBACK_THEME;
  document.documentElement.dataset.theme = id;
  return id;
}

export async function applyFont(loader: ResourceLoader, id: FontId): Promise<FontId> {
  const url = loader.fontUrl(id);
  if (!url || typeof FontFace === "undefined") return applySystemFont();
  try {
    const family = "ImageSplitterDejaVu";
    const face = await new FontFace(family, `url(${JSON.stringify(url)})`).load();
    document.fonts.add(face);
    document.documentElement.dataset.font = id;
    document.documentElement.style.setProperty("--app-font", JSON.stringify(family));
    return id;
  } catch { return applySystemFont(); }
}

function applySystemFont(): FontId {
  document.documentElement.dataset.font = FALLBACK_FONT;
  document.documentElement.style.setProperty("--app-font", "system-ui, sans-serif");
  return FALLBACK_FONT;
}
