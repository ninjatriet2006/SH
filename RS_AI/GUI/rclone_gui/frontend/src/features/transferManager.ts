/*
[INTEGRITY NOTES]
- Mục đích: Quản lý hàng đợi tiến trình (Transfer Queue) cho các tác vụ Copy, Move, Delete.
- Trách nhiệm: Lưu trữ trạng thái tiến trình, lắng nghe sự kiện từ backend (Tauri events) để cập nhật tiến độ, tốc độ, hỗ trợ hủy (cancel) tác vụ.
- Tương tác: Giao tiếp với `fileOps` (Frontend), gọi xuống backend qua IPC, hiển thị UI Fallback Modal.
*/

import { appState } from '../store';
import { undoManager } from '../services/undoManager';
import { joinPath } from './dragDrop';
import * as fileOps from '../services/fileOps';
import { FallbackModal } from '../components/FallbackModal';
import { checkTransferCapability } from '../../../bridge/remote_api';
import { fsDelete, fsCancel } from '../../../bridge/explorer_api';
import { listen } from '@tauri-apps/api/event';

export type TransferKind = 'upload' | 'download' | 'copy' | 'move' | 'delete';
export type TransferStatus = 'queued' | 'running' | 'done' | 'error' | 'cancelled';

export interface TransferTask {
  id: number;
  kind: TransferKind;
  name: string;
  src: string;
  dst: string;
  status: TransferStatus;
  progress: number | null; // 0..1
  bytesDone: number;
  totalBytes: number;
  error?: string;
  speed: number;
  lastUpdateTime: number;
  lastBytesDone: number;
  srcLocal: boolean;
  dstLocal: boolean;
  onSuccess?: () => void;
  transferringFiles?: {name: string, percentage: number, bytes: number, size: number, speed: number, eta: number}[];
  excludes?: string[];
}

// ====================================================================================
// BLOCK: LỚP QUẢN LÝ TIẾN TRÌNH (TRANSFER MANAGER)
// ====================================================================================
class TransferManager {
  public tasks: Map<number, TransferTask> = new Map();
  public onUpdate?: () => void;
  public onQueueEmptyListeners: (() => void)[] = [];
  private nextId = 1;
  private isProcessing = false;

  constructor() {
    // Lắng nghe luồng sự kiện báo cáo tiến độ (progress) trực tiếp từ Backend Rclone
    listen('transfer_progress', (event: any) => {
      const payload = event.payload;
      if (payload && payload.id !== undefined && payload.stats) {
        const task = Array.from(this.tasks.values()).find(t => t.id === payload.id);
        if (task && task.status === 'running') {
          task.bytesDone = payload.stats.bytes;
          task.totalBytes = payload.stats.totalBytes;
          task.speed = payload.stats.speed;
          if (task.totalBytes > 0) {
              task.progress = task.bytesDone / task.totalBytes;
          }
          if (payload.stats.transferring) {
            task.transferringFiles = payload.stats.transferring.map((f: any) => ({
              name: f.name,
              percentage: f.percentage,
              bytes: f.bytes,
              size: f.size,
              speed: f.speed,
              eta: f.eta
            }));
          } else {
            task.transferringFiles = [];
          }
          if (this.onUpdate) this.onUpdate();
        }
      }
    });
  }

  async init() {
    // Không cần lắng nghe sự kiện từ backend ở hàm init nữa vì constructor đã khởi tạo sẵn.
  }

  /**
   * Tên hàm: enqueue
   * Mô tả: Đẩy một tác vụ (Copy/Move/Delete) vào hàng chờ. Tự động kiểm tra tính tương thích tính năng Move trên Cloud.
   */
  async enqueue(
    kind: TransferKind,
    name: string,
    src: string,
    dst: string,
    onSuccess?: () => void,
    isFallback: boolean = false,
    excludes?: string[]
  ): Promise<number> {
    if (!isFallback && kind === 'move') {
      const caps = await checkTransferCapability(src, dst);
      const canMove = caps.canMove;
      const canCopyDelete = caps.canCopyDelete;

      // Xử lý khi Cloud KHÔNG HỖ TRỢ lệnh Move nguyên bản (Server-side Move)
      if (!canMove) {
        // Hiện thông báo cảnh báo (Fallback Modal) để hỏi ý kiến người dùng
        const modal = new FallbackModal(canCopyDelete, "Nguồn", "Đích");
        const action = await modal.open();
        
        // Nếu người dùng chọn dùng Copy sau đó Delete (Server-side fallback)
        if (action === 'copy_delete') {
          await this.enqueue('copy', '[Copy] ' + name, src, dst, async () => {
            await this.enqueue('delete', '[Xoá gốc] ' + name, src, dst);
          }, true, excludes);
          return -1;
        } 
        // Nếu chọn tải xuống máy sau đó tải lên (Local Bandwidth Fallback)
        else if (action === 'local_transfer') {
          await this.enqueue('copy', '[Local Copy] ' + name, src, dst, async () => {
            await this.enqueue('delete', '[Local Xoá gốc] ' + name, src, dst);
          }, true, excludes);
          return -1;
        } 
        // Nếu người dùng ấn nút Hủy
        else {
          console.log(`Đã huỷ Move: Bỏ qua move file ${name}`);
          return -1;
        }
      }
    }

    const id = this.nextId++;

    this.tasks.set(id, {
      id,
      kind,
      name,
      src,
      dst,
      status: 'queued',
      progress: 0,
      bytesDone: 0,
      totalBytes: 0,
      speed: 0,
      lastUpdateTime: performance.now(),
      lastBytesDone: 0,
      srcLocal: !src.includes('::'),
      dstLocal: !dst.includes('::'),
      onSuccess,
      excludes
    });
    
    // Bắn sự kiện cập nhật giao diện (UI Update)
    this.notify();
    // Kích hoạt xử lý hàng đợi chạy nền (không await)
    this.processQueue(); 
    window.dispatchEvent(new CustomEvent('open-transfer-drawer'));
    return id;
  }

