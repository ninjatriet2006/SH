import re

with open("frontend/src/features/clipboard.ts", "r") as f:
    content = f.read()

# Replace setClipboard
old_set = """export function setClipboard(mode: ClipboardMode, items: ClipboardItem[]): void {
  state = { mode, items };

  // Đồng bộ với clipboard của Hệ điều hành (OS) nếu tất cả các file đều ở Local
  if (items.length > 0 && items.every(i => i.pane === 'left')) {
    const paths = items.map(i => i.path);
    invoke('os_clipboard_set', { paths, isCut: mode === 'cut' }).catch(err => {
      console.warn('Failed to set OS clipboard:', err);
    });
  }
}"""

new_set = """export function setClipboard(mode: ClipboardMode, items: ClipboardItem[]): void {
  state = { mode, items };

  if (items.length > 0) {
    invoke('os_clipboard_set', { items, isCut: mode === 'cut' }).catch(err => {
      console.warn('Failed to set OS clipboard:', err);
    });
  }
}"""

content = content.replace(old_set, new_set)

# Replace syncFromOSClipboard
old_sync = """interface OSClipboardData {
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
}"""

new_sync = """interface OSClipboardData {
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
}"""

content = content.replace(old_sync, new_sync)

with open("frontend/src/features/clipboard.ts", "w") as f:
    f.write(content)
print("Success")
