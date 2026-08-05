<!-- INTEGRITY NOTES: Core architecture spec for filen_gui v3.0.0 (framework-agnostic). -->
<!-- Purpose: Define app shell + core service layer with a FIXED boundary API so the core can be reused when the GUI framework changes. -->
<!-- Scope: operations.rs, transfer.rs (core giữ nguyên 100%), src-tauri/ + frontend/ (view/adapter). No GUI-framework code appears in the boundary contract. -->
<!-- Sub-doc #1 of docs v3.0.0 set — hub: docs/neon_ui_design.md. -->

# `filen_gui` Core Architecture (framework-agnostic)

**Version**: 3.0.0
**Status**: chốt v3.0.0 — boundary API được chốt theo code hiện tại (đã verify với `src/operations.rs`, `src/transfer.rs`); UI layer chạy trên Tauri v2
**Target**: `apps_gui/filen_gui`

---

## 1. Mục tiêu

Tách **UI layer** (framework GUI — v3.0.0 đã chốt Tauri v2) khỏi **core service layer**. Core chỉ nói chuyện với UI qua một **boundary API cố định** (trait + function signature mức khái niệm). Khi đổi framework, core giữ nguyên, chỉ viết lại view/adapter.

---

## 2. Tổng quan phân lớp

```
┌─────────────────────────────────────────────────────────┐
│  UI LAYER (framework-specific)                          │
│  - view state, rendering, input capture                 │
│  - gửi command xuống core, nhận event về từ event bus   │
└───────────────▲─────────────────────────────────────────┘
                │ Boundary API (bất biến, không gắn framework)
┌───────────────┴─────────────────────────────────────────┐
│  CORE SERVICE LAYER                                     │
│  Operations · TransferManager · ServersManager          │
│  AccountManager · AuthSession                           │
│  Event bus (message)                                    │
└───────────────▲─────────────────────────────────────────┘
                │ port/adapter
┌───────────────┴─────────────────────────────────────────┐
│  ADAPTERS: filen-cli binary · local fs · child process  │
└─────────────────────────────────────────────────────────┘
```

- **Core** không import gì từ framework GUI. Nó chỉ thao tác: `filen-cli` (qua `Command`), `std::fs`, channel/event bus.
- **UI** không gọi thẳng CLI/fs; mọi tác động đi qua boundary API.

---

## 3. Core service layer

| Service | Trách nhiệm | Nguồn code hiện tại |
|---|---|---|
| `Operations` | Ops cloud + local: list/mkdir/rm/mv/cp/statfs/link/sync/recents/write/cat… | `operations.rs` |
| `TransferManager` | Transfer queue: upload/download/copy/move, runner CLI, cancel/timeout | `transfer.rs` |
| `ServersManager` | Lifecycle server WebDAV / S3 / FUSE (start/stop, log buffer) | `operations.rs` (`WebDavServerState`, `S3ServerState`, `MountState`) |
| `AccountManager` | Persist danh sách tài khoản (`accounts.json`) | `operations.rs` (`load/save_stored_accounts`) |
| `AuthSession` | Trạng thái phiên: whoami/statfs/login/logout/2FA | `operations.rs` (`login_new`, `whoami`, `statfs`, `logout`) |

---

## 4. Boundary API (traits + function signature mức khái niệm)

> Contract: đây là chữ ký **khái niệm** — kiểu trả về dùng `Result<T, String>` / `Vec<T>` / callback; **không** phụ thuộc framework GUI. Các kiểu dữ liệu (`FileItem`, `TransferItem`, `SyncPair`, `ServerStatus`) là plain data, Clone + Debug.

### 4.1 `Operations`

