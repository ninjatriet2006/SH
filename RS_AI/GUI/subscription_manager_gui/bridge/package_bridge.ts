/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Backend (Rust) cho chức năng Gói dịch vụ (Package).
- Trách nhiệm: Giao tiếp với lệnh Tauri, bắt lỗi, và ném ra dữ liệu mảng hoặc đối tượng của `Package`.
- Tương tác: Dùng interface `Package` trong `types.ts`.
*/

// Nhúng lệnh gọi API từ Tauri
import { invoke } from '@tauri-apps/api/core';
// Nhúng kiểu dữ liệu Package
import type { Package } from './types';

// Hàm gọi API thêm gói dịch vụ
export async function addPackage(name: string, duration_days: number, price: number, description?: string): Promise<Package> {
    try {
        // Gửi lệnh "add_package" xuống Rust
        const result = await invoke<Package>('add_package', { 
            name, 
            durationDays: duration_days, 
            price,
            description: description || null 
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API cập nhật gói dịch vụ
export async function updatePackage(id: string, name?: string, duration_days?: number, price?: number, description?: string): Promise<Package> {
    try {
        // Gọi lệnh "update_package" 
        const result = await invoke<Package>('update_package', { 
            id, 
            name: name || null, 
            durationDays: duration_days || null, 
            price: price ?? null,
            description: description || null 
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API xóa gói dịch vụ
export async function deletePackage(id: string): Promise<void> {
    try {
        // Gọi "delete_package"
        await invoke<void>('delete_package', { id });
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API lấy danh sách toàn bộ các gói dịch vụ
export async function listPackages(page?: number, limit?: number): Promise<Package[]> {
    try {
        // Gọi "list_packages"
        const result = await invoke<Package[]>('list_packages', { 
            page: page ?? null, 
            limit: limit ?? null 
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}
