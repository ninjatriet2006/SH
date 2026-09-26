/*
[INTEGRITY NOTES]
- Mục đích: Trang quản trị và giám sát hàng đợi tiến trình (Job Transfers View).
- Trách nhiệm: Hiển thị tiến độ real-time (thanh progress, %, số vé con), điều khiển thứ tự hàng đợi, huỷ job.
- Tương tác: Dùng `useJobsStore`.
*/

import {
  AlertCircle,
  ArrowDown,
  ArrowLeftRight,
  ArrowUp,
  CheckCircle2,
  ChevronsUp,
  Clock,
  RefreshCw,
  XCircle,
} from 'lucide-react';
import React, { useEffect, useState } from 'react';
import type { Job, JobStatus } from '../../../bridge/types';
import { useJobsStore } from '../store/useJobsStore';

export const TransfersPage: React.FC = () => {
  const jobs = useJobsStore((state) => state.jobs);
  const queueIds = useJobsStore((state) => state.queueIds);
  const loadJobs = useJobsStore((state) => state.loadJobs);
  const cancelJob = useJobsStore((state) => state.cancelJob);
  const moveJobUp = useJobsStore((state) => state.moveJobUp);
  const moveJobDown = useJobsStore((state) => state.moveJobDown);
  const moveJobToTop = useJobsStore((state) => state.moveJobToTop);
  const initSubscription = useJobsStore((state) => state.initSubscription);
  const isLoading = useJobsStore((state) => state.isLoading);

  const [filter, setFilter] = useState<'all' | 'active' | 'done' | 'error'>('all');

  useEffect(() => {
    loadJobs();
    initSubscription();
  }, [loadJobs, initSubscription]);

  const filteredJobs = jobs.filter((job) => {
    if (filter === 'active') return job.status === 'running' || job.status === 'queued';
    if (filter === 'done') return job.status === 'done';
    if (filter === 'error') return job.status === 'error' || job.status === 'cancelled';
    return true;
  });

  const renderStatusBadge = (status: JobStatus) => {
    switch (status) {
      case 'running':
        return (
          <span className="badge badge-info">
            <RefreshCw size={11} className="spin" /> Running
          </span>
        );
      case 'queued':
        return (
          <span className="badge badge-warning">
            <Clock size={11} /> Queued
          </span>
        );
      case 'done':
        return (
          <span className="badge badge-success">
            <CheckCircle2 size={11} /> Done
          </span>
        );
      case 'error':
        return (
          <span className="badge badge-danger">
            <AlertCircle size={11} /> Error
          </span>
        );
      case 'cancelled':
        return (
          <span className="badge badge-danger">
            <XCircle size={11} /> Cancelled
          </span>
        );
      default:
        return <span className="badge">{status}</span>;
    }
  };

  return (
    <div className="page-container">
      <div className="page-header">
        <div className="page-title">
          <ArrowLeftRight size={24} color="#818cf8" />
          <span>Hàng đợi tiến trình ({jobs.length})</span>
        </div>

        <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
          {/* Filters */}
          <div style={{ display: 'flex', gap: '0.25rem', background: 'rgba(0,0,0,0.25)', padding: '0.2rem', borderRadius: '6px' }}>
            <button
              className={`btn btn-sm ${filter === 'all' ? 'btn-primary' : 'btn-secondary'}`}
              onClick={() => setFilter('all')}
            >
              Tất cả ({jobs.length})
            </button>
            <button
              className={`btn btn-sm ${filter === 'active' ? 'btn-primary' : 'btn-secondary'}`}
              onClick={() => setFilter('active')}
            >
              Đang chạy / Chờ ({jobs.filter((j) => j.status === 'running' || j.status === 'queued').length})
            </button>
            <button
              className={`btn btn-sm ${filter === 'done' ? 'btn-primary' : 'btn-secondary'}`}
              onClick={() => setFilter('done')}
            >
              Hoàn thành ({jobs.filter((j) => j.status === 'done').length})
            </button>
            <button
              className={`btn btn-sm ${filter === 'error' ? 'btn-primary' : 'btn-secondary'}`}
              onClick={() => setFilter('error')}
            >
              Lỗi / Huỷ ({jobs.filter((j) => j.status === 'error' || j.status === 'cancelled').length})
            </button>
          </div>

          <button className="btn btn-secondary btn-sm" onClick={() => loadJobs()} disabled={isLoading}>
            <RefreshCw size={14} />
            <span>Làm mới</span>
          </button>
        </div>
      </div>

      {/* Jobs List */}
      <div className="glass-panel" style={{ flex: 1, overflowY: 'auto' }}>
        {filteredJobs.length === 0 ? (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              height: '100%',
              padding: '3rem',
              color: 'var(--text-muted)',
              gap: '0.75rem',
            }}
          >
            <ArrowLeftRight size={48} color="rgba(255,255,255,0.1)" />
            <p>Không có tác vụ nào trong danh sách lọc.</p>
          </div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {filteredJobs.map((job: Job) => {
              const isQueued = job.status === 'queued';
              const isRunning = job.status === 'running';

              return (
                <div
                  key={job.id}
                  style={{
                    padding: '0.85rem 1.15rem',
                    borderBottom: '1px solid var(--border)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.45rem',
                    background: isRunning ? 'rgba(99, 102, 241, 0.04)' : 'transparent',
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
                      <span
                        style={{
                          fontSize: '0.8rem',
                          fontFamily: 'var(--font-mono)',
                          fontWeight: 700,
                          color: '#a5b4fc',
                          textTransform: 'uppercase',
                        }}
                      >
                        [{job.kind}]
                      </span>
                      <span
                        style={{
                          fontSize: '0.85rem',
                          fontWeight: 600,
                          color: 'var(--text-primary)',
                          fontFamily: 'var(--font-mono)',
                        }}
                      >
                        {job.src || 'N/A'} {job.dst ? ` ➔ ${job.dst}` : ''}
                      </span>
                    </div>

                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
                      {renderStatusBadge(job.status)}

                      {/* Reorder controls if queued */}
                      {isQueued && queueIds.includes(job.id) && (
                        <div style={{ display: 'flex', gap: '0.2rem' }}>
                          <button
                            className="btn-icon"
                            title="Lên đầu hàng đợi"
                            onClick={() => moveJobToTop(job.id)}
                          >
                            <ChevronsUp size={13} />
                          </button>
                          <button
                            className="btn-icon"
                            title="Lên 1 bậc"
                            onClick={() => moveJobUp(job.id)}
                          >
                            <ArrowUp size={13} />
                          </button>
                          <button
                            className="btn-icon"
                            title="Xuống 1 bậc"
                            onClick={() => moveJobDown(job.id)}
                          >
                            <ArrowDown size={13} />
                          </button>
                        </div>
                      )}

                      {(isQueued || isRunning) && (
                        <button
                          className="btn btn-danger btn-sm"
                          style={{ padding: '0.2rem 0.5rem', fontSize: '0.75rem' }}
                          onClick={() => cancelJob(job.id)}
                        >
                          Huỷ
                        </button>
                      )}
                    </div>
                  </div>

                  {/* Progress bar */}
                  {(isRunning || isQueued || job.progress > 0) && (
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', marginTop: '0.2rem' }}>
                      <div
                        style={{
                          flex: 1,
                          height: '6px',
                          background: 'rgba(255,255,255,0.08)',
                          borderRadius: '999px',
                          overflow: 'hidden',
                        }}
                      >
                        <div
                          style={{
                            height: '100%',
                            background: job.status === 'error' ? 'var(--danger)' : 'var(--primary)',
                            width: `${Math.min(100, Math.max(0, job.progress))}%`,
                            transition: 'width 0.3s ease',
                          }}
                        />
                      </div>
                      <span
                        style={{
                          fontSize: '0.75rem',
                          fontFamily: 'var(--font-mono)',
                          color: 'var(--text-secondary)',
                          width: '42px',
                          textAlign: 'right',
                        }}
                      >
                        {job.progress}%
                      </span>
                    </div>
                  )}

                  {/* Child Items Progress & Error info */}
                  <div
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      fontSize: '0.75rem',
                      color: 'var(--text-muted)',
                    }}
                  >
                    <span>ID: {job.id}</span>
                    {job.child_total > 0 && (
                      <span>
                        Mục con hoàn thành: {job.child_done} / {job.child_total}
                        {job.skipped > 0 && ` (Đã bỏ qua: ${job.skipped})`}
                      </span>
                    )}
                    {job.error && (
                      <span style={{ color: '#f87171', fontWeight: 500 }}>Lỗi: {job.error}</span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
