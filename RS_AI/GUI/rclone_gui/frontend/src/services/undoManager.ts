/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp tính năng Hoàn tác (Undo) và Làm lại (Redo) cho các thao tác file.
- Trách nhiệm: Lưu trữ lịch sử các tác vụ (copy, move, rename), tính toán thao tác đảo ngược (reverse action).
- Tương tác: Giao tiếp với `fileOps` để thực hiện thao tác vật lý, ghi log qua `store`.
*/

import { logActivity } from '../store';
import { rename, remove, copy, move, cpLocal, moveLocal } from './fileOps';

export type UndoActionType = 'rename' | 'copy' | 'move' | 'delete';

export interface UndoAction {
  type: UndoActionType;
  src: string;
  dest: string;
  account?: string;
  isLocal: boolean;
}

class UndoManager {
  private undoStack: UndoAction[] = [];
  private redoStack: UndoAction[] = [];

  /** 
   * Tên hàm: push 
   * Mô tả: Ghi lại một thao tác vừa hoàn tất để có thể hoàn tác sau này. 
   */
  public push(action: UndoAction) {
    this.undoStack.push(action);
    this.redoStack = []; // Xóa ngăn xếp redo khi có hành động mới
    // Giữ kích thước ngăn xếp ở mức vừa phải (ví dụ: 50 thao tác)
    if (this.undoStack.length > 50) {
      this.undoStack.shift();
    }
  }

  public get undoCount() {
    return this.undoStack.length;
  }

  public get redoCount() {
    return this.redoStack.length;
  }

  /** 
   * Tên hàm: undo 
   * Mô tả: Hoàn tác hành động gần nhất 
   */
  public async undo(): Promise<void> {
    const action = this.undoStack.pop();
    if (!action) {
      logActivity('Undo', 'Không có thao tác nào để hoàn tác.');
      return;
    }

    try {
      switch (action.type) {
        case 'rename':
          // Hoàn tác rename: đổi tên đích (dest) trở về nguồn (src)
          await rename(action.dest, this.basename(action.src), action.account);
          break;
        case 'copy':
          // Hoàn tác copy: xóa file/thư mục ở đích (dest)
          await remove(action.dest, action.account);
          break;
        case 'move':
          // Hoàn tác move: di chuyển đích (dest) quay lại nguồn (src)
          if (action.isLocal) {
            await moveLocal(action.dest, action.src);
          } else {
            await move(action.dest, action.src, action.account);
          }
          break;
        case 'delete':
          throw new Error('Undo xoá chưa được hỗ trợ vì chưa có hệ thống Thùng rác.');
      }
      this.redoStack.push(action);
      logActivity('Đã hoàn tác (Undo)', `${this.getActionVerb(action.type)} ${this.basename(action.src)}`);
    } catch (e) {
      logActivity('Lỗi Hoàn tác', String(e));
      // Đặt lại thao tác vào ngăn xếp nếu hoàn tác thất bại
      this.undoStack.push(action);
    }
  }

  /** 
   * Tên hàm: redo 
   * Mô tả: Làm lại hành động vừa bị hoàn tác gần nhất 
   */
  public async redo(): Promise<void> {
    const action = this.redoStack.pop();
    if (!action) {
      logActivity('Redo', 'Không có thao tác nào để làm lại.');
      return;
    }

    try {
      switch (action.type) {
        case 'rename':
          await rename(action.src, this.basename(action.dest), action.account);
          break;
        case 'copy':
          if (action.isLocal) {
            await cpLocal(action.src, action.dest, true);
          } else {
            await copy(action.src, action.dest, action.account);
          }
          break;
        case 'move':
          if (action.isLocal) {
            await moveLocal(action.src, action.dest);
          } else {
            await move(action.src, action.dest, action.account);
          }
          break;
        case 'delete':
          await remove(action.src, action.account);
          break;
      }
      this.undoStack.push(action);
      logActivity('Đã làm lại (Redo)', `${this.getActionVerb(action.type)} ${this.basename(action.src)}`);
    } catch (e) {
      logActivity('Lỗi Làm lại', String(e));
      this.redoStack.push(action);
    }
  }

  private basename(path: string): string {
    const norm = path.replace(/\\/g, '/');
    const parts = norm.split('/').filter(Boolean);
    return parts.length > 0 ? parts[parts.length - 1] : path;
  }

  private getActionVerb(type: UndoActionType): string {
    switch (type) {
      case 'rename': return 'Đổi tên';
      case 'copy': return 'Sao chép';
      case 'move': return 'Di chuyển';
      case 'delete': return 'Xoá';
      default: return type;
    }
  }
}

export const undoManager = new UndoManager();