```rust
pub struct FileItem { name, is_dir, size, mod_time }
pub struct SyncPair { local, remote, sync_mode, alias, … }

trait FileOps {
    async fn list_remote(account: &Option<String>, path: &str) -> Result<Vec<FileItem>, String>;
    fn list_local(path: &str) -> Result<Vec<FileItem>, String>;          // sync
    async fn mkdir(account: &Option<String>, path: &str) -> Result<(), String>;
    async fn rm(account: &Option<String>, path: &str, no_trash: bool) -> Result<(), String>;
    async fn mv(account: &Option<String>, from: &str, to: &str) -> Result<(), String>;
    async fn cp(account: &Option<String>, from: &str, to: &str) -> Result<(), String>;
    async fn cat(account: &Option<String>, path: &str) -> Result<String, String>;
    async fn create_link(account: &Option<String>, path: &str) -> Result<String, String>;
    async fn list_links(account: &Option<String>) -> Result<Vec<(String, String)>, String>;
    async fn recents(account: &Option<String>) -> Result<Vec<FileItem>, String>;
    async fn write_file(account: &Option<String>, path: &str, content: &str) -> Result<(), String>;
    async fn statfs(account: &Option<String>) -> Result<(String, String), String>;
    // sync
    async fn sync_once(account: &Option<String>, pair: &SyncPair) -> Result<(), String>;
    async fn sync_pair_once(account: &Option<String>, pair: &SyncPair) -> Result<(), String>;
    fn sync_pairs() -> Result<Vec<SyncPair>, String>;
}
```

### 4.2 `TransferManager`

```rust
enum TransferKind { Upload, Download, Copy, Move }
enum TransferStatus { Queued, Running, Done, Error, Cancelled }
struct ProgressUpdate { progress: Option<f32>, bytes_done: u64, total_bytes: u64 }

trait TransferQueue {
    fn enqueue(kind, name, src, dst, src_local, dst_local, cleanup_src, src_pane, dst_pane) -> usize;
    fn cancel(&self, id: usize);
    fn cancel_all(&self);
    fn remove_finished(&mut self);
    fn running_count(&self) -> usize;
    fn next_queued_idx(&self) -> Option<usize>;
}

// runner độc lập, chạy ngoài UI thread; gọi callback khi có tiến trình
async fn run_cli_transfer(
    kind: TransferKind, src: &str, dst: &str, timeout_secs: u64,
    cancelled: Arc<AtomicBool>,
    on_update: impl FnMut(ProgressUpdate),
) -> Result<(), TransferError>;

// local → local, không qua CLI
fn copy_local(src: &str, dst: &str) -> Result<(), String>;
fn move_local(src: &str, dst: &str) -> Result<(), String>;
fn delete_local_path(path: &str) -> Result<(), String>;
```

### 4.3 `ServersManager`

```rust
enum ServerProtocol { WebDAV, S3, FuseMount }
enum ServerStatus { Stopped, Starting, Running, Error }

trait ServerLifecycle {
    async fn start_webdav(cfg: WebDavConfig) -> Result<(), String>;       // + start_webdav_proxy
    async fn stop_webdav() -> Result<(), String>;
    async fn start_s3(cfg: S3Config) -> Result<(), String>;
    async fn stop_s3() -> Result<(), String>;
    async fn start_mount(mount_point: Option<&str>) -> Result<String, String>;
    async fn stop_mount() -> Result<(), String>;
    fn logs(&self, protocol: ServerProtocol) -> &[String];
    fn status(&self, protocol: ServerProtocol) -> ServerStatus;
}
```

### 4.4 `AccountManager`

```rust
struct StoredAccount { email, password }

trait AccountStore {
    fn load_accounts() -> Vec<StoredAccount>;
    fn save_accounts(accounts: &[StoredAccount]) -> Result<(), String>;
}
```

### 4.5 `AuthSession`

```rust
enum AuthResult { Success, TwoFARequired }

trait AuthFlow {
    async fn whoami(account: &Option<String>) -> Result<String, String>;
    async fn login(email, password, twofa_code: Option<&str>, keep_logged: &str) -> Result<AuthResult, String>;
    async fn logout(account: &Option<String>) -> Result<(), String>;
    async fn statfs(account: &Option<String>) -> Result<(String, String), String>;
}
```

---

## 5. Luồng dữ liệu async (event bus / message)

Boundary không dùng callbacks phức tạp cho lệnh ngoài transfer. Chuẩn hoá:

```
UI ──command──▶ Core (async worker, không block UI)
                    │
                    ▼
            Event bus (channel)
                    │
                    ▼
UI ──consume event──▶ dispatch → cập nhật view state
```

