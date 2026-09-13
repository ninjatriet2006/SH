/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Backend (Rust) cho chức năng Gói dịch vụ (Package).
- Trách nhiệm: Giao tiếp với lệnh Tauri, bắt lỗi, và ném ra dữ liệu mảng hoặc đối tượng của `Package`.
- Tương tác: Dùng interface `Package` trong `types.ts`.
*/

// Nhúng lệnh gọi API từ Tauri
// Nhúng kiểu dữ liệu Package
import { invokeCommand, type Package } from './types';

// Hàm gọi API thêm gói dịch vụ
export async function addPackage(name: string, duration_days: number, price: number, description?: string): Promise<Package> {
    try {
        // Gửi lệnh "add_package" xuống Rust
        const result = await invokeCommand<Package, { name: string; duration_days: number; description: string | null; price: number | null }>('add_package', {
            name, 
            duration_days: duration_days, 
            price: price ?? null,
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
        const result = await invokeCommand<Package, { id: string; name: string | null; duration_days: number | null; description: string | null; price: number | null }>('update_package', {
            id, 
            name: name || null, 
            duration_days: duration_days ?? null, 
            price: price ?? null,
            // `??` để "" đi qua = xóa mô tả (backend hiểu "" = None).
            description: description ?? null 
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
        await invokeCommand<void, { id: string }>('delete_package', { id });
    } catch (error) {
        throw new Error(String(error));
    }
}

// Hàm gọi API lấy danh sách toàn bộ các gói dịch vụ
export async function listPackages(page?: number, limit?: number): Promise<Package[]> {
    try {
        // Gọi "list_packages"
        const result = await invokeCommand<Package[], { page: number | null; limit: number | null }>('list_packages', {
            page: page ?? null, 
            limit: limit ?? null 
        });
        return result;
    } catch (error) {
        throw new Error(String(error));
    }
}
