/*
[INTEGRITY NOTES]
- Mục đích: Lớp xem mỏng (thin view) cho hàng chờ việc backend P3 — các tác vụ Copy/Move/Delete.
- Trách nhiệm: `enqueue` đặt job qua `bridge/job_api.ts`, subscribe event `job_update` để vẽ,
  `cancel` qua `job_cancel`. Dialog quyền/fallback park + rerun qua job mới (không snapshot localStorage).
- Tương tác: `bridge/job_api.ts` (job), `bridge/remote_api.ts` (capability), `fileOps` + IPC cũ
  (`fs_copy/fs_move/fs_cancel`) chỉ làm fallback khi job IPC lỗi. UI: FallbackModal, PermissionDialog,
  TransferDrawer. API cũ (`enqueue`/`cancel`/`cancelAll`/`retryFailed`/`removeFinished`) giữ nguyên
  tên để caller cũ (clipboard, DualPaneExplorer, TransferDrawer) dùng được.
*/

import { undoManager } from '../services/undoManager';
import { joinPath } from './dragDrop';
import * as fileOps from '../services/fileOps';
import { FallbackModal } from '../components/FallbackModal';
import { checkTransferCapability } from '../../../bridge/remote_api';
import { getTempDir, fsDelete, fsCancel, setPermissionPolicy, isPermissionConsentError } from '../../../bridge/explorer_api';
import { jobEnqueue, jobList, jobCancel, subscribeJobUpdates, type Job, type JobKind } from '../../../bridge/job_api';
import { PermissionDialog } from '../components/PermissionDialog';
import { debugStore } from '../services/debugStore';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type TransferKind = 'upload' | 'download' | 'copy' | 'move' | 'delete';
export type TransferStatus = 'queued' | 'running' | 'done' | 'error' | 'cancelled';

export interface TransferTask {
  id: string;
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

interface PendingCb { onSuccess?: () => void; onFail?: (e: any) => void; }

// ====================================================================================
// BLOCK: LỚP XEM MỎNG CHO JOB QUEUE BACKEND (P3)
// ====================================================================================
class TransferManager {
  public tasks: Map<string, TransferTask> = new Map();
  public onUpdate?: () => void;
  public onQueueEmptyListeners: (() => void)[] = [];
  private callbacks: Map<string, PendingCb> = new Map();
  private fallbackApplyToAllCache: { action: 'fallback_server_side' | 'fallback_local' | 'cancel', expireAt: number } | null = null;
  // P3: job đang park chờ consent + cờ tránh mở nhiều dialog cùng lúc.
  private parkedIds: string[] = [];
  private permissionDialogOpen = false;
  private subscribed = false;
  private unlistens: UnlistenFn[] = [];
  private localSeq = 1;

  constructor() {
    // Đăng ký vẽ từ event backend; fire-and-forget (init() hydrate + đảm bảo lại).
    void this.ensureSubscribed();
  }

  async init() {
    await this.ensureSubscribed();
    // Hydrate từ backend để hết mất hàng khi tải lại (thay snapshot localStorage).
    try {
      const jobs = await jobList();
      for (const job of jobs) this.upsertFromJob(job);
      this.notify();
    } catch (e) {
      console.warn('job_list hydrate fail (giữ hàng local):', e);
    }
  }

  private async ensureSubscribed(): Promise<void> {
    if (this.subscribed) return;
    this.subscribed = true;
    try {
      const un = await subscribeJobUpdates((job) => this.applyJobUpdate(job));
      this.unlistens.push(un);
    } catch (e) {
      console.warn('subscribe job_update fail:', e);
      this.subscribed = false;
    }
  }

