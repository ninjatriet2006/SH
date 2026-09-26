/*
 * Remote cloud — tương ứng `backend/src/api/remote_manager.rs`.
 * Pure gọi lệnh: lỗi đọc trả rỗng, lỗi ghi ném cho UI hiện dialog.
 */

import { invoke } from './ipc';
import type { AboutInfo, IntegrityCheckResult, SizeInfo, TransferCapability } from './types';

export interface RemoteConfig {
  name: string;
  type: string;
  [key: string]: unknown;
}

export interface ProviderOption {
  Name: string;
  Help: string;
  Type: string;
  Required: boolean;
  Advanced: boolean;
  IsPassword?: boolean;
  DefaultStr?: string;
  Examples?: Array<{ Value: string; Help: string }>;
}

export interface ProviderInfo {
  Name: string;
  Description: string;
  Prefix: string;
  Options: ProviderOption[];
}

/** Danh sách remote đã cấu hình (`rclone config dump`). */
export async function listRemotes(): Promise<RemoteConfig[]> {
  try {
    return await invoke<RemoteConfig[]>('list_remotes');
  } catch (error) {
    console.error('Lỗi list_remotes:', error);
    return [];
  }
}

/** Danh sách hãng hỗ trợ (`rclone config providers`, JSON chuỗi). */
export async function getProviders(): Promise<ProviderInfo[]> {
  try {
    return JSON.parse(await invoke<string>('get_providers')) as ProviderInfo[];
  } catch (error) {
    console.error('Lỗi get_providers:', error);
    return [];
  }
}

/** Tạo remote mới. */
export async function createRemote(name: string, provider: string, options: Record<string, string>): Promise<string> {
  return await invoke<string>('create_remote', { name, provider, options });
}

/** Sửa options remote. */
export async function updateRemote(name: string, options: Record<string, string>): Promise<string> {
  return await invoke<string>('update_remote', { name, options });
}

/** Xoá remote. */
export async function deleteRemote(name: string): Promise<string> {
  return await invoke<string>('delete_remote', { name });
}

/** Năng lực backend (`rclone backend features`). */
export async function getBackendFeatures(remote: string): Promise<unknown> {
  try {
    return await invoke<unknown>('get_backend_features', { remote });
  } catch (error) {
    console.error(`Lỗi get_backend_features ${remote}:`, error);
    return null;
  }
}

/** Toàn bộ 52 cờ Features đã parse (`BackendFeatures`, snake_case). */
export interface FeatureFlags {
  about: boolean; bucket_based: boolean; bucket_based_root_ok: boolean;
  can_have_empty_directories: boolean; case_insensitive: boolean;
  change_notify: boolean; chunk_writer_doesnt_seek: boolean; clean_up: boolean;
  command: boolean; copy: boolean; dir_cache_flush: boolean;
  dir_mod_time_updates_on_write: boolean; dir_move: boolean;
  dir_set_mod_time: boolean; disconnect: boolean; double_slash: boolean;
  duplicate_files: boolean; filter_aware: boolean; get_tier: boolean;
  is_local: boolean; list_p: boolean; list_r: boolean; merge_dirs: boolean;
  mkdir_metadata: boolean; move_native: boolean; no_multi_threading: boolean;
  open_chunk_writer: boolean; open_writer_at: boolean; overlay: boolean;
  partial_uploads: boolean; public_link: boolean; purge: boolean;
  put_stream: boolean; put_unchecked: boolean; read_dir_metadata: boolean;
  read_metadata: boolean; read_mime_type: boolean;
  server_side_across_configs: boolean; set_tier: boolean; set_wrapper: boolean;
  shutdown: boolean; slow_hash: boolean; slow_mod_time: boolean;
  un_wrap: boolean; user_dir_metadata: boolean; user_info: boolean;
  user_metadata: boolean; wrap_fs: boolean; write_dir_metadata: boolean;
  write_dir_set_mod_time: boolean; write_metadata: boolean;
  write_mime_type: boolean;
}

/** Toàn bộ 52 cờ Features của một remote (lỗi → null). */
export async function getFeatureFlags(remote: string): Promise<FeatureFlags | null> {
  try {
    return await invoke<FeatureFlags>('get_feature_flags', { remote });
  } catch (error) {
    console.error(`Lỗi get_feature_flags ${remote}:`, error);
    return null;
  }
}

/** Khả năng copy/move giữa 2 đường dẫn (não `checkcap`). */
export async function checkTransferCapability(src: string, dst: string): Promise<TransferCapability> {
  try {
    return await invoke<TransferCapability>('check_transfer_capability', { src, dst });
  } catch (error) {
    console.error('Lỗi check_transfer_capability:', error);
    return { canMove: false, canCopy: false, canCopyDelete: false };
  }
}

/** Dung lượng (`rclone about`). */
export async function getAbout(remote: string): Promise<AboutInfo> {
  try {
    return await invoke<AboutInfo>('rclone_about', { remote });
  } catch (error) {
    console.error(`Lỗi rclone_about ${remote}:`, error);
    return {};
  }
}

/** Đếm (`rclone size`). */
export async function getSize(remote: string): Promise<SizeInfo> {
  try {
    return await invoke<SizeInfo>('rclone_size', { remote });
  } catch (error) {
    console.error(`Lỗi rclone_size ${remote}:`, error);
    return {};
  }
}

/** Lấy mã băm (hash) của tệp tin qua rclone hashsum. */
export async function getFileHash(path: string, hashType: string): Promise<string> {
  return await invoke<string>('get_file_hash', { path, hash_type: hashType });
}

/** Lấy danh sách các loại mã băm mà remote hỗ trợ. */
export async function getRemoteHashes(remote: string): Promise<string[]> {
  try {
    return await invoke<string[]>('get_remote_hashes', { remote });
  } catch (error) {
    console.error(`Lỗi get_remote_hashes ${remote}:`, error);
    return [];
  }
}

/** So sánh kiểm tra toàn vẹn nội dung giữa hai nguồn (rclone check --combined). */
export async function checkFilesIntegrity(
  src: string,
  dst: string,
): Promise<IntegrityCheckResult> {
  return await invoke<IntegrityCheckResult>('check_files_integrity', { src, dst });
}
