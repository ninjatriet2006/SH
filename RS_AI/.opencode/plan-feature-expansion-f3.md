# Feature Phase F3 — OpenCode terminal/web controls

| id | công việc | phụ thuộc | agent | trạng thái |
|---|---|---|---|---|
| F3.1 | Khóa commands/process/terminal contract | F2 | explorer | [x] |
| F3.2 | Cài web child lifecycle | F3.1 | rust-dev | [x] |
| F3.3 | Readiness timeout/unexpected exit | F3.2 | rust-dev | [x] |
| F3.4 | Bảo vệ loopback/URL/process args | F3.1 | rust-dev | [x] |
| F3.5 | Stop ownership/shutdown cleanup | F3.2 | rust-dev | [x] |
| F3.6 | Mở OpenCode terminal độc lập | F3.1,F3.4 | rust-dev | [x] |
| F3.7 | Đăng ký API/DTO/service/UI state | F3.1-F3.6 | rust-dev | [x] |
| F3.8 | Start/Stop và disabled states | F3.2-F3.5,F3.7 | rust-dev | [x] |
| F3.9 | Copy/Open localhost URL | F3.3,F3.4,F3.7 | rust-dev | [x] |
| F3.10 | Thêm EN/VI app-local | F3.7-F3.9 | rust-dev | [x] |
| F3.11 | Test backend lifecycle/security/API | F3.2-F3.7 | tester | [x] |
| F3.12 | Test UI actions/disabled/i18n | F3.8-F3.10 | tester | [x] |

Acceptance kế thừa roadmap. Terminal action chạy `opencode` trong terminal nhìn thấy được và độc lập
với web child. Web chạy ẩn, loopback-only, state machine đầy đủ, start idempotent, stop chỉ child sở hữu,
cleanup lúc app đóng, link bên trái và controls bên phải với spacing rộng. Copy/Open chỉ dùng URL localhost
đã validate khi running; transitional/error states không cho thao tác xung đột.

F3.1 deliverable là command matrix kèm output probe `opencode --version`, `opencode --help` và
web-subcommand help: exact binary/subcommand/args theo version, inherited env, CWD-neutral launch và
typed fallback khi CLI không hỗ trợ web. F3.2/F3.4 không bắt đầu trước khi evidence này có. Terminal
emulator chọn theo OS/available list, thiếu executable/emulator trả typed error.
API bắt buộc gồm status/start/stop/launch_terminal. `WebState` enum snake_case; response
`{state:WebState,url:string|null,error:string|null,generation:u64,revision:u64}` với field nullable luôn
hiện diện. Generation tăng mỗi child, revision tăng mỗi transition; frontend serialize mutations và bỏ
mọi poll/response có `(generation,revision)` cũ.
Readiness bắt buộc HTTP GET loopback thành công và child còn sống, có timeout; tests cover start race/duplicate, early exit, timeout,
restart from error, PID ownership, stop timeout/escalation và shutdown không kill process ngoài sở hữu.
Lifecycle serialize bằng mutex/state machine, child handle + generation token; race matrix thêm
stop-during-start, shutdown-during-start và late-readiness từ generation cũ sau restart.
F3.6 test terminal không đọc/chia sẻ web child. F3.12 cover layout link-left/control-right/spacing,
disabled matrix mọi state, invalid URL, copy/open failures, EN/VI và chỉ open localhost khi running.
F3.11/F3.12 dùng mocked command matrix theo version/help/emulator và platform smoke để chứng minh terminal
visible, web child hidden, fallback typed khi version/subcommand/emulator không hỗ trợ.

Kết quả: Reviewer APPROVE; backend 75/75 và frontend 8/8 tests PASS; fmt/check/clippy sạch,
npm build PASS. Linux command matrix/smoke đã ghi evidence; macOS/Windows runtime platform-unverified.
