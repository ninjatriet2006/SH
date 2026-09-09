import { convertFileSrc } from '@tauri-apps/api/core';
import type { FontInfo, ThemeInfo } from '../../../bridge/appearance_api';

const appliedThemeVariables = new Set<string>();

export function applyTheme(theme: ThemeInfo) {
  const root = document.documentElement;
  appliedThemeVariables.forEach((name) => root.style.removeProperty(`--${name}`));
  appliedThemeVariables.clear();
  for (const [name, value] of Object.entries(theme.variables)) {
    if (!/^[a-zA-Z][a-zA-Z0-9-]*$/.test(name) || /[;{}]/.test(value)) continue;
    root.style.setProperty(`--${name}`, value);
    appliedThemeVariables.add(name);
  }
  root.dataset.theme = theme.id;
}

export function applyFont(font: FontInfo) {
  document.getElementById('rclone-dynamic-font')?.remove();
  const style = document.createElement('style');
  style.id = 'rclone-dynamic-font';
  const family = [...font.family].filter((ch) => ch.charCodeAt(0) >= 32 && !`'"\\<>{};`.includes(ch)).join('');
  const source = font.src_path ? JSON.stringify(convertFileSrc(font.src_path)) : '';
  style.textContent = font.src_path
    ? `@font-face{font-family:'${family}';src:url(${source});font-display:swap}:root{--typography-family-proportional:'${family}',system-ui,sans-serif}`
    : ':root{--typography-family-proportional:"Noto Sans",Roboto,"DejaVu Sans",system-ui,sans-serif}';
  document.head.appendChild(style);
  document.documentElement.dataset.font = font.id;
}
