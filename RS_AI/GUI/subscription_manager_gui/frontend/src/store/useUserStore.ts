/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái (State Management) cho danh sách Người dùng (User).
- Trách nhiệm: Lưu trữ danh sách User, gọi API (thông qua user_bridge) để nạp (fetch), thêm (add), sửa (update), xóa (delete) và đồng bộ với giao diện React.
- Tương tác: Dùng Zustand tạo Hook. Import các hàm từ `../../bridge/user_bridge`.
*/

// Nhúng thư viện Zustand để tạo global state
import { create } from 'zustand';
// Nhúng interface User và các hàm gọi API từ bridge
import type { User } from '../../../bridge/types';
import { listUsers, addUser, updateUser, deleteUser } from '../../../bridge/user_bridge';

// Định nghĩa cấu trúc State và Actions của UserStore
interface UserState {
    // Trạng thái danh sách người dùng
    users: User[];
    // Trạng thái đang tải (loading) để hiển thị giao diện chờ
    isLoading: boolean;
    // Hàm nạp danh sách từ backend
    fetchUsers: (page?: number, limit?: number) => Promise<void>;
    // Hàm thêm người dùng
    addNewUser: (username: string, email?: string, phone?: string, contactUrl?: string) => Promise<void>;
    // Hàm cập nhật người dùng
    editUser: (id: string, username?: string, email?: string, phone?: string, contactUrl?: string) => Promise<void>;
    // Hàm xóa người dùng
    removeUser: (id: string) => Promise<void>;
}

// Khởi tạo Zustand store
export const useUserStore = create<UserState>((set, get) => ({
    users: [],          // Ban đầu mảng người dùng rỗng
    isLoading: false,   // Ban đầu không ở trạng thái tải

    // Hàm gọi API lấy danh sách
    fetchUsers: async (page, limit) => {
        // Đặt trạng thái tải thành true
        set({ isLoading: true });
        try {
            // Lấy dữ liệu từ Backend thông qua bridge
            const data = await listUsers(page, limit);
            // Cập nhật state danh sách users
            set({ users: data });
        } catch (error) {
            console.error("Lỗi lấy danh sách user:", error);
        } finally {
            // Tắt trạng thái tải
            set({ isLoading: false });
        }
    },

    // Hàm gọi API thêm mới người dùng
    addNewUser: async (username, email, phone, contactUrl) => {
        set({ isLoading: true });
        try {
            // Gọi Backend thêm user
            const newUser = await addUser(username, email, phone, contactUrl);
            // Lấy state hiện tại
            const currentUsers = get().users;
            // Nối user mới vào mảng
            set({ users: [...currentUsers, newUser] });
        } catch (error) {
            console.error("Lỗi thêm user:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    // Hàm gọi API cập nhật thông tin
    editUser: async (id, username, email, phone, contactUrl) => {
        set({ isLoading: true });
        try {
            // Gọi Backend sửa user
            const updatedUser = await updateUser(id, username, email, phone, contactUrl);
            const currentUsers = get().users;
            // Thay thế user cũ bằng user đã sửa trong mảng
            set({ users: currentUsers.map(u => u.id === id ? updatedUser : u) });
        } catch (error) {
            console.error("Lỗi cập nhật user:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    // Hàm gọi API xóa
    removeUser: async (id) => {
        set({ isLoading: true });
        try {
            // Gọi Backend xóa user
            await deleteUser(id);
            const currentUsers = get().users;
            // Lọc bỏ user vừa xóa khỏi mảng
            set({ users: currentUsers.filter(u => u.id !== id) });
        } catch (error) {
            console.error("Lỗi xóa user:", error);
        } finally {
            set({ isLoading: false });
        }
    }
}));
