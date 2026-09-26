/*
[INTEGRITY NOTES]
- Mục đích: Quản lý hàng đợi tác vụ truyền tải / thao tác file (Job Queue).
- Trách nhiệm: Nhận tiến độ công việc real-time qua event `job_update`, điều phối ưu tiên hàng chờ.
- Tương tác: Dùng `jobs_bridge.ts` và `events_bridge.ts`.
*/

import { create } from 'zustand';
import {
  jobCancel,
  jobEnqueue,
  jobGetQueue,
  jobList,
  jobMoveDown,
  jobMoveToTop,
  jobMoveUp,
  jobReorder,
  subscribeJobUpdates,
} from '../../../bridge/jobs_bridge';
import type { Job, JobKind } from '../../../bridge/types';

interface JobsStore {
  jobs: Job[];
  queueIds: string[];
  isLoading: boolean;
  isSubscribed: boolean;

  loadJobs: () => Promise<void>;
  enqueueJob: (
    kind: JobKind,
    src?: string | null,
    dst?: string | null,
    skipPaths?: string[],
  ) => Promise<Job>;
  cancelJob: (jobId: string) => Promise<void>;
  reorderQueue: (orderedIds: string[]) => Promise<void>;
  moveJobUp: (jobId: string) => Promise<void>;
  moveJobDown: (jobId: string) => Promise<void>;
  moveJobToTop: (jobId: string) => Promise<void>;
  initSubscription: () => Promise<void>;
}

export const useJobsStore = create<JobsStore>((set, get) => ({
  jobs: [],
  queueIds: [],
  isLoading: false,
  isSubscribed: false,

  loadJobs: async () => {
    set({ isLoading: true });
    try {
      const [list, queue] = await Promise.all([
        jobList().catch(() => []),
        jobGetQueue().catch(() => []),
      ]);
      set({ jobs: list, queueIds: queue });
    } catch (err) {
      console.error('Lỗi loadJobs:', err);
    } finally {
      set({ isLoading: false });
    }
  },

  enqueueJob: async (
    kind: JobKind,
    src?: string | null,
    dst?: string | null,
    skipPaths?: string[],
  ) => {
    const job = await jobEnqueue(kind, src, dst, skipPaths);
    set((state) => ({
      jobs: [job, ...state.jobs.filter((j) => j.id !== job.id)],
    }));
    await get().loadJobs();
    return job;
  },

  cancelJob: async (jobId: string) => {
    try {
      const updated = await jobCancel(jobId);
      set((state) => ({
        jobs: state.jobs.map((j) => (j.id === jobId ? updated : j)),
      }));
      await get().loadJobs();
    } catch (err) {
      console.error(`Lỗi cancelJob ${jobId}:`, err);
    }
  },

  reorderQueue: async (orderedIds: string[]) => {
    await jobReorder(orderedIds);
    await get().loadJobs();
  },

  moveJobUp: async (jobId: string) => {
    await jobMoveUp(jobId);
    await get().loadJobs();
  },

  moveJobDown: async (jobId: string) => {
    await jobMoveDown(jobId);
    await get().loadJobs();
  },

  moveJobToTop: async (jobId: string) => {
    await jobMoveToTop(jobId);
    await get().loadJobs();
  },

  initSubscription: async () => {
    if (get().isSubscribed) return;
    try {
      await subscribeJobUpdates((updatedJob: Job) => {
        set((state) => {
          const exists = state.jobs.some((j) => j.id === updatedJob.id);
          const nextJobs = exists
            ? state.jobs.map((j) => (j.id === updatedJob.id ? updatedJob : j))
            : [updatedJob, ...state.jobs];
          return { jobs: nextJobs };
        });
      });
      set({ isSubscribed: true });
    } catch (err) {
      console.error('Lỗi subscribeJobUpdates:', err);
    }
  },
}));
