/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp Store lưu trữ và quản lý các Action (Lệnh tùy chỉnh) cho menu chuột phải.
- Trách nhiệm: Đọc cấu hình các CustomAction từ hệ thống, kiểm tra tính hợp lệ của Action so với file được chọn, và thực thi lệnh.
- Tương tác: Gọi backend API thông qua `sys_get_custom_actions` và `sys_execute_custom_action`.
*/

import { invoke } from '@tauri-apps/api/core';
import type { FileItem } from '../store';

// ====================================================================================
// BLOCK: INTERFACES VÀ KHAI BÁO KIỂU DỮ LIỆU
// ====================================================================================

/** Cấu trúc lưu trữ thông tin của một lệnh cấu hình thủ công (Custom Action) */
export interface CustomAction {
  id: string;             // Định danh độc nhất của hành động
  name: string;           // Tên hiển thị trên menu chuột phải
  exec: string;           // Lệnh thực thi thực tế (vd: code %f)
  icon: string;           // Biểu tượng icon hiển thị
  selection: string;      // Yêu cầu chọn (s = single/một file, m = multiple/nhiều file, any)
  extensions: string[];   // Các loại đuôi file hỗ trợ (vd: ["txt", "md"])
}

// ====================================================================================
// BLOCK: LỚP QUẢN LÝ ACTION STORE
// ====================================================================================

class ActionStore {
  // Biến lưu trữ tạm trên RAM (mảng các hành động tự cấu hình)
  private actions: CustomAction[] = [];

  constructor() {
    // Tự động tải danh sách actions ngay khi khởi tạo Store
    this.fetchActions();
  }

  /** Tên hàm: fetchActions | Mô tả: Giao tiếp với Backend (sys.rs) để lấy danh sách config CustomAction */
  public async fetchActions() {
    try {
      this.actions = await invoke('sys_get_custom_actions');
      console.log('Đã tải thành công các Custom Actions:', this.actions);
    } catch (e) {
      console.error('Lỗi khi tải Custom Actions:', e);
    }
  }

  /** 
   * Tên hàm: getValidActionsForSelection 
   * Mô tả: Giao tiếp với Backend để lọc danh sách các action hợp lệ (có thể click) dựa vào các file đang được chọn.
   */
  public async getValidActionsForSelection(files: FileItem[]): Promise<CustomAction[]> {
    if (this.actions.length === 0) return [];
    
    try {
      // Gửi xuống backend để lọc extension
      return await invoke('sys_get_valid_actions', { files });
    } catch (e) {
      console.error('Lỗi khi lấy Custom Actions hợp lệ:', e);
      return [];
    }
  }

  /** 
   * Tên hàm: executeAction 
   * Mô tả: Tiến hành thực thi câu lệnh Terminal tùy chỉnh trên mảng các file đang được chọn.
   */
  public async executeAction(action: CustomAction, files: FileItem[], basePath: string) {
    try {
      // Đẩy lệnh, danh sách tên file, và thư mục hiện tại xuống backend thực thi
      await invoke('sys_execute_custom_action', { 
        execTemplate: action.exec,
        basePath: basePath,
        fileNames: files.map(f => f.name)
      });
    } catch (e) {
      console.error(`Thất bại khi chạy Custom Action ${action.name}:`, e);
      alert(`Lỗi thực thi lệnh: ${e}`);
    }
  }
}

// Biến instance toàn cục
export const actionStore = new ActionStore();
