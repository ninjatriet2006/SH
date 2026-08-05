# Dead-Model Registry

Registry ghi nhận các model bị coi là "chết" (không phản hồi / lỗi / timeout) để Lead né khi giao việc.

## Quy tắc sử dụng (Lead)
- **Ghi**: Khi agent fail do model (timeout, lỗi provider, không phản hồi) → thêm 1 dòng `DEAD` (hoặc tăng `fail_count` nếu đã có).
- **Né**: Trước Bước 4 (giao việc), đọc file này; model nào ở trạng thái `DEAD` → KHÔNG giao task cho agent dùng model đó.
- **Gỡ**: Khi model hồi phục (test thành công hoặc user xác nhận) → sửa trạng thái thành `RECOVERED` (giữ 1 dòng lịch sử) hoặc xóa hẳn.
- **Ngưỡng**: 2 lần fail liên tiếp trong cùng phiên → `DEAD`. Dưới ngưỡng → ghi `WATCH` (theo dõi, vẫn dùng được).
- Không tự ghi nếu chỉ là lỗi 1 lần do mạng thoáng qua — ghi `WATCH` trước.

## Registry (mới nhất ở trên)

| model | trạng thái | fail_count | phát hiện lúc | triệu chứng | ghi chú |
|-------|-----------|------------|---------------|-------------|---------|
| _(trống — chưa có model chết)_ | | | | | |

## Lịch sử đã hồi phục
| model | trạng thái | hồi phục lúc | ghi chú |
|-------|-----------|--------------|---------|
| _(trống)_ | | | |
