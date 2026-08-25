// features/dragDrop.ts — Payload kéo thả file giữa các pane.
//
// Dùng DataTransfer chuẩn (HTML5 DnD) — text/plain chứa JSON:
//   { pane: 'left' | 'right', paths: string[] }
// Tauri webview hỗ trợ HTML5 DnD nên không cần plugin.
import type { Pane } from '../services/explorerStore';

export interface DragPayload {
  pane: Pane;
  paths: string[];
}

export function serializeDrag(p: DragPayload): string {
  return JSON.stringify(p);
}

/** Parse payload từ dataTransfer; trả null nếu không hợp lệ (kéo từ ngoài app). */
export function parseDrag(text: string): DragPayload | null {
  try {
    const obj = JSON.parse(text) as Partial<DragPayload>;
    if (obj && typeof obj === 'object' && (obj.pane === 'left' || obj.pane === 'right') && Array.isArray(obj.paths)) {
      return { pane: obj.pane, paths: obj.paths.filter((p) => typeof p === 'string') };
    }
  } catch {
    /* ignore */
  }
  return null;
}

/** Lấy tên file/thư mục cuối từ path. */
export function baseName(path: string): string {
  const trimmed = path.endsWith('/') ? path.slice(0, -1) : path;
  const idx = trimmed.lastIndexOf('/');
  return idx >= 0 ? trimmed.slice(idx + 1) : trimmed;
}

export function joinPath(dir: string, name: string): string {
  return dir.endsWith('/') ? dir + name : dir + '/' + name;
}

export function generateUniqueName(name: string, existingNames: string[]): string {
  if (!existingNames.includes(name)) return name;
  const match = name.match(/^(.*)\.([^.]+)$/);
  const base = match ? match[1] : name;
  const ext = match ? `.${match[2]}` : '';
  let i = 1;
  while (existingNames.includes(`${base} (${i})${ext}`)) {
    i++;
  }
  return `${base} (${i})${ext}`;
}

/** Bắt đầu drag-out ra hệ điều hành cho các tệp Local */
export async function startOSDrag(paths: string[]): Promise<void> {
  try {
    const localPaths = paths
      .filter((p) => p.startsWith('Local::'))
      .map((p) => p.replace(/^Local::/, ''));

    if (localPaths.length === 0) return;

    const { startDrag } = await import('@crabnebula/tauri-plugin-drag');
    await startDrag({
      item: localPaths,
      icon: ''
    });
  } catch (e) {
    console.error('OS Drag out failed', e);
  }
}
