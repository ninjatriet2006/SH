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
  ChevronDown,
  ChevronRight,
  ChevronsUp,
  Clock,
  File,
  Folder,
  RefreshCw,
  XCircle,
} from 'lucide-react';
import React, { useEffect, useState } from 'react';
import { jobGetChildren } from '../../../bridge/jobs_bridge';
import type { Job, JobStatus, QueueItem } from '../../../bridge/types';
import { useJobsStore } from '../store/useJobsStore';
import { useTranslation } from '../utils/i18n';

export const TransfersPage: React.FC = () => {
  const { t } = useTranslation();
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
  const [expandedJobIds, setExpandedJobIds] = useState<Set<string>>(new Set());
  const [childrenMap, setChildrenMap] = useState<Record<string, QueueItem[]>>({});
  const [loadingChildren, setLoadingChildren] = useState<Record<string, boolean>>({});

  useEffect(() => {
    loadJobs();
    initSubscription();
  }, [loadJobs, initSubscription]);

  const fetchChildren = async (jobId: string) => {
    setLoadingChildren((prev) => ({ ...prev, [jobId]: true }));
    try {
      const kids = await jobGetChildren(jobId);
      setChildrenMap((prev) => ({ ...prev, [jobId]: kids }));
    } finally {
      setLoadingChildren((prev) => ({ ...prev, [jobId]: false }));
    }
  };

  const toggleExpandJob = (jobId: string) => {
    setExpandedJobIds((prev) => {
      const next = new Set(prev);
      if (next.has(jobId)) {
        next.delete(jobId);
      } else {
        next.add(jobId);
        fetchChildren(jobId);
      }
      return next;
    });
  };

  // Tự động làm mới danh sách vé con cho các job đang chạy
  useEffect(() => {
    const hasRunningExpanded = jobs.some(
      (j) => j.status === 'running' && expandedJobIds.has(j.id)
    );
    if (!hasRunningExpanded) return;

    const interval = setInterval(() => {
      for (const j of jobs) {
        if (j.status === 'running' && expandedJobIds.has(j.id)) {
          jobGetChildren(j.id).then((kids) => {
            setChildrenMap((prev) => ({ ...prev, [j.id]: kids }));
          });
        }
      }
    }, 1500);

    return () => clearInterval(interval);
  }, [jobs, expandedJobIds]);

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
          <span data-lang-id="transfers_title">
            {t('transfers_title', 'Hàng đợi tiến trình')} ({jobs.length})
          </span>
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
            <p data-lang-id="transfers_empty">
              {t('transfers_empty', 'Hàng đợi trống. Chưa có tác vụ nào.')}
            </p>
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
                      alignItems: 'center',
                      fontSize: '0.75rem',
                      color: 'var(--text-muted)',
                      flexWrap: 'wrap',
                      gap: '0.4rem',
                    }}
                  >
                    <span>ID: {job.id}</span>
                    {job.child_total > 0 && (
                      <button
                        className="btn btn-secondary btn-sm"
                        style={{
                          fontSize: '0.72rem',
                          padding: '0.15rem 0.45rem',
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '0.3rem',
                        }}
                        onClick={() => toggleExpandJob(job.id)}
                      >
                        {expandedJobIds.has(job.id) ? (
                          <ChevronDown size={12} />
                        ) : (
                          <ChevronRight size={12} />
                        )}
                        <span>
                          {expandedJobIds.has(job.id) ? 'Thu gọn' : 'Xem chi tiết'} mục con (
                          {job.child_done}/{job.child_total}
                          {job.skipped > 0 && `, bỏ qua: ${job.skipped}`})
                        </span>
                      </button>
                    )}
                    {job.error && (
                      <span style={{ color: '#f87171', fontWeight: 500 }}>Lỗi: {job.error}</span>
                    )}
                  </div>

                  {/* Expandable Child Jobs Sub-list */}
                  {expandedJobIds.has(job.id) && (
                    <div
                      style={{
                        marginTop: '0.4rem',
                        background: 'rgba(0,0,0,0.3)',
                        border: '1px solid var(--border)',
                        borderRadius: '6px',
                        padding: '0.5rem',
                        maxHeight: '260px',
                        overflowY: 'auto',
                        display: 'flex',
                        flexDirection: 'column',
                        gap: '0.25rem',
                      }}
                    >
                      {loadingChildren[job.id] && !childrenMap[job.id]?.length ? (
                        <div
                          style={{
                            fontSize: '0.75rem',
                            color: 'var(--text-muted)',
                            textAlign: 'center',
                            padding: '0.5rem',
                          }}
                        >
                          Đang tải danh sách mục con...
                        </div>
                      ) : !childrenMap[job.id] || childrenMap[job.id].length === 0 ? (
                        <div
                          style={{
                            fontSize: '0.75rem',
                            color: 'var(--text-muted)',
                            textAlign: 'center',
                            padding: '0.5rem',
                          }}
                        >
                          Chưa có thông tin vé con hoặc đã được dọn dẹp.
                        </div>
                      ) : (
                        childrenMap[job.id].map((kid, idx) => (
                          <div
                            key={`${kid.job_id}-${kid.path}-${idx}`}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              justifyContent: 'space-between',
                              padding: '0.25rem 0.5rem',
                              borderRadius: '4px',
                              background:
                                kid.status === 'running'
                                  ? 'rgba(99, 102, 241, 0.15)'
                                  : 'rgba(255,255,255,0.02)',
                              fontSize: '0.74rem',
                              fontFamily: 'var(--font-mono)',
                            }}
                          >
                            <div
                              style={{
                                display: 'flex',
                                alignItems: 'center',
                                gap: '0.4rem',
                                overflow: 'hidden',
                                textOverflow: 'ellipsis',
                                whiteSpace: 'nowrap',
                              }}
                            >
                              {kid.is_dir ? (
                                <Folder size={12} color="#818cf8" style={{ flexShrink: 0 }} />
                              ) : (
                                <File size={12} color="#94a3b8" style={{ flexShrink: 0 }} />
                              )}
                              <span
                                style={{
                                  color:
                                    kid.status === 'done'
                                      ? 'var(--text-secondary)'
                                      : 'var(--text-primary)',
                                }}
                                title={kid.path}
                              >
                                {kid.path || '(Gốc thư mục)'}
                              </span>
                            </div>

                            <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', flexShrink: 0 }}>
                              {kid.status === 'done' && (
                                <span style={{ color: '#34d399', fontWeight: 500 }}>✓ Xong</span>
                              )}
                              {kid.status === 'running' && (
                                <span style={{ color: '#818cf8', display: 'flex', alignItems: 'center', gap: '0.25rem', fontWeight: 500 }}>
                                  <RefreshCw size={10} className="spin" /> Đang chép
                                </span>
                              )}
                              {kid.status === 'queued' && (
                                <span style={{ color: '#94a3b8' }}>Chờ</span>
                              )}
                              {kid.status === 'cancelled' && (
                                <span style={{ color: '#f59e0b' }}>Bỏ qua</span>
                              )}
                              {kid.status === 'error' && (
                                <span style={{ color: '#f87171' }}>✕ Lỗi: {kid.error || ''}</span>
                              )}
                            </div>
                          </div>
                        ))
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
