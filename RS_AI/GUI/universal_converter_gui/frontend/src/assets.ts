import { convertFileSrc } from "@tauri-apps/api/core";

export type LanguageId = "en" | "vi";
export type ThemeId = "system" | "light" | "dark";
export type FontId = "system-default" | "dejavusans";
export type Dictionary = { [key: string]: string | Dictionary };

export interface ThemeAsset {
  id: ThemeId;
  tokens: Record<string, string>;
  light_tokens?: Record<string, string>;
}

export interface ResourcePaths {
  languages: Record<LanguageId, string>;
  themes: Record<ThemeId, string>;
  fonts: {
    primary: string | null;
  };
}

export interface FontSource {
  id: "dejavusans";
  kind: "bundled";
  url: string;
}

export const FALLBACK_LANGUAGE: LanguageId = "vi";
export const FALLBACK_THEME: ThemeId = "system";
export const FALLBACK_FONT: FontId = "system-default";
const REQUIRED_THEME_TOKENS = ["canvas", "foreground", "border", "panel", "panel_foreground", "accent", "error"];

function assertResourcePath(path: string, section: "langs" | "themes" | "fonts", extension: string): string {
  const normalized = path.replaceAll("\\", "/");
  const marker = `/${section}/`;
  const index = normalized.lastIndexOf(marker);
  const relative = index >= 0 ? normalized.slice(index + 1) : normalized;
  if (relative.startsWith("/") || relative.split("/").some((part) => !part || part === "." || part === "..")) {
    throw new Error(`resource path escapes ${section}`);
  }
  if (!relative.startsWith(`${section}/`) || !relative.endsWith(extension)) throw new Error(`invalid ${section} resource path`);
  return path;
}

function dictionary(value: unknown): Dictionary {
  if (!value || Array.isArray(value) || typeof value !== "object") throw new Error("invalid language asset");
  for (const child of Object.values(value)) {
    if (typeof child !== "string") dictionary(child);
  }
  return value as Dictionary;
}

function theme(value: unknown, expected: ThemeId): ThemeAsset {
  if (!value || typeof value !== "object") throw new Error("invalid theme asset");
  const candidate = value as Partial<ThemeAsset>;
  if (candidate.id !== expected || !candidate.tokens || REQUIRED_THEME_TOKENS.some((token) => typeof candidate.tokens?.[token] !== "string")) {
    throw new Error("invalid theme tokens");
  }
  if (expected === "system" && (!candidate.light_tokens || REQUIRED_THEME_TOKENS.some((token) => typeof candidate.light_tokens?.[token] !== "string"))) {
    throw new Error("invalid system light tokens");
  }
  return candidate as ThemeAsset;
}

async function json(path: string): Promise<unknown> {
  const response = await fetch(convertFileSrc(path));
  if (!response.ok) throw new Error(`resource response ${response.status}`);
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
    try {
      fallback = dictionary(await json(assertResourcePath(this.paths.languages[FALLBACK_LANGUAGE], "langs", ".json")));
    } catch {
      return { id: FALLBACK_LANGUAGE, dictionary: {} };
    }
    if (id === FALLBACK_LANGUAGE) return { id, dictionary: fallback };
    try {
      const selected = dictionary(await json(assertResourcePath(this.paths.languages[id], "langs", ".json")));
      if (recursiveKeys(selected).join("\n") !== recursiveKeys(fallback).join("\n")) throw new Error("language key parity mismatch");
      return { id, dictionary: selected };
    } catch {
      return { id: FALLBACK_LANGUAGE, dictionary: fallback };
    }
  }

  async loadTheme(id: ThemeId): Promise<ThemeAsset | null> {
    try {
      return theme(await json(assertResourcePath(this.paths.themes[id], "themes", ".json")), id);
    } catch {
      try {
        return theme(await json(assertResourcePath(this.paths.themes[FALLBACK_THEME], "themes", ".json")), FALLBACK_THEME);
      } catch {
        return null;
      }
    }
  }

  fontSource(id: FontId): FontSource | null {
    if (id !== "dejavusans" || !this.paths.fonts.primary) return null;
    try {
      return { id, kind: "bundled", url: convertFileSrc(assertResourcePath(this.paths.fonts.primary, "fonts", ".ttf")) };
    } catch {
      return null;
    }
  }
}

export function translation(dictionary: Dictionary, key: string, values: Record<string, string | number> = {}): string {
  let value: string | Dictionary | undefined = dictionary;
  for (const part of key.split(".")) value = typeof value === "object" ? value[part] : undefined;
  if (typeof value !== "string") return key;
  return Object.entries(values).reduce((text, [name, replacement]) => text.replaceAll(`{${name}}`, String(replacement)), value);
}

export function applyThemeAsset(asset: ThemeAsset | null, prefersLight = matchMedia("(prefers-color-scheme: light)").matches): void {
  const tokens = asset?.id === "system" && prefersLight ? asset.light_tokens : asset?.tokens;
  for (const name of REQUIRED_THEME_TOKENS) document.documentElement.style.setProperty(`--${name.replaceAll("_", "-")}`, tokens?.[name] ?? "");
  document.documentElement.dataset.theme = asset?.id ?? FALLBACK_THEME;
}

export async function applyFontAsset(loader: ResourceLoader, id: FontId): Promise<FontId> {
  const selected = loader.fontSource(id);
  if (!selected || typeof FontFace === "undefined") {
    applySystemFont();
    return FALLBACK_FONT;
  }
  try {
    const family = "UniversalConverterDejaVu";
    const face = await new FontFace(family, `url(${JSON.stringify(selected.url)})`).load();
    document.fonts.add(face);
    document.documentElement.dataset.font = selected.id;
    document.documentElement.dataset.fontSource = selected.kind;
    document.documentElement.style.setProperty("--app-font", JSON.stringify(family));
    return selected.id;
  } catch {
    applySystemFont();
    return FALLBACK_FONT;
  }
}

function applySystemFont(): void {
  document.documentElement.dataset.font = FALLBACK_FONT;
  document.documentElement.dataset.fontSource = "system";
  document.documentElement.style.setProperty("--app-font", "system-ui, sans-serif");
}
