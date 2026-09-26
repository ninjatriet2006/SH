/*
[INTEGRITY NOTES]
- Mục đích: Trang quản lý các điểm Mount rclone thông qua Systemd (Mounts View).
- Trách nhiệm: Giám sát trạng thái FUSE, kiểm soát vòng đời service (Start, Stop, Restart, Enable, Disable), tạo mount mới.
- Tương tác: Dùng `useMountsStore`.
*/

import {
  AlertTriangle,
  HardDrive,
  Play,
  Plus,
  Power,
  RefreshCw,
  Square,
  Trash2,
} from 'lucide-react';
import React, { useEffect, useState } from 'react';
import { CreateMountModal } from '../components/CreateMountModal';
import { useMountsStore } from '../store/useMountsStore';

export const MountsPage: React.FC = () => {
  const mounts = useMountsStore((state) => state.mounts);
  const fuseInstalled = useMountsStore((state) => state.fuseInstalled);
  const loadMounts = useMountsStore((state) => state.loadMounts);
  const manageMount = useMountsStore((state) => state.manageMount);
  const deleteMount = useMountsStore((state) => state.deleteMount);
  const isLoading = useMountsStore((state) => state.isLoading);

  const [showCreateModal, setShowCreateModal] = useState(false);
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);

  useEffect(() => {
    loadMounts();
  }, [loadMounts]);

  const handleAction = async (
    serviceName: string,
    isUser: boolean,
    action: 'start' | 'stop' | 'enable' | 'disable' | 'restart',
  ) => {
    setActionInProgress(`${serviceName}-${action}`);
    try {
      await manageMount(serviceName, isUser, action, true);
    } catch (err) {
      alert(`Lỗi thực hiện ${action}: ${String(err)}`);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleDelete = async (serviceName: string, isUser: boolean) => {
    if (confirm(`Bạn có chắc muốn xoá dịch vụ Mount "${serviceName}"?`)) {
      try {
        await deleteMount(serviceName, isUser, true);
      } catch (err) {
        alert(`Lỗi xoá service: ${String(err)}`);
      }
    }
  };

  return (
    <div className="page-container">
      <div className="page-header">
        <div className="page-title">
          <HardDrive size={24} color="#38bdf8" />
          <span>Dịch vụ Mount Systemd ({mounts.length})</span>
        </div>

        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <button className="btn btn-secondary btn-sm" onClick={() => loadMounts()} disabled={isLoading}>
            <RefreshCw size={14} />
            <span>Làm mới</span>
          </button>
          <button className="btn btn-primary btn-sm" onClick={() => setShowCreateModal(true)}>
            <Plus size={14} />
            <span>Tạo Mount mới</span>
          </button>
        </div>
      </div>

      {/* FUSE Warning Banner if missing */}
      {!fuseInstalled && (
        <div
          style={{
            padding: '0.75rem 1rem',
            background: 'rgba(239, 68, 68, 0.15)',
            border: '1px solid rgba(239, 68, 68, 0.3)',
            borderRadius: '8px',
            color: '#fca5a5',
            display: 'flex',
            alignItems: 'center',
            gap: '0.65rem',
            marginBottom: '1rem',
            fontSize: '0.85rem',
          }}
        >
          <AlertTriangle size={18} />
          <span>
            <b>Cảnh báo:</b> Thư viện FUSE (fuse3/fuse) chưa được phát hiện trên hệ thống. Bạn cần cài
            đặt FUSE để có thể mount ổ đĩa rclone (ví dụ: <code>sudo apt install fuse3</code>).
          </span>
        </div>
      )}

      {/* Mounts Table */}
      <div className="glass-panel" style={{ flex: 1, overflowY: 'auto' }}>
        {mounts.length === 0 ? (
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
            <HardDrive size={48} color="rgba(255,255,255,0.1)" />
            <p>Chưa có dịch vụ rclone mount nào trên hệ thống.</p>
            <button className="btn btn-primary btn-sm" onClick={() => setShowCreateModal(true)}>
              Tạo dịch vụ Mount đầu tiên
            </button>
          </div>
        ) : (
          <table className="file-table">
            <thead>
              <tr>
                <th>Tên Service</th>
                <th style={{ width: '120px' }}>Cấp độ</th>
                <th style={{ width: '130px' }}>Trạng thái</th>
                <th style={{ width: '120px' }}>Tự khởi động</th>
                <th style={{ width: '220px', textAlign: 'right' }}>Thao tác</th>
              </tr>
            </thead>
            <tbody>
              {mounts.map((m) => {
                const isRunning = m.status === 'active' || m.status === 'running';
                const isBusy = actionInProgress?.startsWith(m.name);

                return (
                  <tr key={m.name}>
                    <td>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontWeight: 600 }}>
                        <HardDrive size={16} color={isRunning ? '#34d399' : '#94a3b8'} />
                        <span style={{ color: 'var(--text-primary)' }}>{m.name}</span>
                      </div>
                    </td>
                    <td>
                      <span className="badge badge-info">{m.is_user ? 'User' : 'System'}</span>
                    </td>
                    <td>
                      <span className={`badge ${isRunning ? 'badge-success' : 'badge-danger'}`}>
                        {isRunning ? 'Running' : m.status || 'Stopped'}
                      </span>
                    </td>
                    <td>
                      <span className={`badge ${m.enabled ? 'badge-success' : 'badge-warning'}`}>
                        {m.enabled ? 'Enabled' : 'Disabled'}
                      </span>
                    </td>
                    <td style={{ textAlign: 'right' }}>
                      <div style={{ display: 'inline-flex', gap: '0.35rem' }}>
                        {isRunning ? (
                          <button
                            className="btn btn-secondary btn-sm"
                            disabled={isBusy}
                            title="Dừng service"
                            onClick={() => handleAction(m.name, m.is_user, 'stop')}
                          >
                            <Square size={12} color="#f87171" />
                            <span>Stop</span>
                          </button>
                        ) : (
                          <button
                            className="btn btn-secondary btn-sm"
                            disabled={isBusy}
                            title="Khởi chạy service"
                            onClick={() => handleAction(m.name, m.is_user, 'start')}
                          >
                            <Play size={12} color="#34d399" />
                            <span>Start</span>
                          </button>
                        )}

                        <button
                          className="btn-icon"
                          disabled={isBusy}
                          title="Khởi động lại (Restart)"
                          onClick={() => handleAction(m.name, m.is_user, 'restart')}
                        >
                          <RefreshCw size={13} />
                        </button>

                        <button
                          className="btn-icon"
                          disabled={isBusy}
                          title={m.enabled ? 'Tắt tự chạy cùng hệ thống' : 'Bật tự chạy cùng hệ thống'}
                          onClick={() =>
                            handleAction(m.name, m.is_user, m.enabled ? 'disable' : 'enable')
                          }
                        >
                          <Power size={13} color={m.enabled ? '#34d399' : '#94a3b8'} />
                        </button>

                        <button
                          className="btn-icon"
                          disabled={isBusy}
                          title="Xoá dịch vụ mount"
                          style={{ color: 'var(--danger)' }}
                          onClick={() => handleDelete(m.name, m.is_user)}
                        >
                          <Trash2 size={13} />
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      {showCreateModal && <CreateMountModal onClose={() => setShowCreateModal(false)} />}
    </div>
  );
};
