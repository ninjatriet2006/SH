/*
[INTEGRITY NOTES]
- Mục đích: Quản lý trạng thái cho các Gói dịch vụ (Package).
- Trách nhiệm: Giữ dữ liệu của các package trên giao diện, tương tác với `package_bridge` để cập nhật dữ liệu.
- Tương tác: Dùng Zustand, kết nối với bridge.
*/

import { create } from 'zustand';
import type { Package } from '../../../bridge/types';
import { listPackages, addPackage, updatePackage, deletePackage } from '../../../bridge/package_bridge';

interface PackageState {
    // Mảng lưu trữ các gói dịch vụ
    packages: Package[];
    // Trạng thái chờ
    isLoading: boolean;
    // Hàm tải danh sách
    fetchPackages: (page?: number, limit?: number) => Promise<void>;
    // Hàm thêm
    addNewPackage: (name: string, duration_days: number, price: number, description?: string) => Promise<void>;
    // Hàm sửa
    editPackage: (id: string, name?: string, duration_days?: number, price?: number, description?: string) => Promise<void>;
    // Hàm xóa
    removePackage: (id: string) => Promise<void>;
}

export const usePackageStore = create<PackageState>((set, get) => ({
    packages: [],
    isLoading: false,

    fetchPackages: async (page, limit) => {
        set({ isLoading: true });
        try {
            // Lấy từ API
            const data = await listPackages(page, limit);
            set({ packages: data });
        } catch (error) {
            console.error("Lỗi lấy danh sách package:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    addNewPackage: async (name, duration_days, price, description) => {
        set({ isLoading: true });
        try {
            // Thêm mới
            const newPkg = await addPackage(name, duration_days, price, description);
            // Nối vào mảng
            set({ packages: [...get().packages, newPkg] });
        } catch (error) {
            console.error("Lỗi thêm package:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    editPackage: async (id, name, duration_days, price, description) => {
        set({ isLoading: true });
        try {
            // Sửa
            const updatedPkg = await updatePackage(id, name, duration_days, price, description);
            // Cập nhật lại mảng
            set({ packages: get().packages.map(p => p.id === id ? updatedPkg : p) });
        } catch (error) {
            console.error("Lỗi cập nhật package:", error);
        } finally {
            set({ isLoading: false });
        }
    },

    removePackage: async (id) => {
        set({ isLoading: true });
        try {
            // Xóa
            await deletePackage(id);
            // Xóa khỏi mảng
            set({ packages: get().packages.filter(p => p.id !== id) });
        } catch (error) {
            console.error("Lỗi xóa package:", error);
        } finally {
            set({ isLoading: false });
        }
    }
}));
