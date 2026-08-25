// features/clipboard.ts — Clipboard nội bộ cho copy/cut/paste giữa các pane.
//
// Chỉ lưu state (mode + danh sách item). Paste logic nằm ở `pasteTo` — gọi
// fs_cp_batch (copy local), fs_mv_local (cut local) hoặc fs_upload (cloud).
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

  // Sync to OS clipboard if items are all local
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
        pane: 'left', // OS clipboard files are always local
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
 * Paste clipboard vào pane đích.
 * - Đích local (left): copy → fs_cp_batch; cut → fs_mv_local từng item.
 * - Đích cloud (right): fs_upload từng item.
 * Sau khi xong gọi onRefresh để cập nhật danh sách pane đích.
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
  const modal = new OperationModal(
    'Xác nhận Paste',
    `<p>Bạn có chắc muốn ${actionText} ${items.length} mục vào <br><strong>${destPath}</strong>?</p>`
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
           destNames.push(targetName); // Update local memory for subsequent conflicts
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
             await transferManager.enqueue('copy', baseName(item.src), item.src, item.dest, true, true);
          }
        } else {
          for (const item of finalSrcsToProcess) {
            await transferManager.enqueue('move', baseName(item.src), item.src, item.dest, true, true);
          }
        }
      } else {
        for (const item of finalSrcsToProcess) {
          await transferManager.enqueue('upload', baseName(item.src), item.src, item.dest, true, false);
        }
      }
      
      logActivity(actionText, `${finalSrcsToProcess.length} mục tới ${destPath}`);
      
      // Cut xong → xoá clipboard (copy giữ lại để paste nhiều lần như Nemo).
      if (mode === 'cut') clearClipboard();
      await onRefresh(destPane, destPath);
    } catch (e) {
      console.warn('paste fail:', e);
      logActivity(`Lỗi ${actionText}`, `Chi tiết: ${e}`);
    }
  });
}