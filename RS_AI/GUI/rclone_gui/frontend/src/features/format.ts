/*
[INTEGRITY NOTES]
- Mục đích: features/format.ts — Cung cấp các hàm định dạng hiển thị (dung lượng, ngày giờ).
- Trách nhiệm: Xử lý hiển thị thô thành chuỗi thân thiện với người dùng (ví dụ: bytes -> MB, ISO string -> dd/mm/yyyy).
- Tương tác: Sử dụng trong tất cả các lưới dữ liệu (DataGrid/Table) và UI hiển thị. Test độc lập.
*/

/** Tên hàm: formatSize | Mô tả: Định dạng số byte thành chuỗi hiển thị tương ứng B/KB/MB/GB. */
export function formatSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '-';
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes;
  let unit = 'B';
  for (const u of units) {
    value /= 1024;
    unit = u;
    if (value < 1024) break;
  }
  return `${value.toFixed(1)} ${unit}`;
}

/** Tên hàm: formatDate | Mô tả: Định dạng chuỗi ISO nguyên gốc sang dạng dd/mm/yyyy hh:mm. */
export function formatDate(iso: string): string {
  if (!iso) return '-';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/**
 * Tên hàm: escapeHtml
 * Mô tả: Thoát các ký tự đặc biệt của HTML để chuỗi dữ liệu ngoài (tên file, owner,
 * group, mod_time từ remote) không thể chèn thẻ/script khi ghép vào innerHTML.
 */
export function escapeHtml(value: unknown): string {
  if (value === null || value === undefined) return '';
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
