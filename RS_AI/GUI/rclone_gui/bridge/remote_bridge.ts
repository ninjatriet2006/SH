/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Rust Backend cho cấu hình và giám sát Remote Cloud.
- Trách nhiệm: Đóng gói tham số, gọi lệnh qua `invokeCommand` theo chuẩn Enveloped IPC A.1.
- Tương tác: Dùng các interface trong `types.ts`.
*/

import {
  invokeCommand,
  type AboutInfo,
  type FeatureFlags,
  type IntegrityCheckResult,
  type ProviderInfo,
  type RemoteConfig,
  type SizeInfo,
  type TransferCapability,
} from './types';

/** Danh sách remote đã cấu hình (`rclone config dump`). */
export async function listRemotes(): Promise<RemoteConfig[]> {
  try {
    return await invokeCommand<RemoteConfig[]>('list_remotes');
  } catch (error) {
    console.error('Lỗi list_remotes:', error);
    return [];
  }
}

/** Danh sách nhà cung cấp dịch vụ đám mây hỗ trợ (`rclone config providers`). */
export async function getProviders(): Promise<ProviderInfo[]> {
  try {
    const raw = await invokeCommand<string>('get_providers');
    return JSON.parse(raw) as ProviderInfo[];
  } catch (error) {
    console.error('Lỗi get_providers:', error);
    return [];
  }
}

/** Tạo remote mới. */
export async function createRemote(
  name: string,
  provider: string,
  options: Record<string, string>,
): Promise<string> {
  return await invokeCommand<string, { name: string; provider: string; options: Record<string, string> }>(
    'create_remote',
    { name, provider, options },
  );
}

/** Sửa options remote. */
export async function updateRemote(name: string, options: Record<string, string>): Promise<string> {
  return await invokeCommand<string, { name: string; options: Record<string, string> }>('update_remote', {
    name,
    options,
  });
}

/** Xoá remote. */
export async function deleteRemote(name: string): Promise<string> {
  return await invokeCommand<string, { name: string }>('delete_remote', { name });
}

/** Năng lực backend (`rclone backend features`). */
export async function getBackendFeatures(remote: string): Promise<unknown> {
  try {
    return await invokeCommand<unknown, { remote: string }>('get_backend_features', { remote });
  } catch (error) {
    console.error(`Lỗi get_backend_features ${remote}:`, error);
    return null;
  }
}

/** Toàn bộ 52 cờ Features của một remote. */
export async function getFeatureFlags(remote: string): Promise<FeatureFlags | null> {
  try {
    return await invokeCommand<FeatureFlags, { remote: string }>('get_feature_flags', { remote });
  } catch (error) {
    console.error(`Lỗi get_feature_flags ${remote}:`, error);
    return null;
  }
}

/** Khả năng copy/move giữa 2 đường dẫn (não `checkcap`). */
export async function checkTransferCapability(src: string, dst: string): Promise<TransferCapability> {
  try {
    return await invokeCommand<TransferCapability, { src: string; dst: string }>(
      'check_transfer_capability',
      { src, dst },
    );
  } catch (error) {
    console.error('Lỗi check_transfer_capability:', error);
    return { canMove: false, canCopy: false, canCopyDelete: false };
  }
}

/** Dung lượng (`rclone about`). */
export async function getAbout(remote: string): Promise<AboutInfo> {
  try {
    return await invokeCommand<AboutInfo, { remote: string }>('rclone_about', { remote });
  } catch (error) {
    console.error(`Lỗi rclone_about ${remote}:`, error);
    return {};
  }
}

/** Đếm file và dung lượng (`rclone size`). */
export async function getSize(remote: string): Promise<SizeInfo> {
  try {
    return await invokeCommand<SizeInfo, { remote: string }>('rclone_size', { remote });
  } catch (error) {
    console.error(`Lỗi rclone_size ${remote}:`, error);
    return {};
  }
}

/** Lấy mã băm (hash) của tệp tin qua rclone hashsum. */
export async function getFileHash(path: string, hashType: string): Promise<string> {
  return await invokeCommand<string, { path: string; hash_type: string }>('get_file_hash', {
    path,
    hash_type: hashType,
  });
}

/** Lấy danh sách các loại mã băm mà remote hỗ trợ. */
export async function getRemoteHashes(remote: string): Promise<string[]> {
  try {
    return await invokeCommand<string[], { remote: string }>('get_remote_hashes', { remote });
  } catch (error) {
    console.error(`Lỗi get_remote_hashes ${remote}:`, error);
    return [];
  }
}

/** So sánh kiểm tra toàn vẹn nội dung giữa hai nguồn (rclone check --combined). */
export async function checkFilesIntegrity(src: string, dst: string): Promise<IntegrityCheckResult> {
  return await invokeCommand<IntegrityCheckResult, { src: string; dst: string }>(
    'check_files_integrity',
    { src, dst },
  );
}
