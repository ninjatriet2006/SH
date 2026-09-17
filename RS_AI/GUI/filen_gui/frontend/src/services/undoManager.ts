import { logActivity } from '../store';

/**
 * File-operation facade được inject từ tầng bootstrap (main.ts).
 * undoManager KHÔNG import trực tiếp fileOps để tránh circular import
 * fileOps ⇄ undoManager — xem TODO filen_gui_circular_import_todo.
 */
export interface UndoFileOps {
  rename(path: string, newName: string, account?: string): Promise<void>;
  remove(path: string, account?: string): Promise<void>;
  copy(src: string, dest: string, account?: string): Promise<void>;
  move(src: string, dest: string, account?: string): Promise<void>;
  cpLocal(from: string, to: string, overwrite?: boolean): Promise<void>;
  moveLocal(from: string, to: string): Promise<void>;
}

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
  private ops?: UndoFileOps;

  /** Inject file-ops từ bootstrap (main.ts). Phải gọi trước khi undo/redo lần đầu. */
  public setFileOps(ops: UndoFileOps) {
    this.ops = ops;
  }

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
    if (!this.ops) {
      logActivity('Lỗi Hoàn tác', 'UndoManager chưa được khởi tạo file-ops.');
      this.undoStack.push(action);
      return;
    }
    const ops = this.ops;

    try {
      switch (action.type) {
        case 'rename':
          // Undo rename: rename dest back to src
          await ops.rename(action.dest, this.basename(action.src), action.account);
          break;
        case 'copy':
          // Undo copy: delete the destination file
          if (!confirm(`Xoá ${this.basename(action.dest)} để hoàn tác thao tác sao chép?`)) {
            throw new Error('Đã huỷ hoàn tác sao chép.');
          }
          await ops.remove(action.dest, action.account);
          break;
        case 'move':
          // Undo move: move the destination back to the source
          if (!confirm(`Di chuyển ${this.basename(action.dest)} về vị trí cũ?`)) {
            throw new Error('Đã huỷ hoàn tác di chuyển.');
          }
          if (action.isLocal) {
            await ops.moveLocal(action.dest, action.src);
          } else {
            await ops.move(action.dest, action.src, action.account);
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
    if (!this.ops) {
      logActivity('Lỗi Làm lại', 'UndoManager chưa được khởi tạo file-ops.');
      this.redoStack.push(action);
      return;
    }
    const ops = this.ops;

    try {
      switch (action.type) {
        case 'rename':
          if (!confirm(`Thực hiện lại việc đổi tên ${this.basename(action.src)}?`)) {
            throw new Error('Đã huỷ làm lại thao tác đổi tên.');
          }
          await ops.rename(action.src, this.basename(action.dest), action.account);
          break;
        case 'copy':
          if (!confirm(`Thực hiện lại việc sao chép ${this.basename(action.src)}?`)) {
            throw new Error('Đã huỷ làm lại thao tác sao chép.');
          }
          if (action.isLocal) {
            await ops.cpLocal(action.src, action.dest, true);
          } else {
            await ops.copy(action.src, action.dest, action.account);
          }
          break;
        case 'move':
          if (!confirm(`Thực hiện lại việc di chuyển ${this.basename(action.src)}?`)) {
            throw new Error('Đã huỷ làm lại thao tác di chuyển.');
          }
          if (action.isLocal) {
            await ops.moveLocal(action.src, action.dest);
          } else {
            await ops.move(action.src, action.dest, action.account);
          }
          break;
        case 'delete':
          if (!confirm(`Xoá lại ${this.basename(action.src)}?`)) {
            throw new Error('Đã huỷ làm lại thao tác xoá.');
          }
          await ops.remove(action.src, action.account);
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
