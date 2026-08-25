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

  /** Record a completed action so it can be undone later */
  public push(action: UndoAction) {
    this.undoStack.push(action);
    this.redoStack = []; // Clear redo stack on new action
    // Keep stack size reasonable (e.g. 50 actions)
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

  /** Undo the most recent action */
  public async undo(): Promise<void> {
    const action = this.undoStack.pop();
    if (!action) {
      logActivity('Undo', 'Không có thao tác nào để hoàn tác.');
      return;
    }

    try {
      switch (action.type) {
        case 'rename':
          // Undo rename: rename dest back to src
          await rename(action.dest, this.basename(action.src), action.account);
          break;
        case 'copy':
          // Undo copy: delete the destination file
          await remove(action.dest, action.account);
          break;
        case 'move':
          // Undo move: move the destination back to the source
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
      // Put it back on the stack since it failed?
      this.undoStack.push(action);
    }
  }

  /** Redo the most recently undone action */
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