  /**
   * Tên hàm: enqueue (giữ API cũ, delegate sang job)
   * Mô tả: Đặt tác vụ vào hàng chờ backend. Move/Copy kiểm tra capability và mở
   * FallbackModal trước khi đặt job; lỗi quyền park + PermissionDialog rồi rerun job mới.
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
  ): Promise<string> {
    // Fallback mở rộng (giữ luồng UI cũ) — các job con đặt qua queue với isFallback=true.
    if (!isFallback && (kind === 'move' || kind === 'copy')) {
      const expanded = await this.expandFallback(kind, name, src, dst, onSuccess, onFail, excludes);
      if (expanded !== null) return expanded;
    }
    return this.placeJob(kind, name, src, dst, { onSuccess, onFail }, isFallback, excludes);
  }

  /** Đặt 1 job backend; job IPC lỗi thì rớt về luồng cũ (fileOps trực tiếp). */
  private async placeJob(
    kind: TransferKind,
    name: string,
    src: string,
    dst: string,
    cb: PendingCb,
    isFallback?: boolean,
    excludes?: string[]
  ): Promise<string> {
    const jobKind: JobKind = kind === 'delete' ? 'delete' : kind === 'move' ? 'move' : 'copy';
    const normDst = kind === 'delete' ? joinPath(dst, name) : dst;
    try {
      const job = await jobEnqueue(jobKind, src, kind === 'delete' ? null : normDst);
      this.callbacks.set(job.id, cb);
      this.tasks.set(job.id, {
        id: job.id,
        kind,
        name,
        src,
        dst: normDst,
        status: this.mapStatus(job.status),
        progress: job.progress / 100,
        bytesDone: 0,
        totalBytes: 0,
        speed: 0,
        lastUpdateTime: performance.now(),
        lastBytesDone: 0,
        srcLocal: !src.includes('::'),
        dstLocal: !normDst.includes('::'),
        onSuccess: cb.onSuccess,
        onFail: cb.onFail,
        isFallback,
        excludes,
      });
      this.notify();
      window.dispatchEvent(new CustomEvent('open-transfer-drawer'));
      return job.id;
    } catch (e) {
      // Fallback luồng cũ: chạy trực tiếp qua IPC cũ khi job IPC chưa có/lỗi.
      console.warn('job_enqueue fail, fallback luồng cũ:', e);
      return this.runLegacy(kind, name, src, normDst, cb, isFallback, excludes);
    }
  }

  /** Luồng cũ (giữ nguyên để fallback): chạy trực tiếp qua fileOps/fsDelete. */
  private async runLegacy(
    kind: TransferKind,
    name: string,
    src: string,
    dst: string,
    cb: PendingCb,
    isFallback?: boolean,
    excludes?: string[]
  ): Promise<string> {
    const id = `local-${Date.now()}-${this.localSeq++}`;
    const task: TransferTask = {
      id, kind, name, src, dst, status: 'running', progress: 0.1,
      bytesDone: 0, totalBytes: 0, speed: 0,
      lastUpdateTime: performance.now(), lastBytesDone: 0,
      srcLocal: !src.includes('::'), dstLocal: !dst.includes('::'),
      onSuccess: cb.onSuccess, onFail: cb.onFail, isFallback, excludes,
    };
    this.tasks.set(id, task);
    this.callbacks.set(id, cb);
    this.notify();
    try {
      if (kind === 'move') await fileOps.moveLocal(src, dst);
      else if (kind === 'copy' || kind === 'upload' || kind === 'download') await fileOps.cpLocal(src, dst, true);
      else await fsDelete(src);
      task.status = 'done';
      task.progress = 1.0;
      this.finishSuccess(task);
    } catch (e: any) {
      if (isPermissionConsentError(e)) {
        task.status = 'error';
        task.error = e?.message ?? String(e);
        this.notify();
        await this.handlePermissionConsent(task);
        return id;
      }
      task.status = 'error';
      task.error = e?.toString() || 'Lỗi không xác định';
      cb.onFail?.(e);
    }
    this.notify();
    this.maybeQueueEmpty();
    return id;
  }

