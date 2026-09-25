/*
 * Kiểu dữ liệu dùng chung — khớp 1-1 với struct serde của backend.
 * Bridge KHÔNG import type từ frontend: đây là hợp đồng dây độc lập.
 */

/** Một file/thư mục do `list_files` trả (`actions::list::FileItem`). */
export interface FileItem {
  uuid: string;
  name: string;
  size: number;
  is_dir: boolean;
  mod_time: string;
  file_type: string | null;
}

/** Thông tin xung đột khi chép đè (`actions::conflicts::ConflictInfo`). */
export interface ConflictInfo {
  relative_path: string;
  src_full_path: string;
  dest_full_path: string;
  src_is_dir: boolean;
  dest_is_dir: boolean;
}

/** Thống kê `fs_stat_advanced` (`actions::stat::StatInfo`). */
export interface StatInfo {
  size: number;
  file_count: number;
  dir_count: number;
  permissions: number;
  uid: number;
  gid: number;
}

/** Một kết quả tìm kiếm (`actions::search::SearchResultItem`). */
export interface SearchResultItem {
  item: FileItem;
  path: string;
}

/** Vị trí XDG (`actions::system::UserPlace`). */
export interface UserPlace {
  name: string;
  path: string;
  icon: string;
  kind: string;
}

/** Một app mở file (`actions::system::DesktopApp`). */
export interface DesktopApp {
  name: string;
  exec: string;
  icon: string;
}

/** Mục clipboard (`logic::clipboard`). */
export interface OSClipboardItem {
  pane: string;
  path: string;
}
export interface OSClipboardData {
  items: OSClipboardItem[];
  is_cut: boolean;
}

/** Lệnh tự tạo chuột phải (`logic::custom_action`). */
export interface CustomAction {
  id: string;
  name: string;
  exec: string;
  icon: string;
  selection: string;
  extensions: string[];
}
export interface SimpleFileItem {
  name: string;
  is_dir: boolean;
}

/** Mục thùng rác local (`actions::trash_list::TrashItemLocal`). */
export interface TrashItemLocal {
  id: string;
  name: string;
  original_path: string;
  time_deleted: string;
}

/** Loại việc (`logic::jobs::JobKind`, snake_case). */
export type JobKind = 'copy' | 'move' | 'delete' | 'list' | 'manifest';
/** Trạng thái việc (`logic::jobs::JobStatus`, snake_case). */
export type JobStatus = 'queued' | 'running' | 'done' | 'error' | 'cancelled';
/** Chính sách quyền (`actions::perm::Policy`, snake_case). */
export type PermissionPolicy = 'deny' | 'ask_once' | 'allow_system';

/** Snapshot một việc (`logic::jobs::Job`). */
export interface Job {
  id: string;
  kind: JobKind;
  src: string | null;
  dst: string | null;
  status: JobStatus;
  progress: number;
  error: string | null;
  child_done: number;
  child_total: number;
  policy: PermissionPolicy;
  skip_paths: string[];
  skipped: number;
}

/** Cấu hình engine (`settings::engine::EngineSettings`, JSON PHẲNG). */
export interface EngineSettings {
  transfers: number;
  checkers: number;
  fast_list: boolean;
  server_side_across: boolean;
  dry_run: boolean;
  backup_dir: string | null;
  bulk_transfer: boolean;
}

/** Cấu hình chẩn đoán (`settings::diagnostics::DebugSettings`). */
export interface DebugSettings {
  log_rotate_mb: number;
}

/** Lựa chọn + nguồn giao diện (`settings::appearance::AppearanceSettings`). */
export interface AppearanceSettings {
  lang: string;
  theme: string;
  font: string;
  langs_dir: string;
  themes_dir: string;
  fonts_dir: string;
}

/** Theme (`actions::types::ThemeInfo`). */
export interface ThemeInfo {
  id: string;
  name: string;
  variables: Record<string, string>;
}

/** Font (`actions::types::FontInfo`). */
export interface FontInfo {
  id: string;
  name: string;
  family: string;
  src_path: string | null;
}

/** Cấu hình mount systemd (`actions::mount_editor::MountConfig`). */
export interface MountConfig {
  service_name: string;
  is_user_level: boolean;
  remote_name: string;
  remote_path: string;
  mount_path: string;
  description: string;
  vfs_cache_mode: string;
  vfs_cache_max_size: string;
  vfs_cache_max_age: string;
  dir_cache_time: string;
  buffer_size: string;
  allow_other: boolean;
  read_only: boolean;
}

/** Thông tin service systemd (`actions::mount_query::SystemdServiceInfo`). */
export interface SystemdServiceInfo {
  name: string;
  is_user: boolean;
  status: string;
  enabled: boolean;
}

/** Khả năng transfer (`check_transfer_capability`). */
export interface TransferCapability {
  canMove: boolean;
  canCopy: boolean;
  canCopyDelete: boolean;
}

/** Dung lượng (`rclone_about`). */
export interface AboutInfo {
  total?: number;
  used?: number;
  free?: number;
  trashed?: number;
  other?: number;
}

/** Đếm (`rclone_size`). */
export interface SizeInfo {
  count?: number;
  bytes?: number;
  sizeless?: number;
}

/** Sự kiện nhật ký backend (`core::debug::LogEvent`). */
export interface BackendLogEvent {
  level: 'Info' | 'Warn' | 'Error';
  tag: string;
  message: string;
}
