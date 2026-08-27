/*
[INTEGRITY NOTES]
- Mục đích: features/dragDrop.ts — Quản lý khối dữ liệu (Payload) thao tác kéo thả (Drag & Drop) file giữa các pane.
- Trách nhiệm: Đóng gói (serialize) và phân tích (parse) chuỗi JSON chứa thông tin kéo thả theo chuẩn HTML5 DataTransfer.
- Tương tác: Giao tiếp với UI Webview, hỗ trợ xuất Drag ra hệ điều hành (OS Drag) bằng tauri-plugin-drag.
*/
import type { Pane } from '../services/explorerStore';

export interface DragPayload {
  pane: Pane;
  paths: string[];
}

export function serializeDrag(p: DragPayload): string {
  return JSON.stringify(p);
}

/** Tên hàm: parseDrag | Mô tả: Phân tích payload JSON từ dataTransfer; trả về null nếu kéo từ ngoài app. */
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

/** Tên hàm: startOSDrag | Mô tả: Kích hoạt kéo thả các tệp tin Local ra màn hình hệ điều hành (Desktop OS) */
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
