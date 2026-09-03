/*
[INTEGRITY NOTES]
- Mục đích: Quản lý hàng đợi tiến trình (Transfer Queue) cho các tác vụ Copy, Move, Delete.
- Trách nhiệm: Lưu trữ trạng thái tiến trình, lắng nghe sự kiện từ backend (Tauri events) để cập nhật tiến độ, tốc độ, hỗ trợ hủy (cancel) tác vụ.
- Tương tác: Giao tiếp với `fileOps` (Frontend), gọi xuống backend qua IPC, hiển thị UI Fallback Modal.
*/

import { undoManager } from '../services/undoManager';
import { joinPath } from './dragDrop';
import * as fileOps from '../services/fileOps';
import { FallbackModal } from '../components/FallbackModal';
import { checkTransferCapability } from '../../../bridge/remote_api';
import { getTempDir, fsDelete, fsCancel } from '../../../bridge/explorer_api';
import { debugStore } from '../services/debugStore';
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
  onFail?: (e: any) => void;
  isFallback?: boolean;
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
  private fallbackApplyToAllCache: { action: 'fallback_server_side' | 'fallback_local' | 'cancel', expireAt: number } | null = null;

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
    onFail?: (e: any) => void,
    isFallback?: boolean,
    excludes?: string[]
  ): Promise<number> {
    // Cập nhật: Loại bỏ block if kiểm tra fallback ở đây để task move được ném ra giao diện ngay lập tức.
    // Việc thực hiện if cái move sẽ được dời vào processQueue.

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
      onFail,
      isFallback,
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
              let action: 'fallback_server_side' | 'fallback_local' | 'cancel' | null = null;
              let canCopyDelete = false;

              // Kiểm tra cache trước khi gọi API
              if (this.fallbackApplyToAllCache && Date.now() < this.fallbackApplyToAllCache.expireAt) {
                action = this.fallbackApplyToAllCache.action;
              } else {
                const caps = await checkTransferCapability(task.src, task.dst);
                if (!caps.canMove) {
                  canCopyDelete = caps.canCopyDelete;
                  const modal = new FallbackModal(canCopyDelete, "Nguồn", "Đích");
                  const res = await modal.open();
                  action = res.action as any;
                  
                  if (res.applyToAll) {
                    this.fallbackApplyToAllCache = { action: action as any, expireAt: Date.now() + 5000 };
                  }
                }
              }

              // Xử lý khi Cloud KHÔNG HỖ TRỢ lệnh Move nguyên bản
              if (action) {
                if (action === 'fallback_server_side') {
                  // Đẩy các task fallback dạng Copy/Delete ngang hàng vào hàng đợi UI
                  await this.enqueue('copy', '[Move: Copy] ' + task.name, task.src, task.dst, async () => {
                    await this.enqueue('delete', '[Move: Xoá gốc] ' + task.name, task.src, task.dst, async () => {
                      undoManager.push({
                        type: 'move',
                        src: task.src,
                        dest: joinPath(task.dst, task.name),
                        isLocal: task.srcLocal && task.dstLocal
                      });
                    }, undefined, true);
                  }, undefined, true, task.excludes);
                } 
                else if (action === 'fallback_local') {
                  const sysTemp = await getTempDir();
                  const tempFolder = joinPath(`Local::${sysTemp}`, `rclone_gui_temp_${Date.now()}_${Math.floor(Math.random() * 1000)}`);
                  debugStore.log('TRANSFER', 'Create Temp Folder', { path: tempFolder, for: task.name });
                  
                  const cleanupTemp = async () => {
                    await this.enqueue('delete', '[Move: Dọn Temp lỗi] ' + task.name, tempFolder, task.dst, undefined, undefined, true);
                  };
                  
                  await this.enqueue('copy', '[Move: Download Tạm] ' + task.name, task.src, tempFolder, async () => {
                    await this.enqueue('copy', '[Move: Upload Lên] ' + task.name, tempFolder, task.dst, async () => {
                      await this.enqueue('delete', '[Move: Dọn Temp] ' + task.name, tempFolder, task.dst, async () => {
                        debugStore.log('TRANSFER', 'Clean Temp Folder', { path: tempFolder, for: task.name });
                        await this.enqueue('delete', '[Move: Xoá gốc] ' + task.name, task.src, task.dst, async () => {
                          undoManager.push({
                            type: 'move',
                            src: task.src,
                            dest: joinPath(task.dst, task.name),
                            isLocal: task.srcLocal && task.dstLocal
                          });
                        }, undefined, true);
                      }, undefined, true);
                    }, cleanupTemp, true, task.excludes);
                  }, cleanupTemp, true, task.excludes);
                }
                
                // Đã chuyển thành các task fallback (hoặc cancel), task move gốc này coi như bị huỷ bỏ để nhường chỗ
                task.status = 'cancelled';
                task.progress = 1.0;
                this.notify();
                continue; // Bỏ qua việc thực thi lệnh rclone moveto bên dưới
              }

              // Nếu không cần fallback (hoặc người dùng chọn server-side)
              await fileOps.moveLocal(task.src, task.dst, task.id);
            } else if (task.kind === 'copy') {
              let action: 'fallback_local' | 'cancel' | null = null;
              
              const caps = await checkTransferCapability(task.src, task.dst);
              if (!caps.canCopy) {
                const modal = new FallbackModal(false, "Nguồn", "Đích", false); // isMove = false
                const res = await modal.open();
                action = res.action as any;
              }

              if (action) {
                if (action === 'fallback_local') {
                  const sysTemp = await getTempDir();
                  const tempFolder = joinPath(`Local::${sysTemp}`, `rclone_gui_temp_${Date.now()}_${Math.floor(Math.random() * 1000)}`);
                  debugStore.log('TRANSFER', 'Create Temp Folder', { path: tempFolder, for: task.name });
                  
                  const cleanupTemp = async () => {
                    await this.enqueue('delete', '[Copy: Dọn Temp lỗi] ' + task.name, tempFolder, task.dst, undefined, undefined, true);
                  };

                  await this.enqueue('copy', '[Copy: Download Tạm] ' + task.name, task.src, tempFolder, async () => {
                    await this.enqueue('copy', '[Copy: Upload Lên] ' + task.name, tempFolder, task.dst, async () => {
                      await this.enqueue('delete', '[Copy: Dọn Temp] ' + task.name, tempFolder, task.dst, async () => {
                        debugStore.log('TRANSFER', 'Clean Temp Folder', { path: tempFolder, for: task.name });
                        undoManager.push({
                          type: 'copy',
                          src: task.src,
                          dest: joinPath(task.dst, task.name),
                          isLocal: task.srcLocal && task.dstLocal
                        });
                      }, undefined, true);
                    }, cleanupTemp, true, task.excludes);
                  }, cleanupTemp, true, task.excludes);
                }
                
                task.status = 'cancelled';
                task.progress = 1.0;
                this.notify();
                continue;
              }

              await fileOps.cpLocal(task.src, task.dst, true, task.id);
            } else if (task.kind === 'delete') {
              await fsDelete(task.src);
            }

            // Đánh dấu hoàn tất — nhưng chỉ khi task chưa bị người dùng hủy.
            // `cancel()` đặt status='cancelled' và kill tiến trình rclone; lệnh
            // await ở trên khi đó vẫn resolve bình thường (không throw), nên nếu
            // không kiểm tra lại thì task bị hủy sẽ hiện "hoàn tất" và còn bị
            // đẩy vào undoManager cho một thao tác chưa từng xong.
            if ((task.status as TransferStatus) === 'cancelled') {
              this.notify();
              continue;
            }

            task.status = 'done';
            task.progress = 1.0;

            if (task.onSuccess) {
              task.onSuccess();
            }
            
            // Ghi nhận lịch sử vào Undo Manager (hỗ trợ Ctrl+Z)
            if (!task.isFallback && (task.kind === 'move' || task.kind === 'copy')) {
              const destPath = joinPath(task.dst, task.name);
              undoManager.push({
                type: task.kind,
                src: task.src,
                dest: destPath,
                isLocal: task.srcLocal && task.dstLocal
              });
            }
          } catch (e: any) {
            // Nếu người dùng đã hủy thì giữ nguyên trạng thái 'cancelled',
            // không báo lỗi (rclone bị kill có thể throw).
            if ((task.status as TransferStatus) !== 'cancelled') {
              task.status = 'error';
              task.error = e?.toString() || 'Lỗi không xác định';
              if (task.onFail) {
                task.onFail(e);
              }
            }
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

  /** Tên hàm: retryFailed | Mô tả: Thử lại tất cả các tác vụ bị lỗi */
  async retryFailed() {
    let hasRetry = false;
    for (const task of this.tasks.values()) {
      if (task.status === 'error') {
        task.status = 'queued';
        task.error = undefined;
        task.progress = 0;
        task.bytesDone = 0;
        hasRetry = true;
      }
    }
    if (hasRetry) {
      this.notify();
      this.processQueue();
    }
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