  /** Tên hàm: processQueue | Mô tả: Vòng lặp chính xử lý các tác vụ trong hàng đợi một cách tuần tự (Chạy nền) */
  private async processQueue() {
    if (this.isProcessing) return;
    this.isProcessing = true;

    try {
      for (const [_id, task] of this.tasks.entries()) {
        if (task.status === 'queued') {
          task.status = 'running';
          task.progress = 0.1; // Khởi tạo thanh tiến trình ảo ở mức 10%
          this.notify();

          try {
            // Lựa chọn lệnh thực thi dựa trên loại task
            if (task.kind === 'move') {
              await fileOps.moveLocal(task.src, task.dst, task.id, task.excludes);
              // Xoá file nguồn vì đây là thao tác di chuyển
              await fsDelete(task.src);
            } else if (task.kind === 'delete') {
              await fsDelete(task.src);
            } else {
              await fileOps.cpLocal(task.src, task.dst, true, task.id, task.excludes);
            }

            // Đánh dấu hoàn tất
            task.status = 'done';
            task.progress = 1.0;

            if (task.onSuccess) {
              task.onSuccess();
            }
            
            // Ghi nhận lịch sử vào Undo Manager (hỗ trợ Ctrl+Z)
            if (task.kind === 'move' || task.kind === 'copy') {
              const destPath = joinPath(task.dst, task.name);
              undoManager.push({
                type: task.kind,
                src: task.src,
                dest: destPath,
                account: (task.srcLocal && task.dstLocal) ? undefined : appState.auth?.user,
                isLocal: task.srcLocal && task.dstLocal
              });
            }
          } catch (e: any) {
            // Ghi nhận lỗi nếu thao tác thất bại
            task.status = 'error';
            task.error = e?.toString() || 'Lỗi không xác định';
          }
          
          this.notify();
        }
      }
    } finally {
      this.isProcessing = false;
      
      // Khi toàn bộ hàng đợi đã xử lý xong, bắn sự kiện (event trigger) để UI Panel tự động tải lại file
      if (this.onQueueEmptyListeners.length > 0) {
        this.onQueueEmptyListeners.forEach(fn => fn());
      }
    }
  }

  /** Tên hàm: cancel | Mô tả: Yêu cầu Backend hủy (Kill PID) một tác vụ đang chạy qua ID */
  async cancel(id: number) {
    const task = this.tasks.get(id);
    if (task && (task.status === 'queued' || task.status === 'running')) {
      task.status = 'cancelled';
      
      // Gửi tín hiệu xuống Backend Tauri để giết tiến trình ngầm
      fsCancel(id).catch(err => console.error("Lỗi khi cancel task:", err));

      if (this.onUpdate) this.onUpdate();
    }
  }

  /** Tên hàm: cancelAll | Mô tả: Hủy tất cả các tác vụ còn đang kẹt trong hàng đợi (Queued) */
  async cancelAll() {
    for (const task of this.tasks.values()) {
      if (task.status === 'queued') {
        task.status = 'cancelled';
        task.error = 'Đã huỷ bởi người dùng';
      }
    }
    this.notify();
  }

  /** Tên hàm: removeFinished | Mô tả: Loại bỏ (Clear) các task đã Done hoặc Cancel ra khỏi UI Box */
  async removeFinished() {
    for (const [id, task] of this.tasks.entries()) {
      if (task.status === 'done' || task.status === 'cancelled' || task.status === 'error') {
        this.tasks.delete(id);
      }
    }
    this.notify();
  }

  private notify() {
    if (this.onUpdate) {
      this.onUpdate();
    }
  }

  addQueueEmptyListener(fn: () => void) {
    this.onQueueEmptyListeners.push(fn);
  }

  getAllTasks(): TransferTask[] {
    return Array.from(this.tasks.values());
  }

  destroy() {
    // Cleanup
  }
}

export const transferManager = new TransferManager();