  /**
   * Mở rộng fallback cho move/copy không được backend hỗ trợ native.
   * Trả null khi không cần fallback (caller đặt job thẳng), ngược lại trả id task
   * đại diện (job con đầu tiên hoặc placeholder cancelled).
   */
  private async expandFallback(
    kind: TransferKind,
    name: string,
    src: string,
    dst: string,
    onSuccess?: () => void,
    onFail?: (e: any) => void,
    excludes?: string[]
  ): Promise<string | null> {
    let action: 'fallback_server_side' | 'fallback_local' | 'cancel' | null = null;
    let canCopyDelete = false;
    if (this.fallbackApplyToAllCache && Date.now() < this.fallbackApplyToAllCache.expireAt) {
      action = this.fallbackApplyToAllCache.action;
    } else {
      const caps = await checkTransferCapability(src, dst);
      if (kind === 'move' && !caps.canMove) {
        canCopyDelete = caps.canCopyDelete;
        const modal = new FallbackModal(canCopyDelete, 'Nguồn', 'Đích');
        const res = await modal.open();
        action = res.action as any;
        if (res.applyToAll) this.fallbackApplyToAllCache = { action: action as any, expireAt: Date.now() + 5000 };
      } else if (kind === 'copy' && !caps.canCopy) {
        const modal = new FallbackModal(false, 'Nguồn', 'Đích', false);
        const res = await modal.open();
        action = res.action as any;
      }
    }
    if (!action) return null; // backend hỗ trợ native → đặt job thẳng.
    if (action === 'cancel') return this.placeholderCancelled(kind, name, src, dst);
    if (action === 'fallback_server_side' && kind === 'move') {
      // Copy rồi xoá gốc — 2 job nối nhau qua onSuccess.
      return this.enqueue('copy', '[Move: Copy] ' + name, src, dst, async () => {
        await this.enqueue('delete', '[Move: Xoá gốc] ' + name, src, dst, async () => {
          undoManager.push({ type: 'move', src, dest: joinPath(dst, name), isLocal: !src.includes('::') && !dst.includes('::') });
          onSuccess?.();
        }, onFail, true);
      }, onFail, true, excludes);
    }
    // fallback_local: relay qua thư mục tạm Local (download → upload → dọn temp → xoá gốc nếu move).
    const sysTemp = await getTempDir();
    const tempFolder = joinPath(`Local::${sysTemp}`, `rclone_gui_temp_${Date.now()}_${Math.floor(Math.random() * 1000)}`);
    debugStore.log('TRANSFER', 'Create Temp Folder', { path: tempFolder, for: name });
    const cleanupTemp = () => {
      void this.enqueue('delete', `[${kind === 'move' ? 'Move' : 'Copy'}: Dọn Temp lỗi] ` + name, tempFolder, dst, undefined, undefined, true);
    };
    const tailAfterUpload = async (): Promise<void> => {
      await this.enqueue('delete', `[${kind === 'move' ? 'Move' : 'Copy'}: Dọn Temp] ` + name, tempFolder, dst, async () => {
        debugStore.log('TRANSFER', 'Clean Temp Folder', { path: tempFolder, for: name });
        if (kind === 'move') {
          await this.enqueue('delete', '[Move: Xoá gốc] ' + name, src, dst, async () => {
            undoManager.push({ type: 'move', src, dest: joinPath(dst, name), isLocal: false });
            onSuccess?.();
          }, onFail, true);
        } else {
          undoManager.push({ type: 'copy', src, dest: joinPath(dst, name), isLocal: false });
          onSuccess?.();
        }
      }, undefined, true);
    };
    await this.enqueue('copy', `[${kind === 'move' ? 'Move' : 'Copy'}: Download Tạm] ` + name, src, tempFolder, async () => {
      await this.enqueue('copy', `[${kind === 'move' ? 'Move' : 'Copy'}: Upload Lên] ` + name, tempFolder, dst, () => { void tailAfterUpload(); }, () => cleanupTemp(), true, excludes);
    }, () => cleanupTemp(), true, excludes);
    // Task đại diện: job download-tạm vừa đặt (tìm theo tên để trả id cho caller).
    const rep = Array.from(this.tasks.values()).find((t) => t.name === `[${kind === 'move' ? 'Move' : 'Copy'}: Download Tạm] ` + name);
    return rep ? rep.id : '';
  }

  private placeholderCancelled(kind: TransferKind, name: string, src: string, dst: string): string {
    const id = `local-${Date.now()}-${this.localSeq++}`;
    this.tasks.set(id, {
      id, kind, name, src, dst, status: 'cancelled', progress: 1.0,
      bytesDone: 0, totalBytes: 0, speed: 0,
      lastUpdateTime: performance.now(), lastBytesDone: 0,
      srcLocal: !src.includes('::'), dstLocal: !dst.includes('::'),
    });
    this.notify();
    return id;
  }

