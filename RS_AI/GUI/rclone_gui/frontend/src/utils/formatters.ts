/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm tiện ích định dạng dữ liệu (Bytes, Thời gian, Đường dẫn).
- Trách nhiệm: Xử lý hiển thị an toàn, tránh NaN hoặc lỗi parse ngày tháng.
*/

/** Định dạng byte sang KB, MB, GB, TB với độ chính xác cao. */
export function formatBytes(bytes: number | null | undefined, decimals = 2): string {
  if (bytes === null || bytes === undefined || isNaN(bytes) || bytes < 0) return '0 B';
  if (bytes === 0) return '0 B';

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];

  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const safeI = Math.min(i, sizes.length - 1);
  return `${parseFloat((bytes / Math.pow(k, safeI)).toFixed(dm))} ${sizes[safeI]}`;
}

/** Định dạng ngày tháng theo định dạng địa phương. */
export function formatDate(dateStr: string | null | undefined): string {
  if (!dateStr) return '—';
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return dateStr;
    return d.toLocaleString(undefined, {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  } catch {
    return dateStr;
  }
}

/** Lấy tên tệp tin từ đường dẫn (hỗ trợ cả dấu / và \\). */
export function getFileName(path: string): string {
  if (!path) return '';
  const clean = path.replace(/[\\/]+$/, '');
  const parts = clean.split(/[\\/]/);
  return parts[parts.length - 1] || clean;
}

/** Lấy đường dẫn cha từ đường dẫn hiện tại. */
export function getParentPath(path: string): string {
  if (!path || path === '/' || path === '') return '/';
  
  // Xử lý Remote::/path
  if (path.includes('::')) {
    const [remote, relPath] = path.split('::');
    if (!relPath || relPath === '/' || relPath === '') return `${remote}::/`;
    const clean = relPath.replace(/\/+$/, '');
    const idx = clean.lastIndexOf('/');
    if (idx <= 0) return `${remote}::/`;
    return `${remote}::${clean.substring(0, idx)}`;
  }

  const clean = path.replace(/[\\/]+$/, '');
  const idx = Math.max(clean.lastIndexOf('/'), clean.lastIndexOf('\\'));
  if (idx <= 0) return '/';
  return clean.substring(0, idx);
}

/** Ghép 2 đoạn đường dẫn chuẩn hoá. */
export function joinPath(parent: string, child: string): string {
  if (!parent) return child;
  if (!child) return parent;

  if (parent.includes('::')) {
    const [remote, relPath] = parent.split('::');
    const base = relPath.endsWith('/') ? relPath : `${relPath}/`;
    return `${remote}::${base}${child.replace(/^\/+/, '')}`;
  }

  const sep = parent.includes('\\') ? '\\' : '/';
  const cleanParent = parent.endsWith(sep) ? parent : `${parent}${sep}`;
  return `${cleanParent}${child.replace(/^[\\/]+/, '')}`;
}