- **Command**: gọi thẳng qua trait ở section 4 (mỗi lệnh chạy trên worker riêng).
- **Event** (message từ core về UI) — dạng `enum` khái niệm, mỗi variant là một kết quả hoàn tất:

```rust
enum CoreEvent {
    FilesListed(pane: usize, items: Vec<FileItem>),
    OpFailed(pane: usize, message: String),
    WhoAmIFinished(Result<Option<String>, String>),
    StatfsFinished(Result<(String, String), String>),
    LoginFinished { email: String, result: Result<(), String> },
    LogoutFinished { email: String, result: Result<(), String> },
    TransferProgress { id: usize, update: ProgressUpdate },
    TransferFinished { id: usize, result: Result<(), TransferError> },
    FileOpFinished { kind: FileOpKind, pane: usize, name: String, result: Result<(), String> },
    FileTextFinished { kind: FileOpKind, name: String, result: Result<String, String> },
    RecentsFinished(Result<Vec<FileItem>, String>),
    SyncPairsFinished(Result<Vec<SyncPair>, String>),
    SyncPairFinished { idx: usize, result: Result<(), String> },
    ServerStarted { protocol: ServerProtocol, result: Result<(), String> },
}
```

- Mỗi UI frame: drain event bus một lần, dispatch vào state. Đây là **điểm duy nhất** UI đọc kết quả từ core.

---

## 6. Lifecycle

1. **Khởi tạo**: UI tạo event bus → `AccountManager.load_accounts()` → `AuthSession.whoami(None)` (session hợp lệ?) → nếu có session: `statfs` + load các pane; nếu không: hiện form đăng nhập.
2. **Vòng đời chạy**: mỗi frame, UI gửi lệnh (nếu cần) và drain event bus. Transfer runner chạy độc lập, push `TransferProgress`/`TransferFinished`.
3. **Teardown**: huỷ mọi transfer đang chạy (`cancel_all`), dừng server (`stop_webdav/s3/mount`) nếu đang chạy, lưu tài khoản nếu có thay đổi.

---

## 7. Boundary bất biến (rules bắt buộc khi đổi framework)

1. **Core không import GUI framework** — không type GUI nào lọt vào trait signature; chỉ plain data + `Result`.
2. **UI không gọi CLI/fs trực tiếp** — mọi truy cập đi qua boundary API (trừ adapter helper đã chốt như `copy_local` cũng phải qua core).
3. **Event bus là kênh duy nhất core → UI** cho kết quả async; UI không poll state core.
4. **Toàn bộ thao tác dài (network/CLI) chạy ngoài UI thread** — contract: không được block UI.
5. **Plain data types là hợp đồng chung** — `FileItem`, `TransferItem`, `SyncPair`, `ProgressUpdate`, `TransferStatus`… không được phụ thuộc framework; nếu framework cần thêm trường chỉ để render, phải bọc ở view layer.
6. **Thêm lệnh mới**: thêm method vào trait + variant vào `CoreEvent`; không được đổi chữ ký lệnh cũ (thêm tham số phải qua struct config có default).

---

## 8. Map với code hiện tại

| Mục | Hiện tại |
|---|---|
| App shell + view state | `main.rs` (`FilenGuiApp`, `PaneState`, `AsyncResult`) |
| `Operations` | `src/operations.rs` (`impl Operations` — đúng signature ở section 4) |
| `TransferManager` | `src/transfer.rs` (`TransferManager`, `run_cli_transfer`) |
| `ServersManager` | `src/operations.rs` (`WebDavServerState`/`S3ServerState`/`MountState`) + `main.rs` (`spawn_webdav/s3/mount`) |
| `AccountManager` | `src/operations.rs` (`load_stored_accounts`/`save_stored_accounts`) |
| `AuthSession` | `src/operations.rs` (`whoami`/`statfs`/`login_new`/`logout`) |
| Event bus | `main.rs` `mpsc::Sender<AsyncResult>` — tương đương `CoreEvent` |

> Lưu ý: hiện tại `login_new` nhận thêm `tx: Option<UnboundedSender<AppEvent>>` cho log — về sau nên tách thành event `AuthLog(String)` trong `CoreEvent` để boundary gọn.