  /** Vẽ lại từ event `job_update`; xử lý done/error/cancel + park quyền. */
  private applyJobUpdate(job: Job): void {
    let task = this.tasks.get(job.id);
    if (!task) {
      task = {
        id: job.id, kind: job.kind === 'move' ? 'move' : job.kind === 'delete' ? 'delete' : 'copy',
        name: job.src ?? job.id, src: job.src ?? '', dst: job.dst ?? '',
        status: 'queued', progress: 0, bytesDone: 0, totalBytes: 0, speed: 0,
        lastUpdateTime: performance.now(), lastBytesDone: 0,
        srcLocal: !(job.src ?? '').includes('::'), dstLocal: !(job.dst ?? '').includes('::'),
      };
      const cb = this.callbacks.get(job.id);
      task.onSuccess = cb?.onSuccess;
      task.onFail = cb?.onFail;
      this.tasks.set(job.id, task);
    }
    const prev = task.status;
    task.status = this.mapStatus(job.status);
    task.progress = job.progress / 100;
    if (job.status === 'error') {
      task.error = job.error ?? 'Lỗi không xác định';
      if (prev !== 'error' && isPermissionConsentError(job.error ?? '')) {
        void this.handlePermissionConsent(task);
        this.notify();
        return;
      }
      if (prev !== 'error') this.callbacks.get(job.id)?.onFail?.(job.error ?? 'Lỗi không xác định');
    } else if (job.status === 'done') {
      task.error = undefined;
      if (prev !== 'done') this.finishSuccess(task);
    }
    this.notify();
    this.maybeQueueEmpty();
  }

  private upsertFromJob(job: Job): void {
    if (this.tasks.has(job.id)) {
      this.applyJobUpdate(job);
      return;
    }
    this.tasks.set(job.id, {
      id: job.id, kind: job.kind === 'move' ? 'move' : job.kind === 'delete' ? 'delete' : 'copy',
      name: job.src ?? job.id, src: job.src ?? '', dst: job.dst ?? '',
      status: this.mapStatus(job.status), progress: job.progress / 100,
      bytesDone: 0, totalBytes: 0, speed: 0,
      lastUpdateTime: performance.now(), lastBytesDone: 0,
      srcLocal: !(job.src ?? '').includes('::'), dstLocal: !(job.dst ?? '').includes('::'),
      error: job.error ?? undefined,
    });
  }

  private finishSuccess(task: TransferTask): void {
    if (!task.isFallback && (task.kind === 'move' || task.kind === 'copy')) {
      undoManager.push({
        type: task.kind,
        src: task.src,
        dest: joinPath(task.dst, task.name),
        isLocal: task.srcLocal && task.dstLocal,
      });
    }
    this.callbacks.get(task.id)?.onSuccess?.();
  }

  private mapStatus(s: Job['status']): TransferStatus {
    return s === 'queued' || s === 'running' || s === 'done' || s === 'error' ? s : 'cancelled';
  }

  /** Tên hàm: cancel (giữ API cũ) | job_cancel hủy; id số cũ rớt về fs_cancel. */
  async cancel(id: string | number) {
    const key = String(id);
    const task = this.tasks.get(key);
    if (task && (task.status === 'queued' || task.status === 'running')) {
      task.status = 'cancelled';
      this.notify();
      try {
        await jobCancel(key);
      } catch (err) {
        // IPC cũ: id số legacy hoặc backend chưa có job → fs_cancel.
        const numeric = typeof id === 'number' ? id : Number(id);
        if (Number.isFinite(numeric)) {
          fsCancel(numeric).catch((e) => console.error('Lỗi khi cancel task:', e));
        } else {
          console.error('Lỗi khi cancel job:', err);
        }
      }
      if (this.onUpdate) this.onUpdate();
    }
  }

  /** Tên hàm: cancelAll | Hủy mọi job còn queued/running qua job_cancel. */
  async cancelAll() {
    const pending = Array.from(this.tasks.values()).filter((t) => t.status === 'queued' || t.status === 'running');
    for (const task of pending) {
      task.status = 'cancelled';
      task.error = 'Đã huỷ bởi người dùng';
      try {
        await jobCancel(task.id);
      } catch {
        // job local/legacy hoặc đã kết thúc ở backend — giữ cancelled ở UI.
      }
    }
    this.notify();
    this.maybeQueueEmpty();
  }

