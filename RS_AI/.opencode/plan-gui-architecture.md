# Roadmap — Bảy GUI Tauri v2 độc lập

## Mục tiêu
- Chuẩn hóa 7 dự án trong `RS_AI/GUI` thành ứng dụng Tauri v2 với ba lớp vật lý
  `frontend/`, `backend/`, `bridge/`.
- Mỗi dự án phải có tài nguyên runtime đầy đủ tại `langs/`, `themes/`, `fonts/`.
- Migrate ba ứng dụng egui sang Tauri v2, giữ nguyên hành vi, persistence và native integration.
- Mỗi GUI là project độc lập: không dùng chung component/package/path dependency/symlink/runtime
  asset với GUI khác; phần giống nhau phải được sao chép vật lý và sở hữu riêng trong từng project.
- Verify bằng test/build phù hợp, `cargo check`, `cargo test`, `cargo clippy` và cross-check độc lập.

## Phạm vi
`filen_gui`, `rclone_gui`, `opencode_manager_gui`, `subscription_manager_gui`,
`universal_converter_gui`, `img_splt_gui`, `universe_manager_gui`.

## Hiện trạng sơ bộ
- Đã là Tauri: Filen, Rclone, OpenCode Manager, Subscription Manager.
- Đang là egui và phải migrate: Universal Converter, IMG_SPLT, Universe Manager.
- Cần audit nội dung/ranh giới dependency và tính đầy đủ của resource ở cả 7 dự án.

## Tiêu chí kiến trúc
- `frontend/`: web presentation, view/component, interaction và frontend entrypoint.
- `backend/`: Rust domain/application logic, persistence, filesystem/process/network adapters.
- `bridge/`: Tauri composition root, commands/events, DTO mapping, permissions/capabilities;
  không chứa business logic.
- `langs/`, `themes/`, `fonts/`: có asset mặc định hợp lệ, được load runtime và được đóng gói.
- Cấm mọi GUI phụ thuộc source/runtime asset của GUI khác. Backend có thể tiếp tục dùng TUI domain
  crate hiện hữu theo phạm vi nghiệp vụ; lệnh audit phải chứng minh không coupling GUI-to-GUI.

## Roadmap
| phase | mô tả | phụ thuộc | ưu tiên | trạng thái |
|---|---|---|---|---|
| 1 | Chốt baseline parity, persistence, artifact bảy GUI | - | 1 | [x] |
| 2 | Chốt layer, IPC, capabilities/permissions Tauri v2 | 1 | 1 | [x] |
| 3 | Nhân bản assets/components, cấm dependency chéo GUI | 1,2 | 1 | [~] |
| 4 | Chuẩn hóa bốn GUI Tauri hiện hữu | 2,3 | 1 | [x] |
| 5 | Migrate `universal_converter_gui`, checkpoint và rollback | 4 | 1 | [x] |
| 6 | Migrate `img_splt_gui`, checkpoint và rollback | 5 | 1 | [ ] |
| 7 | Migrate `universe_manager_gui`, checkpoint và rollback | 6 | 1 | [ ] |
| 8 | Audit package/workspace/path/symlink/runtime coupling | 4,7 | 1 | [ ] |
| 9 | Nghiệm thu bảy standalone artifacts | 8 | 1 | [ ] |

## Gates và acceptance
- Gate 1: baseline hành vi, persistence, build/package và test hiện có của từng app được ghi nhận.
- Gate 2: ba layer là thư mục vật lý; bridge Tauri là composition root duy nhất, chỉ wiring/IPC/DTO;
  frontend chỉ invoke/listen bridge, bridge gọi backend; cấm frontend gọi backend trực tiếp.
- Mỗi command/event có DTO, error contract và capability/permission tối thiểu; không bật quyền rộng
  ngoài nhu cầu. Identifier, product name, output và updater metadata phải riêng từng app.
- Gate 3-6: từng app load được language/theme/font runtime, có fallback mặc định, lưu lựa chọn,
  package đủ assets, mọi key UI có bản dịch, theme/font thật sự đổi UI, và asset lỗi/thiếu phải
  fallback an toàn; không chuyển sang app kế tiếp khi app hiện tại chưa build/test đạt.
