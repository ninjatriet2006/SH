import { appState } from '../store';
import { undoManager } from '../services/undoManager';
import { joinPath } from './dragDrop';
import * as fileOps from '../services/fileOps';
import { FallbackModal } from '../components/FallbackModal';
import { getBackendFeatures } from '../../../bridge/remote_api';
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
}

class TransferManager {
  public tasks: Map<number, TransferTask> = new Map();
  public onUpdate?: () => void;
  public onQueueEmptyListeners: (() => void)[] = [];
  private nextId = 1;
  private isProcessing = false;

  constructor() {
    // Listen to backend transfer progress
    listen('transfer_progress', (event: any) => {
      const payload = event.payload;
      if (payload && payload.id !== undefined && payload.stats) {
        const task = Array.from(this.tasks.values()).find(t => t.id === payload.id);
        if (task && task.status === 'running') {
          task.bytesDone = payload.stats.bytes;
          task.totalBytes = payload.stats.totalBytes;
          task.speed = payload.stats.speed;
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
    // Không cần listen event từ backend nữa vì frontend tự xử lý
  }

  async enqueue(
    kind: TransferKind,
    name: string,
    src: string,
    dst: string,
    srcLocal: boolean,
    dstLocal: boolean,
    _cleanupSrc: boolean = false,
    onSuccess?: () => void
  ): Promise<void> {
    if (kind === 'move') {
      const srcParsed = fileOps.parseRemotePath(src);
      const dstParsed = fileOps.parseRemotePath(dst);
      const srcRemote = srcLocal ? 'Local' : srcParsed.remote;
      const dstRemote = dstLocal ? 'Local' : dstParsed.remote;
      
      let canMove = false;
      let canCopyDelete = false;

      if (srcRemote === dstRemote && srcRemote === 'Local') {
        canMove = true;
      } else if (srcRemote === dstRemote && srcRemote !== 'Local') {
        try {
          const feats = await getBackendFeatures(srcRemote);
          if (feats && feats.Features) {
            canMove = !!feats.Features.Move || !!feats.Features.DirMove;
            canCopyDelete = !!feats.Features.Copy && !!feats.Features.Purge;
          }
        } catch (e) {
          console.warn("Failed to get features", e);
        }
      }

      if (!canMove) {
        const modal = new FallbackModal(canCopyDelete, srcRemote, dstRemote);
        const action = await modal.open();
        if (action === 'copy_delete') {
          await this.enqueue('copy', '[Copy] ' + name, src, dst, srcLocal, dstLocal, false, async () => {
            await this.enqueue('delete', '[Xoá gốc] ' + name, src, dst, srcLocal, dstLocal);
          });
          return;
        } else if (action === 'local_transfer') {
          await this.enqueue('copy', '[Local Copy] ' + name, src, dst, srcLocal, dstLocal, false, async () => {
            await this.enqueue('delete', '[Local Xoá gốc] ' + name, src, dst, srcLocal, dstLocal);
          });
          return;
        } else {
          return; // Cancelled
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
      srcLocal,
      dstLocal,
      onSuccess
    });
    
    this.notify();
    this.processQueue(); // Không await để chạy nền
  }

  private async processQueue() {
    if (this.isProcessing) return;
    this.isProcessing = true;

    try {
      for (const [_id, task] of this.tasks.entries()) {
        if (task.status === 'queued') {
          task.status = 'running';
          task.progress = 0.1; // Fake starting progress
          this.notify();

          try {
            if (task.kind === 'move') {
              await fileOps.moveLocal(task.src, task.dst, task.id);
            } else if (task.kind === 'delete') {
              const parsed = fileOps.parseRemotePath(task.src);
              await fsDelete(parsed.remote, parsed.realPath);
            } else {
              await fileOps.cpLocal(task.src, task.dst, true, task.id);
            }

            task.status = 'done';
            task.progress = 1.0;

            if (task.onSuccess) {
              task.onSuccess();
            }
            
            // Thêm vào Undo Manager
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
            task.status = 'error';
            task.error = e?.toString() || 'Lỗi không xác định';
          }
          
          this.notify();
        }
      }
    } finally {
      this.isProcessing = false;
      
      // Khi toàn bộ hàng đợi đã chạy xong, bắn sự kiện để UI (DualPane) biết và tải lại
      if (this.onQueueEmptyListeners.length > 0) {
        this.onQueueEmptyListeners.forEach(fn => fn());
      }
    }
  }

  async cancel(id: number) {
    const task = this.tasks.get(id);
    if (task && (task.status === 'queued' || task.status === 'running')) {
      task.status = 'cancelled';
      
      // Call backend to kill the actual rclone process
      fsCancel(id).catch(err => console.error("Lỗi khi cancel task:", err));

      if (this.onUpdate) this.onUpdate();
    }
  }

  async cancelAll() {
    for (const task of this.tasks.values()) {
      if (task.status === 'queued') {
        task.status = 'cancelled';
        task.error = 'Đã huỷ';
      }
    }
    this.notify();
  }

  async removeFinished() {
    for (const [id, task] of this.tasks.entries()) {
      if (task.status === 'done' || task.status === 'cancelled') {
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
