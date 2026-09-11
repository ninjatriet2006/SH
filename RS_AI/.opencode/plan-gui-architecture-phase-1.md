# Phase 1 — Baseline và acceptance

| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|---|---|---|---|---|---|
| 1.1 | Lập ma trận bảy app và tiêu chí | - | 1 | explorer | [x] |
| 1.2 | Chốt acceptance hành vi từng app | 1.1 | 1 | tester | [x] |
| 1.3 | Chốt acceptance persistence từng app | 1.1 | 1 | explorer | [x] |
| 1.4 | Chốt acceptance packaging từng app | 1.1 | 1 | explorer | [x] |
| 1.5 | Audit physical layers và dependency violations | 1.1 | 1 | explorer | [x] |
| 1.6 | Audit resource completeness và runtime loading | 1.1 | 1 | explorer | [x] |
| 1.7 | Kiểm kê commands/tests/build/package hiện có | 1.1 | 1 | tester | [x] |
| 1.8 | Đánh giá migration risks từng app | 1.2-1.7 | 2 | reviewer | [x] |
| 1.9 | Tổng hợp checkpoint baseline Phase 1 | 1.8 | 2 | docs | [x] |

## Baseline matrix
| app | hiện trạng layer | resource/persistence | rủi ro chính |
|---|---|---|---|
| filen_gui | Có 3 thư mục; bridge còn workflow | lang/theme nhúng frontend; root thiếu langs/themes; fonts README | Hai kiến trúc egui/Tauri cùng tồn tại, package contract thiếu |
| rclone_gui | 3 lớp, dependency gần đúng | langs/themes runtime, persist; fonts README | Chưa có font asset thật và artifact smoke |
| opencode_manager_gui | 3 lớp, TS bridge rõ | langs/themes runtime, persist; fonts README | Chưa có font asset thật và artifact smoke |
| subscription_manager_gui | 3 lớp, TS bridge rõ | langs/themes runtime, persist; fonts README | Mirror theo CWD; chưa artifact smoke |
| universal_converter_gui | Một `src/main.rs` trộn mọi lớp | i18n inline, font hệ thống, eframe storage | DTO JSON string, unwrap runtime/font, không package assets |
| img_splt_gui | Một `src/main.rs` trộn mọi lớp | i18n inline, font hệ thống, eframe storage | Đổi CWD process-wide; upstream prompt/exit; không package assets |
| universe_manager_gui | Một `src/main.rs` 989 dòng | i18n inline, font hệ thống, prefs file | Scan font blocking; persistence không atomic; không package assets |

## Acceptance baseline
- Hành vi: entrypoint/tab/action hiện có không đổi; async result và lỗi phải hiển thị, không panic/exit GUI.
- Persistence: language/theme/font sống qua restart; asset đã mất hoặc sai phải fallback mặc định.
- Resources: mọi UI key có EN/VI; theme và font selection áp dụng thật; cả ba thư mục được package.
- Layers: frontend chỉ phụ thuộc bridge; bridge map DTO/API; backend giữ domain, IO và persistence.
- Validation: command theo bảng roadmap; smoke artifact từ CWD ngoài repo và kiểm manifest assets.

## Kết quả kiểm thử baseline
- Rust checks và frontend builds hiện tại chạy được; 235 Rust tests pass trong lượt audit.
- Rclone frontend: 29/29 pass. Filen frontend: 34/38, bốn lỗi DOM tại `OperationModal.ts`.
- Clippy `-D warnings` chưa sạch (38 diagnostics toàn phạm vi được audit).
- Chưa có release artifact smoke; đây là acceptance bắt buộc ở Gate 7.

## Migration seams và state ownership
- `universal_converter_gui`: frontend sở hữu tab/form; bridge sở hữu typed request/result và async
  channel; backend adapter gọi scanner/dependency APIs. Loại JSON display DTO và runtime unwrap.
- `img_splt_gui`: frontend sở hữu form/status; bridge sở hữu request/result + worker; backend API phải
  nhận `&Path`, trả `Result`, tuyệt đối không `set_current_dir`, prompt/sudo hay `process::exit`.
- `universe_manager_gui`: frontend sở hữu tabs/view state; bridge worker map `Config`/`AppEntry`;
  backend sở hữu scan/start/stop/search và persistence atomic, không scan font trên UI thread.
- Mỗi app giữ `main.rs` chỉ làm composition root; checkpoint build/test đạt trước khi migrate app sau.

## Asset manifest baseline
- Không có file `.ttf/.otf/.woff/.woff2` nào trong repo; mọi `fonts/` hiện tại chỉ README hoặc thiếu.
- Filen canonical artifact là Tauri crate độc lập tại `filen_gui/bridge`, không phải root library.
- Filen thiếu root `langs/`; theme đang ở `frontend/public/themes`; bundle mới khai báo themes/fonts.
- Rclone/OpenCode/Subscription có khai báo bundle đủ ba dirs; ba egui chưa có dirs/loader/package.
- Phase implementation phải thêm font có license redistributable và file license/attribution, manifest
  liệt kê relative path + checksum; test artifact xác minh đủ langs/themes/fonts ngoài source tree.

## Baseline debt policy
- Filen frontend baseline: `npm test` pass 34/38; 4 lỗi cùng nguyên nhân DOM tại
  `frontend/src/components/OperationModal.ts:6`. Không được phát sinh failure mới; phải sửa trước Gate 7.
- Clippy strict baseline: 38 diagnostics trong lượt audit (26 workspace, 12 Filen bridge), gồm
  `backend/models.rs:209` và bridge `fs_cmds.rs:563`; Phase 7 yêu cầu không warning mới và xử lý
  diagnostics trong các file chạm tới, sau đó tiến tới `-D warnings` sạch cho bảy GUI.
