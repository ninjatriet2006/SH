import { convertFileSrc } from "@tauri-apps/api/core";
import { ThemeManager, type ThemeEntry } from "./themes/ThemeManager";

export interface FontEntry {
  id: string;
  name: string;
  path: string;
}

export const themeManager = new ThemeManager();
let themes: ThemeEntry[] = [];
let fonts: FontEntry[] = [];
const loadedFonts = new Map<string, FontFace>();

export async function discoverAppearance(): Promise<void> {
  themes = (await themeManager.loadAll()).filter((theme) => theme.valid);
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    fonts = await invoke<FontEntry[]>("appearance_list_fonts");
  } catch {
    fonts = [];
  }
}

export function availableThemes(): ThemeEntry[] {
  return themes;
}

export function availableFonts(): FontEntry[] {
  return fonts;
}

function resolvedTheme(id: string): ThemeEntry | null {
  return themes.find((theme) => theme.slug === id) ?? null;
}

export function resolvedFont(id: string): FontEntry | null {
  return fonts.find((font) => font.id === id) ?? null;
}

export function applyTheme(id: string): string {
  const selected = resolvedTheme(id);
  themeManager.apply(selected);
  return selected?.slug ?? "default";
}

export async function applyFont(id: string): Promise<string> {
  if (id === "default") {
    themeManager.removeRuntimeTweak("typography.family.proportional");
    return "default";
  }

  const selected = resolvedFont(id);
  if (!selected) {
    themeManager.removeRuntimeTweak("typography.family.proportional");
    return "default";
  }

  const family = `FilenLocalFont${selected.id.replace(/[^a-zA-Z0-9]/g, "")}`;
  let face = loadedFonts.get(selected.id);
  if (!face) {
    face = new FontFace(family, `url(${JSON.stringify(convertFileSrc(selected.path))})`);
    await face.load();
    document.fonts.add(face);
    loadedFonts.set(selected.id, face);
  }
  themeManager.setRuntimeTweak("typography.family.proportional", `"${family}", system-ui, sans-serif`);
  return selected.id;
}
