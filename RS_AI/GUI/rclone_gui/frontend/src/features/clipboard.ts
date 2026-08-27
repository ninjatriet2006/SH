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
import { logActivity } from '../store';
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

  if (items.length > 0) {
    invoke('os_clipboard_set', { items, isCut: mode === 'cut' }).catch(err => {
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
  is_cut: boolean;
  items: { pane: string, path: string }[];
}

export async function syncFromOSClipboard(): Promise<void> {
  try {
    const data = await invoke<OSClipboardData | null>('os_clipboard_get');
    if (data && data.items && data.items.length > 0) {
      const mode: ClipboardMode = data.is_cut ? 'cut' : 'copy';
      const items: ClipboardItem[] = data.items.map(i => ({
        pane: i.pane as import('../services/explorerStore').Pane,
        path: i.path
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
    
    // Hiện thông báo đang kiểm tra xung đột đệ quy
    const checkingModal = new OperationModal('Đang kiểm tra...', '<p>Đang quét sâu để tìm các tệp tin trùng lặp...</p>');
    checkingModal.open();
    checkingModal.getElement().querySelector('.confirm')?.remove();
    checkingModal.getElement().querySelector('.cancel')?.remove();

    try {
      const conflictPaths: string[] = await invoke('fs_check_conflicts', { srcs, destPath });
      checkingModal.close();

      let applyToAllRes: ConflictResult | null = null;
      let excludes: string[] = [];
      let explicitCopies: { src: string, dest: string }[] = [];

      // 2. Xử lý từng conflict
      for (const conflictPath of conflictPaths) {
        let action: 'replace' | 'skip' | 'keep_both' = 'replace';

        if (applyToAllRes) {
          action = applyToAllRes.resolution;
        } else {
          const conflictModal = new ConflictModal(conflictPath, conflictPaths.length > 1);
          conflictModal.open();
          const res = await conflictModal.waitForResolution();
          action = res.resolution;
          if (res.applyToAll) applyToAllRes = res;
        }

        if (action === 'skip') {
          excludes.push(conflictPath);
        } else if (action === 'keep_both') {
          excludes.push(conflictPath); // Không ghi đè bản cũ
          
          // Tạo một bản copy thứ hai bằng cách thêm (1)
          const dotIdx = conflictPath.lastIndexOf('.');
          const baseName = dotIdx > 0 ? conflictPath.substring(0, dotIdx) : conflictPath;
          const ext = dotIdx > 0 ? conflictPath.substring(dotIdx) : '';
          const newName = `${baseName} (1)${ext}`;
          
          // Đường dẫn nguồn đầy đủ của file này
          // conflictPath trả về là tương đối so với src (ví dụ thư mục `Docs/file.txt`)
          // Ta cần map lại nó với src ban đầu tương ứng
          let matchingSrc = srcs.find(s => {
              const srcBase = s.endsWith('/') ? s.slice(0, -1).split('/').pop() : s.split('/').pop();
              return conflictPath.startsWith(srcBase + '/') || conflictPath === srcBase;
          });
          
          if (matchingSrc) {
              const parentDir = matchingSrc.substring(0, matchingSrc.lastIndexOf('/'));
              explicitCopies.push({
                  src: `${parentDir}/${conflictPath}`,
                  dest: joinPath(destPath, newName)
              });
          }
        }
      }

      // 3. Thực thi copy hàng loạt với mảng excludes
      for (const src of srcs) {
         await transferManager.enqueue(mode === 'cut' ? 'move' : 'copy', baseName(src), src, joinPath(destPath, baseName(src)), undefined, false, excludes.length > 0 ? excludes : undefined);
      }

      // 4. Thực thi copyto từng file cho các file keep_both
      for (const item of explicitCopies) {
         // Sử dụng lệnh copy đặc biệt cho file (rclone copyto / copy)
         await transferManager.enqueue('copy', `[Keep Both] ${baseName(item.src)}`, item.src, item.dest);
      }
      
      logActivity(actionText, `${srcs.length} mục tới ${destPath} (Xung đột: ${conflictPaths.length})`);
      
      // Nếu thao tác là Cắt (Cut), tự động xóa bộ nhớ tạm.
      if (mode === 'cut') clearClipboard();
      await onRefresh(destPane, destPath);
    } catch (e) {
      checkingModal.close();
      console.warn('paste fail:', e);
      logActivity(`Lỗi ${actionText}`, `Chi tiết: ${e}`);
      alert(`Đã xảy ra lỗi khi ${actionText}: ${e}`);
    }
  });
}