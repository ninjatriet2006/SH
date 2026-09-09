export interface AppSettings {
  showHiddenFiles: boolean;
  language: string;
  theme: string;
  font: string;
}

export const DEFAULT_SETTINGS: AppSettings = {
  showHiddenFiles: true,
  language: "en",
  theme: "default",
  font: "default",
};

export function normalizeSettings(value: unknown): AppSettings {
  if (typeof value !== "object" || value === null) return { ...DEFAULT_SETTINGS };
  const saved = value as Partial<AppSettings>;
  return {
    showHiddenFiles: typeof saved.showHiddenFiles === "boolean" ? saved.showHiddenFiles : DEFAULT_SETTINGS.showHiddenFiles,
    language: typeof saved.language === "string" ? saved.language : DEFAULT_SETTINGS.language,
    theme: typeof saved.theme === "string" ? saved.theme : DEFAULT_SETTINGS.theme,
    font: typeof saved.font === "string" ? saved.font : DEFAULT_SETTINGS.font,
  };
}
