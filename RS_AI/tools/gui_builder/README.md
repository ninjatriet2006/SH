# GUI BUILDER

TUI chọn và build project trong workspace RS_AI, rồi xuất binary vào `release/<bin>/`.

## Chạy

```sh
./build_release.sh          # từ workspace root — tự dựng builder nếu cần
```

Mở bằng cách nào cũng được: nếu không có TTY (double-click từ file manager),
builder tự mở lại chính nó trong một terminal emulator.

## Phím

| Phím    | Tác dụng                    |
|---------|-----------------------------|
| `↑` `↓` | Di chuyển (cuộn vòng)        |
| `Space` | Chọn / bỏ chọn               |
| `a`     | Chọn tất cả / bỏ tất cả      |
| `Enter` | Build (chưa tick → build dòng đang trỏ) |
| `Esc`   | Quay lại danh sách (sau khi build xong) |
| `q`     | Thoát                        |

## Danh sách project

Phát hiện động qua `cargo metadata` — thêm project vào `Cargo.toml` của workspace
là tự xuất hiện, không cần sửa code. Chỉ nhận package có target kiểu `bin`.

Phân loại tự động:

- **[Tauri]** — có build-dependency `tauri-build` *và* `tauri.conf.json`.
  Build bằng `cargo tauri build --no-bundle` để `beforeBuildCommand` dựng frontend
  và nhúng `dist/` vào binary.
- **[Cargo]** — `cargo build --release -p <package>`.

Phân biệt này là bắt buộc: build app Tauri bằng `cargo build` tạo binary rơi về
`devUrl`, app mở lên báo *connection refused*.

## Tiến trình

Hai thanh gauge:

1. **Tổng tiến trình** — số project đã xong / tổng số đã chọn.
2. **cargo build** — số crate đã biên dịch / tổng số crate (lấy từ `cargo metadata`),
   kèm tên crate đang compile.

Log hiển thị output cargo/npm theo dòng, tô màu theo mức độ (lỗi đỏ, cảnh báo vàng,
mốc xanh). Kết quả từng project hiện ở khối "Kết quả" và in lại ra terminal sau khi thoát.

## Kiểm thử

```sh
cargo test -p gui_builder              # 13 test đơn vị
cargo test -p gui_builder -- --ignored # thêm test build thật (~15s)
```