- Bốn app ở Phase 4 được triển khai song song, nhưng mỗi app có gate độc lập và không được đánh
  dấu hoàn tất nếu chưa qua toàn bộ acceptance của chính app đó.
- Gate 7: từng app pass test/build/check/clippy phù hợp và regression từ artifact/package.
- Artifact smoke phải chạy từ thư mục tạm nằm ngoài source tree và với CWD khác repo, sau đó
  xác nhận loader đọc đúng `langs/`, `themes/`, `fonts/` cạnh executable/resource bundle.
- Gate 5-7: trước mỗi migration phải checkpoint mapping egui state/storage/window/native behavior
  sang Tauri và cách rollback; rollback drill phải khôi phục được source checkpoint và baseline
  test/build trong worktree tạm, lưu log bằng chứng; app phải pass trước khi bắt đầu app kế tiếp.
- Gate 8: không có import, package, workspace member coupling, path dependency, symlink hay resource
  lookup trỏ sang GUI khác. Chỉ chấp nhận file component/asset được copy vật lý vào project sở hữu.
- Audit tái lập dùng search Cargo/package manifests, TS imports, symlink inventory và resource path;
  phạm vi toàn bộ `RS_AI/GUI`, output lưu `.opencode/gui-standalone-audit.md`.
- Lệnh audit chuẩn được checkpoint ở `.opencode/audit-gui-standalone.sh`; false positive chỉ được
  allowlist khi là dependency tới `RS_AI/TUI`, toolchain/cache/build output, hoặc chuỗi tài liệu không
  được load runtime. Mỗi allowlist phải ghi path, match và lý do trong report.
- Gate 9: chỉ chạy sau khi cả bảy app qua Gate 8; Cross Checker độc lập xác nhận yêu cầu gốc.

## Bằng chứng
- Baseline/audit: `.opencode/plan-gui-architecture-phase-1.md`.
- Contract: `.opencode/plan-gui-architecture-phase-2.md`.
- Mỗi phase migration có checkpoint riêng và log test/package; chưa có evidence thì checkbox giữ `[ ]`.
- Final standalone audit: `.opencode/gui-standalone-audit.md`.
- Evidence mỗi app: `.opencode/evidence/gui/<app>/{baseline,contract,migration,verify,package,smoke,rollback}.md`.

## Deliverables theo app
Mỗi checkbox cần log command/output, bundle manifest và smoke ngoài source tree.

| app | package command/output | base | Tauri v2 | physical layers | IPC/cap | parity/persist | resources | standalone | smoke |
|---|---|---|---|---|---|---|---|---|---|
| filen_gui | `bridge: cargo tauri build`; `bridge/target/release/bundle` | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] |
| rclone_gui | `backend: cargo tauri build`; workspace `target/release/bundle` | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] |
| opencode_manager_gui | `backend: cargo tauri build`; workspace `target/release/bundle` | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] |
| subscription_manager_gui | `backend: cargo tauri build`; workspace `target/release/bundle` | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] |
| universal_converter_gui | `bridge: cargo tauri build`; workspace `target/release/bundle` | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] |
| img_splt_gui | `bridge: cargo tauri build`; workspace `target/release/bundle` | [~] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] |
| universe_manager_gui | `bridge: cargo tauri build`; `bridge/target/release/bundle` | [~] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] |

Pass/fail: build/run + app-specific tests; IPC DTO/error/capability inventory; restart persistence and
egui parity checklist; EN/VI key coverage + applied/fallback theme/font; no GUI coupling audit; bundle
contains three resource dirs and launches from external CWD. Migrated apps additionally require
state/storage/window/native mapping and rollback-drill log.
Tauri v2 pass khi Cargo/npm manifest dùng major 2 và generated context build thành công; physical layer
pass khi ba thư mục tồn tại và audit dependency direction không phát hiện frontend→backend trực tiếp.

## Trạng thái
- Audit sơ bộ: [x]
- Plan approved: [x]
- Implementation: [~] — Phase 4 và Phase 5 hoàn tất; Phase 6–7 chưa triển khai
- Validation/review/docs: [~] — Phase 5 review/docs/cross-check complete; Phase 6–7 còn chờ
