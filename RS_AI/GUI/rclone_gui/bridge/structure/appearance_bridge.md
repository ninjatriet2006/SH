[Pattern Docs]
# appearance_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri tuỳ biến giao diện, font chữ và ngôn ngữ i18n.

- **Tên hàm**: `getAvailableLangs`
- **Mô tả**: Quét các mã ngôn ngữ có file dịch (.json).
- **Đầu ra**: `Promise<string[]>`

- **Tên hàm**: `getLangContent`
- **Mô tả**: Tải toàn bộ nội dung từ điển dịch của một ngôn ngữ.
- **Tham số đầu vào**: `langCode: string`
- **Đầu ra**: `Promise<Record<string, JsonValue>>`

- **Tên hàm**: `getAvailableThemes` / `getAvailableFonts`
- **Mô tả**: Quét các chủ đề màu và phông chữ cài đặt trong hệ thống.
- **Đầu ra**: `Promise<ThemeInfo[]>` / `Promise<FontInfo[]>`

- **Tên hàm**: `getAppearance` / `setAppearance`
- **Mô tả**: Đọc hoặc lưu cấu hình giao diện người dùng.
- **Đầu ra**: `Promise<AppearanceSettings>`