  /** Tên hàm: removeFinished | Dọn các task done/cancelled/error khỏi UI. */
  async removeFinished() {
    for (const [id, task] of this.tasks.entries()) {
      if (task.status === 'done' || task.status === 'cancelled' || task.status === 'error') {
        this.tasks.delete(id);
        this.callbacks.delete(id);
      }
    }
    this.notify();
  }

  /** Tên hàm: retryFailed | Rerun lỗi bằng job mới (park+rerun qua queue). */
  async retryFailed() {
    const failed = Array.from(this.tasks.values()).filter((t) => t.status === 'error');
    for (const task of failed) {
      const cb = this.callbacks.get(task.id);
      this.tasks.delete(task.id);
      this.callbacks.delete(task.id);
      this.parkedIds = this.parkedIds.filter((x) => x !== task.id);
      if (isPermissionConsentError(task.error ?? '')) {
        // Lỗi quyền: đi qua dialog park 1 lần thay vì rerun mù.
        if (!this.parkedIds.includes(task.id)) this.parkedIds.push(task.id);
        this.tasks.set(task.id, task);
        if (cb) this.callbacks.set(task.id, cb);
        task.status = 'error';
        void this.handlePermissionConsent(task);
        continue;
      }
      task.status = 'queued';
      task.error = undefined;
      task.progress = 0;
      await this.placeJob(task.kind, task.name, task.src, task.dst, { onSuccess: cb?.onSuccess ?? task.onSuccess, onFail: cb?.onFail ?? task.onFail }, task.isFallback, task.excludes);
    }
    this.notify();
  }

  private notify() {
    if (this.onUpdate) {
      this.onUpdate();
    }
  }

  private maybeQueueEmpty() {
    const busy = Array.from(this.tasks.values()).some((t) => t.status === 'queued' || t.status === 'running');
    if (!busy && this.onQueueEmptyListeners.length > 0) {
      this.onQueueEmptyListeners.forEach((fn) => {
        try { fn(); } catch (e) { console.error('queue-empty listener fail:', e); }
      });
    }
  }

  /**
   * P3: park job dính `PERMISSION_CONSENT`, hiện dialog consent duy nhất rồi
   * rerun các job parked bằng job mới (`once` rerun 1 vòng, `always`
   * set policy `allow_system` rồi rerun, `never` set `deny` và thôi).
   */
  private async handlePermissionConsent(failedTask: TransferTask): Promise<void> {
    if (!this.parkedIds.includes(failedTask.id)) this.parkedIds.push(failedTask.id);
    if (this.permissionDialogOpen) return;
    this.permissionDialogOpen = true;
    try {
      const detail = failedTask.error ?? 'Thiếu quyền truy cập.';
      const choice = await new PermissionDialog(failedTask.name, detail).open();
      if (choice === 'never') {
        await setPermissionPolicy('deny').catch(() => undefined);
        for (const id of this.parkedIds) {
          const t = this.tasks.get(id);
          if (t && t.status === 'error') this.callbacks.get(id)?.onFail?.(t.error ?? 'Bị từ chối quyền');
        }
        this.parkedIds = [];
        this.notify();
        return;
      }
      if (choice === 'always') {
        await setPermissionPolicy('allow_system').catch(() => undefined);
      }
      // `once`: không đổi policy bền — backend mặc định AskOnce nên chỉ rerun;
      // vòng rerun này nếu vẫn lỗi sẽ park lại và hỏi tiếp.
      const rerun = [...this.parkedIds];
      this.parkedIds = [];
      for (const id of rerun) {
        const t = this.tasks.get(id);
        if (!t || t.status !== 'error') continue;
        const cb = this.callbacks.get(id);
        this.tasks.delete(id);
        this.callbacks.delete(id);
        await this.placeJob(t.kind, t.name, t.src, t.dst, { onSuccess: cb?.onSuccess ?? t.onSuccess, onFail: cb?.onFail ?? t.onFail }, t.isFallback, t.excludes);
      }
      this.notify();
    } finally {
      this.permissionDialogOpen = false;
    }
  }

  addQueueEmptyListener(fn: () => void) {
    this.onQueueEmptyListeners.push(fn);
  }

  getAllTasks(): TransferTask[] {
    return Array.from(this.tasks.values());
  }

  destroy() {
    for (const un of this.unlistens) {
      try { un(); } catch { /* best-effort */ }
    }
    this.unlistens = [];
  }
}

export const transferManager = new TransferManager();
