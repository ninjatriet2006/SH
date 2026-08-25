import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { appState } from '../store';
import { undoManager } from '../services/undoManager';
import { joinPath } from './dragDrop';

export type TransferKind = 'upload' | 'download' | 'copy' | 'move';
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
  speed: number; // bytes per second
  lastUpdateTime: number;
  lastBytesDone: number;
  srcLocal: boolean;
  dstLocal: boolean;
}

class TransferManager {
  public tasks: Map<number, TransferTask> = new Map();
  private unlistenProgress?: UnlistenFn;
  private unlistenFinished?: UnlistenFn;
  public onUpdate?: () => void;

  async init() {
    this.unlistenProgress = await listen<{
      id: number;
      progress: number | null;
      bytes_done: number;
      total_bytes: number;
    }>('transfer:progress', (event) => {
      const payload = event.payload;
      const task = this.tasks.get(payload.id);
      if (task) {
        task.status = 'running';
        task.progress = payload.progress;
        task.totalBytes = payload.total_bytes;
        
        // Calculate speed
        const now = performance.now();
        const dt = (now - task.lastUpdateTime) / 1000; // in seconds
        if (dt >= 0.5) { // update speed every 500ms
          const dBytes = payload.bytes_done - task.lastBytesDone;
          task.speed = Math.max(0, dBytes / dt);
          task.lastUpdateTime = now;
          task.lastBytesDone = payload.bytes_done;
        }
        
        task.bytesDone = payload.bytes_done;
        this.notify();
      }
    });

    this.unlistenFinished = await listen<{
      id: number;
      ok: boolean;
      error: string | null;
    }>('transfer:finished', (event) => {
      const payload = event.payload;
      const task = this.tasks.get(payload.id);
      if (task) {
        if (payload.ok) {
          task.status = 'done';
          task.progress = 1.0;
          
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
        } else {
          task.status = payload.error === 'Đã huỷ' ? 'cancelled' : 'error';
          task.error = payload.error || 'Lỗi không xác định';
        }
        task.speed = 0;
        this.notify();
      }
    });
  }

  async enqueue(
    kind: TransferKind,
    name: string,
    src: string,
    dst: string,
    srcLocal: boolean,
    dstLocal: boolean,
    cleanupSrc: boolean = false
  ): Promise<void> {
    const account = appState.auth?.user;
    const id = await invoke<number>('transfer_enqueue', {
      kind,
      name,
      src,
      dst,
      srcLocal,
      dstLocal,
      cleanupSrc,
      srcPane: 0, // not really used by backend for anything critical
      dstPane: 1
    });

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
      dstLocal
    });
    
    this.notify();

    // Start immediately
    await invoke('transfer_start', { account });
  }

  async cancel(id: number) {
    await invoke('transfer_cancel', { id });
  }

  async cancelAll() {
    await invoke('transfer_cancel_all');
  }

  async removeFinished() {
    await invoke('transfer_remove_finished');
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

  getAllTasks(): TransferTask[] {
    return Array.from(this.tasks.values());
  }

  destroy() {
    if (this.unlistenProgress) this.unlistenProgress();
    if (this.unlistenFinished) this.unlistenFinished();
  }
}

export const transferManager = new TransferManager();
