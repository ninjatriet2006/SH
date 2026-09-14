/*
[INTEGRITY NOTES]
 - Mục đích: Bộ kéo ngang dùng chung cho mọi chỗ cần resize (cột bảng, sidebar).
- Trách nhiệm: Gắn mousemove/mouseup tạm trong lúc kéo và tháo sạch khi thả;
   đặt con trỏ col-resize + khoá select text toàn trang để kéo mượt không bôi
   nhầm nội dung. Không biết gì về phần tử được resize — bên gọi tự tính toán.
- Tương tác: `hooks/useColumnWidths.ts`, `App.tsx` (sidebar).
*/

/** Bắt đầu kéo ngang; `onResize` nhận clientX hiện tại mỗi nhích chuột. */
export function startHorizontalDrag(
    onResize: (clientX: number) => void,
    onEnd?: () => void,
): void {
    const onMove = (ev: MouseEvent) => onResize(ev.clientX);
    const onUp = () => {
        window.removeEventListener('mousemove', onMove);
        window.removeEventListener('mouseup', onUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
        onEnd?.();
    };
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
}
