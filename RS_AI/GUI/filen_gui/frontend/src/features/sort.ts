// features/sort.ts — Sắp xếp danh sách file.
// Thư mục luôn lên đầu (bất kể hướng sort); so sánh tên case-insensitive.
import type { FileItem } from '../store';

export type SortKey = 'name' | 'size' | 'date' | 'type' | 'permissions' | 'owner' | 'group';
export type SortDir = 'asc' | 'desc';

/** Sắp xếp files theo key; dirs luôn đứng trước files. Trả về mảng mới. */
export function sortFiles(
  files: FileItem[],
  key: SortKey = 'name',
  dirsFirst = true,
  dir: SortDir = 'asc',
): FileItem[] {
  const sorted = [...files];
  sorted.sort((a, b) => {
    if (dirsFirst && a.is_dir !== b.is_dir) {
      return a.is_dir ? -1 : 1;
    }
    let cmp: number;
    switch (key) {
      case 'size':
        cmp = a.size - b.size;
        break;
      case 'date':
        cmp = new Date(a.mod_time).getTime() - new Date(b.mod_time).getTime();
        break;
      case 'type':
        cmp = typeLabel(a).localeCompare(typeLabel(b), undefined, { sensitivity: 'base' });
        break;
      case 'permissions':
        cmp = (a.permissions || '').localeCompare(b.permissions || '');
        break;
      case 'owner':
        cmp = (a.owner || '').localeCompare(b.owner || '');
        break;
      case 'group':
        cmp = (a.group || '').localeCompare(b.group || '');
        break;
      case 'name':
      default:
        cmp = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    }
    return dir === 'desc' ? -cmp : cmp;
  });
  return sorted;
}

/** Nhãn cột Type dùng để sort — khớp với giá trị hiển thị trong FileTable. */
export function typeLabel(f: FileItem): string {
  if (f.file_type) return f.file_type;
  if (f.is_dir) return 'Folder';
  const idx = f.name.lastIndexOf('.');
  return idx > 0 && idx + 1 < f.name.length ? f.name.slice(idx + 1).toUpperCase() : '';
}