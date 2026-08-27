/*
[INTEGRITY NOTES]
- Mục đích: features/sort.ts — Bộ công cụ sắp xếp danh sách file (Files Array).
- Trách nhiệm: Cung cấp hàm `sortFiles` phân loại thư mục ưu tiên (luôn lên đầu), so sánh chuỗi không phân biệt hoa thường.
- Tương tác: Dùng bởi FileTable hoặc ListView khi người dùng nhấn Header để sắp xếp.
*/
import type { FileItem } from '../store.ts';

export type SortKey = 'name' | 'size' | 'date' | 'type';
export type SortDir = 'asc' | 'desc';



/** 
 * Tên hàm: sortFiles 
 * Mô tả: Sắp xếp files theo thuộc tính (key). Mặc định ưu tiên hiển thị Thư mục trước File.
 */
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
      case 'name':
      default:
        cmp = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    }
    return dir === 'desc' ? -cmp : cmp;
  });
  return sorted;
}

/** 
 * Tên hàm: typeLabel 
 * Mô tả: Sinh nhãn phân loại (Type Label) phục vụ cột Type để sắp xếp và hiển thị UI. 
 */
export function typeLabel(f: FileItem): string {
  if (f.file_type) return f.file_type;
  if (f.is_dir) return 'Folder';
  const idx = f.name.lastIndexOf('.');
  return idx > 0 && idx + 1 < f.name.length ? f.name.slice(idx + 1).toUpperCase() : '';
}