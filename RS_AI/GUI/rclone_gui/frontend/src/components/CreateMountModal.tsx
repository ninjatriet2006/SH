/*
[INTEGRITY NOTES]
- Mục đích: Hộp thoại tạo mới dịch vụ Systemd Mount cho rclone.
- Trách nhiệm: Cho phép nhập service name, remote đích, đường dẫn mount cục bộ, chế độ VFS cache.
- Tương tác: Dùng `useMountsStore` và `useRemotesStore`.
*/

import { HardDrive, X } from 'lucide-react';
import React, { useState } from 'react';
import type { MountConfig } from '../../../bridge/types';
import { useMountsStore } from '../store/useMountsStore';
import { useRemotesStore } from '../store/useRemotesStore';

interface CreateMountModalProps {
  onClose: () => void;
}

export const CreateMountModal: React.FC<CreateMountModalProps> = ({ onClose }) => {
  const remotes = useRemotesStore((state) => state.remotes);
  const createMount = useMountsStore((state) => state.createMount);

  const [serviceName, setServiceName] = useState('');
  const [selectedRemote, setSelectedRemote] = useState(remotes[0]?.name || '');
  const [remotePath, setRemotePath] = useState('');
  const [mountPath, setMountPath] = useState('');
  const [isUserLevel, setIsUserLevel] = useState(true);
  const [vfsMode, setVfsMode] = useState('full');
  const [allowOther, setAllowOther] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!serviceName.trim() || !mountPath.trim() || !selectedRemote) {
      setErrorMsg('Vui lòng điền đầy đủ tên dịch vụ, Remote và thư mục Mount.');
      return;
    }

    setIsSubmitting(true);
    setErrorMsg(null);
    try {
      const config: MountConfig = {
        service_name: serviceName.trim().endsWith('.service')
          ? serviceName.trim()
          : `${serviceName.trim()}.service`,
        is_user_level: isUserLevel,
        remote_name: selectedRemote,
        remote_path: remotePath.trim(),
        mount_path: mountPath.trim(),
        description: `Rclone Mount Service for ${selectedRemote}`,
        vfs_cache_mode: vfsMode,
        vfs_cache_max_size: '10G',
        vfs_cache_max_age: '24h',
        dir_cache_time: '72h',
        buffer_size: '16M',
        allow_other: allowOther,
        read_only: false,
      };

      await createMount(config, true);
      onClose();
    } catch (err) {
      setErrorMsg(`Lỗi tạo mount: ${String(err)}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-content"
        style={{ maxWidth: '540px' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <HardDrive size={18} color="#38bdf8" />
            <h3 style={{ fontSize: '1rem', margin: 0 }}>Tạo dịch vụ Mount mới (Systemd)</h3>
          </div>
          <button className="btn-icon" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
            <div className="form-group" style={{ margin: 0 }}>
              <label className="form-label">Tên Systemd Service:</label>
              <input
                type="text"
                className="input-text"
                placeholder="rclone-gdrive"
                autoFocus
                value={serviceName}
                onChange={(e) => setServiceName(e.target.value)}
              />
            </div>

            <div className="form-group" style={{ margin: 0 }}>
              <label className="form-label">Chọn Remote Cloud:</label>
              <select
                className="input-text"
                value={selectedRemote}
                onChange={(e) => setSelectedRemote(e.target.value)}
              >
                {remotes.map((r) => (
                  <option key={r.name} value={r.name}>
                    {r.name} ({r.type})
                  </option>
                ))}
              </select>
            </div>

            <div className="form-group" style={{ margin: 0 }}>
              <label className="form-label">Đường dẫn trong Remote (để trống nếu mount gốc):</label>
              <input
                type="text"
                className="input-text"
                placeholder="/documents hoặc để trống"
                value={remotePath}
                onChange={(e) => setRemotePath(e.target.value)}
              />
            </div>

            <div className="form-group" style={{ margin: 0 }}>
              <label className="form-label">Thư mục Mount cục bộ (Mount Point):</label>
              <input
                type="text"
                className="input-text"
                placeholder="/home/user/GDrive"
                value={mountPath}
                onChange={(e) => setMountPath(e.target.value)}
              />
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.75rem' }}>
              <div className="form-group" style={{ margin: 0 }}>
                <label className="form-label">VFS Cache Mode:</label>
                <select
                  className="input-text"
                  value={vfsMode}
                  onChange={(e) => setVfsMode(e.target.value)}
                >
                  <option value="off">Off</option>
                  <option value="minimal">Minimal</option>
                  <option value="writes">Writes</option>
                  <option value="full">Full (Khuyên dùng)</option>
                </select>
              </div>

              <div className="form-group" style={{ margin: 0 }}>
                <label className="form-label">Cấp độ Systemd:</label>
                <select
                  className="input-text"
                  value={isUserLevel ? 'user' : 'system'}
                  onChange={(e) => setIsUserLevel(e.target.value === 'user')}
                >
                  <option value="user">User level (~/.config/systemd/user)</option>
                  <option value="system">System level (/etc/systemd/system)</option>
                </select>
              </div>
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: '0.25rem' }}>
              <input
                type="checkbox"
                id="allowOther"
                checked={allowOther}
                onChange={(e) => setAllowOther(e.target.checked)}
              />
              <label htmlFor="allowOther" style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                Bật cờ --allow-other (Cho phép user khác truy cập)
              </label>
            </div>

            {errorMsg && (
              <div
                style={{
                  padding: '0.5rem',
                  background: 'rgba(239, 68, 68, 0.15)',
                  color: '#fca5a5',
                  borderRadius: '6px',
                  fontSize: '0.8rem',
                }}
              >
                {errorMsg}
              </div>
            )}
          </div>

          <div className="modal-footer">
            <button className="btn btn-secondary btn-sm" type="button" onClick={onClose}>
              Huỷ
            </button>
            <button className="btn btn-primary btn-sm" type="submit" disabled={isSubmitting}>
              {isSubmitting ? 'Đang tạo...' : 'Tạo Service Mount'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
