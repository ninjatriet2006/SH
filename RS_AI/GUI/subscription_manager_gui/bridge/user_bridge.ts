/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp các hàm gọi API xuống Backend (Rust) cho chức năng Người dùng.
- Trách nhiệm: Giao tiếp với lệnh Tauri (`invoke`), bắt lỗi và trả về kiểu dữ liệu `User` hoặc ném ra lỗi.
- Tương tác: Import interface từ `types.ts` và sử dụng bởi các Component/Store của Frontend.
*/

// Nhúng hàm invoke từ thư viện Tauri để gọi lệnh Rust
import { invoke } from '@tauri-apps/api/core';
// Nhúng định nghĩa kiểu dữ liệu User
import type { User } from './types';

// Hàm gọi API thêm người dùng
export async function addUser(username: string, email?: string, phone?: string, contact_url?: string): Promise<User> {
    try {
        // Gửi lệnh "add_user" xuống Rust với tham số
        const result = await invoke<User>('add_user', { 
            username, 
            email: email || null,
            phone: phone || null,
            contact_url: contact_url || null
        });
        return result;
    } catch (error) {
        // Nếu có lỗi từ Rust (Result::Err), thì ném lỗi ra ngoài cho Frontend xử lý
        throw new Error(String(error));
    }
}

// Hàm gọi API cập nhật người dùng
export async function updateUser(id: string, username?: string, email?: string, phone?: string, contact_url?: string): Promise<User> {
    try {
        // Gửi lệnh "update_user" xuống Rust
        const result = await invoke<User>('update_user', { 
            id, 
            username: username || null, 
            email: email || null,
            phone: phone || null,
            contact_url: contact_url || null 
        });
        return result;
    } catch (error) {
        // Bắt và ném lỗi
        throw new Error(String(error));
    }
}

// Hàm gọi API xóa người dùng
export async function deleteUser(id: string): Promise<void> {
    try {
        // Gửi lệnh "delete_user" xuống Rust
        await invoke<void>('delete_user', { id });
    } catch (error) {
        // Bắt và ném lỗi
        throw new Error(String(error));
    }
}

// Hàm gọi API lấy danh sách người dùng (hỗ trợ phân trang)
export async function listUsers(page?: number, limit?: number): Promise<User[]> {
    try {
        // Gửi lệnh "list_users" xuống Rust
        const result = await invoke<User[]>('list_users', { 
            page: page ?? null, 
            limit: limit ?? null 
        });
        return result;
    } catch (error) {
        // Bắt và ném lỗi
        throw new Error(String(error));
    }
}
