/*
[INTEGRITY NOTES]
- Mục đích: features/clipboard.ts — Clipboard nội bộ phục vụ tính năng Copy/Cut/Paste giữa hai khung (Dual Pane).
- Trách nhiệm: Chỉ lưu trữ trạng thái (chế độ copy/cut và danh sách đường dẫn). Giao tiếp với clipboard của hệ điều hành OS qua Tauri `invoke`.
- Tương tác: Tính năng dán thực tế (Paste) gọi hàm `pasteTo`, xử lý xung đột tệp tin, và chuyển xuống `transferManager` để tải lên/copy.
*/
import { invoke } from '@tauri-apps/api/core';
import type { Pane } from '../services/explorerStore';
// Removed fileOps import
import { OperationModal } from '../components/OperationModal';
import { logActivity, appState } from '../store';
import { ConflictModal, type ConflictResult } from '../components/ConflictModal';
import { transferManager } from './transferManager';

export type ClipboardMode = 'copy' | 'cut';

export interface ClipboardItem {
  pane: Pane;
  path: string;
}

export interface ClipboardState {
  mode: ClipboardMode;
  items: ClipboardItem[];
}

let state: ClipboardState = { mode: 'copy', items: [] };

export function setClipboard(mode: ClipboardMode, items: ClipboardItem[]): void {
  state = { mode, items };

  // Đồng bộ với clipboard của Hệ điều hành (OS) nếu tất cả các file đều ở Local
  if (items.length > 0 && items.every(i => i.pane === 'left')) {
    const paths = items.map(i => i.path);
    invoke('os_clipboard_set', { paths, isCut: mode === 'cut' }).catch(err => {
      console.warn('Failed to set OS clipboard:', err);
    });
  }
}

export function getClipboard(): ClipboardState {
  return state;
}

export function hasClipboard(): boolean {
  return state.items.length > 0;
}

export function clearClipboard(): void {
  state = { mode: 'copy', items: [] };
}

interface OSClipboardData {
  mode: string;
  paths: string[];
}

export async function syncFromOSClipboard(): Promise<void> {
  try {
    const data = await invoke<OSClipboardData | null>('os_clipboard_get');
    if (data && data.paths && data.paths.length > 0) {
      const mode: ClipboardMode = data.mode === 'cut' ? 'cut' : 'copy';
      const items: ClipboardItem[] = data.paths.map(p => ({
        pane: 'left', // Các file lấy từ clipboard OS mặc định luôn là Local (left pane)
        path: p
      }));
      state = { mode, items };
    }
  } catch (e) {
    console.warn('Failed to get OS clipboard:', e);
  }
}

/** Lấy tên file/thư mục cuối từ path. */
function baseName(path: string): string {
  const trimmed = path.endsWith('/') ? path.slice(0, -1) : path;
  const idx = trimmed.lastIndexOf('/');
  return idx >= 0 ? trimmed.slice(idx + 1) : trimmed;
}

function joinPath(dir: string, name: string): string {
  return dir.endsWith('/') ? dir + name : dir + '/' + name;
}

function generateUniqueName(name: string, existingNames: string[]): string {
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

/**
 * Tên hàm: pasteTo
 * Mô tả: Thực thi thao tác Dán (Paste) clipboard hiện tại vào thư mục đích.
 * - Hiển thị cảnh báo xung đột (Conflict) nếu trùng tên.
 * - Đẩy danh sách cuối cùng vào TransferManager (hàng đợi tiến trình).
 * Tham số đầu vào: 
 *   - destPane (Bắt buộc), destPath (Bắt buộc): Thông tin đích
 *   - onRefresh (Bắt buộc): Hàm callback tải lại UI sau khi xong
 */
export async function pasteTo(
  destPane: Pane,
  destPath: string,
  onRefresh: (pane: Pane, path: string) => Promise<void>,
): Promise<void> {
  await syncFromOSClipboard();
  if (!hasClipboard()) return;
  const { mode, items } = getClipboard();
  const srcs = items.map((i) => i.path);
  
  const actionText = mode === 'copy' ? 'Sao chép' : 'Di chuyển';
  
  let targetNamesStr = '';
  if (items.length === 1) {
    targetNamesStr = `<strong>${baseName(items[0].path)}</strong>`;
  } else if (items.length <= 3) {
    targetNamesStr = `<strong>${items.map(i => baseName(i.path)).join(', ')}</strong>`;
  } else {
    targetNamesStr = `<strong>${items.length} mục</strong> (gồm ${baseName(items[0].path)}...)`;
  }

  const modal = new OperationModal(
    'Xác nhận Paste',
    `<p>Bạn có chắc muốn ${actionText} ${targetNamesStr} vào <br><strong>${destPath}</strong>?</p>`
  );
  modal.open();

  modal.getElement().querySelector('.confirm')?.addEventListener('click', async () => {
    modal.close();
    try {
      const destFiles = destPane === 'left' ? appState.explorer?.leftFiles || [] : appState.explorer?.rightFiles || [];
      const destNames = destFiles.map(f => f.name);
      
      let applyToAllRes: ConflictResult | null = null;
      let finalSrcsToProcess: { src: string, dest: string, action: 'replace' | 'skip' | 'keep_both' }[] = [];

      for (const src of srcs) {
        const name = baseName(src);
        const existing = destNames.includes(name);
        let action: 'replace' | 'skip' | 'keep_both' = 'replace';

        if (existing) {
          if (applyToAllRes) {
            action = applyToAllRes.resolution;
          } else {
            const conflictModal = new ConflictModal(name, srcs.length > 1);
            conflictModal.open();
            const res = await conflictModal.waitForResolution();
            action = res.resolution;
            if (res.applyToAll) applyToAllRes = res;
          }
        }
        
        if (action === 'skip') continue;

        let targetName = name;
        if (action === 'keep_both') {
           targetName = generateUniqueName(name, destNames);
           destNames.push(targetName); // Cập nhật danh sách nội bộ để tránh trùng lặp tiếp theo
        }

        finalSrcsToProcess.push({ src, dest: joinPath(destPath, targetName), action });
      }

      if (finalSrcsToProcess.length === 0) {
        logActivity('Bỏ qua', 'Tất cả các mục đã bị bỏ qua');
        return;
      }

      if (destPane === 'left') {
        if (mode === 'copy') {
          for (const item of finalSrcsToProcess) {
             await transferManager.enqueue('copy', baseName(item.src), item.src, item.dest);
          }
        } else {
          for (const item of finalSrcsToProcess) {
            await transferManager.enqueue('move', baseName(item.src), item.src, item.dest);
          }
        }
      } else {
        if (mode === 'copy') {
          for (const item of finalSrcsToProcess) {
            await transferManager.enqueue('copy', baseName(item.src), item.src, item.dest);
          }
        } else {
          for (const item of finalSrcsToProcess) {
            await transferManager.enqueue('move', baseName(item.src), item.src, item.dest);
          }
        }
      }
      
      logActivity(actionText, `${finalSrcsToProcess.length} mục tới ${destPath}`);
      
      // Nếu thao tác là Cắt (Cut), tự động xóa bộ nhớ tạm. Giữ lại nếu là Copy để có thể dán nhiều lần.
      if (mode === 'cut') clearClipboard();
      await onRefresh(destPane, destPath);
    } catch (e) {
      console.warn('paste fail:', e);
      logActivity(`Lỗi ${actionText}`, `Chi tiết: ${e}`);
    }
  });
}