/*
[INTEGRITY NOTES]
- Mục đích: Bridge gọi API provider xuống backend Rust.
- Trách nhiệm: Gọi lệnh Tauri và NÉM lỗi ra cho UI xử lý. Không nuốt lỗi: đây là
  thao tác ghi cấu hình thật của người dùng, thất bại phải báo.
- Tương tác: `store/useProviderStore.ts`, backend `api/provider.rs`.

Tham số dùng snake_case khớp `rename_all = "snake_case"` ở backend. Tauri v2 mặc
định đổi sang camelCase; nếu hai bên lệch, IPC báo "missing required key" và
command không bao giờ chạy.
*/

import { invoke } from '@tauri-apps/api/core';
import type {
    ProviderView, PresetView, SaveResult, StatusView, ScannedModel, BadProvider, ModelCaps,
} from './types';

export async function listProviders(): Promise<ProviderView[]> {
    try {
        return await invoke<ProviderView[]>('list_providers');
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function listPresets(): Promise<PresetView[]> {
    try {
        return await invoke<PresetView[]>('list_presets');
    } catch (error) {
        throw new Error(String(error));
    }
}

/** Lấy API key THẬT — chỉ gọi khi mở form sửa, không lưu lâu trong state. */
export async function getProviderSecret(providerId: string): Promise<string> {
    try {
        return await invoke<string>('get_provider_secret', { provider_id: providerId });
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function saveProvider(args: {
    providerId?: string;
    presetId: string;
    name: string;
    baseUrl: string;
    apiKey: string;
    forceOverwriteId?: string;
    /** Package AI SDK (`@ai-sdk/openai-compatible` hay `@ai-sdk/openai`). */
    npm?: string;
    /** ID provider do người dùng tự đặt (thay vì tự sinh `custom_2`…). */
    customId?: string | null;
}): Promise<SaveResult> {
    try {
        return await invoke<SaveResult>('save_provider', {
            provider_id: args.providerId ?? '',
            preset_id: args.presetId,
            name: args.name,
            base_url: args.baseUrl,
            api_key: args.apiKey,
            force_overwrite_id: args.forceOverwriteId ?? null,
            npm: args.npm ?? null,
            custom_id: args.customId ?? null,
        });
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function deleteProvider(providerId: string): Promise<void> {
    try {
        await invoke<void>('delete_provider', { provider_id: providerId });
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function deleteProviders(providerIds: string[]): Promise<number> {
    try {
        return await invoke<number>('delete_providers', { provider_ids: providerIds });
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function testProvider(providerId: string): Promise<StatusView> {
    try {
        return await invoke<StatusView>('test_provider', { provider_id: providerId });
    } catch (error) {
        throw new Error(String(error));
    }
}

/** Kiểm tra một cặp URL/key CHƯA lưu (nút Kiểm tra trong form). */
export async function testConnection(baseUrl: string, apiKey: string): Promise<StatusView> {
    try {
        return await invoke<StatusView>('test_connection', { base_url: baseUrl, api_key: apiKey });
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function testAllProviders(): Promise<StatusView[]> {
    try {
        return await invoke<StatusView[]>('test_all_providers');
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function scanProviderModels(providerId: string): Promise<ScannedModel[]> {
    try {
        return await invoke<ScannedModel[]>('scan_provider_models', { provider_id: providerId });
    } catch (error) {
        throw new Error(String(error));
    }
}

/**
 * `selected` là danh sách CUỐI CÙNG — model không có trong đây sẽ bị xoá.
 * `caps` (tuỳ chọn) ghi đè capability từng model: tool_call/reasoning/
 * interleaved (case `hy3` — model reasoning cần khai đúng nếu không OpenCode
 * gửi request sai shape).
 */
export async function setProviderModels(
    providerId: string,
    selected: string[],
    caps?: Record<string, ModelCaps>,
): Promise<ProviderView> {
    try {
        return await invoke<ProviderView>('set_provider_models', {
            provider_id: providerId,
            selected,
            caps: caps ?? null,
        });
    } catch (error) {
        throw new Error(String(error));
    }
}

export async function findBadProviders(): Promise<BadProvider[]> {
    try {
        return await invoke<BadProvider[]>('find_bad_providers');
    } catch (error) {
        throw new Error(String(error));
    }
}
