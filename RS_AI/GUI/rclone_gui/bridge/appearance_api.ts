import { invoke } from './ipc';

export interface ThemeInfo {
  id: string;
  name: string;
  variables: Record<string, string>;
}

export interface FontInfo {
  id: string;
  name: string;
  family: string;
  src_path: string | null;
}

export const getAvailableThemes = () => invoke<ThemeInfo[]>('get_available_themes');
export const getAvailableFonts = () => invoke<FontInfo[]>('get_available_fonts');
