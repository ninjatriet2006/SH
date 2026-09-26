/*
[INTEGRITY NOTES]
- Mục đích: Hộp thoại hiển thị và chỉnh sửa thông số tệp tin / thư mục nâng cao (Stat, Quyền POSIX).
- Trách nhiệm: Gọi `statAdvanced`, hiển thị dung lượng, số tệp/thư mục con, cho phép đổi quyền chmod/chown.
- Tương tác: Dùng `files_bridge.ts`.
*/

import { Info, Shield, X } from 'lucide-react';
import React, { useEffect, useState } from 'react';
import { chmodPath, chownPath, statAdvanced } from '../../../bridge/files_bridge';
import type { StatInfo } from '../../../bridge/types';
import { formatBytes, getFileName } from '../utils/formatters';

interface PropertiesModalProps {
  path: string;
  onClose: () => void;
}

export const PropertiesModal: React.FC<PropertiesModalProps> = ({ path, onClose }) => {
  const [stat, setStat] = useState<StatInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [modeInput, setModeInput] = useState('');
  const [uidInput, setUidInput] = useState('');
  const [gidInput, setGidInput] = useState('');
  const [feedback, setFeedback] = useState<string | null>(null);

  const isLocal = !path.includes('::');
  const fileName = getFileName(path);

  useEffect(() => {
    let mounted = true;
    statAdvanced(path)
      .then((info) => {
        if (!mounted) return;
        setStat(info);
        if (info) {
          setModeInput((info.permissions & 0o777).toString(8).padStart(3, '0'));
          setUidInput(String(info.uid));
          setGidInput(String(info.gid));
        }
      })
      .finally(() => {
        if (mounted) setIsLoading(false);
      });
    return () => {
      mounted = false;
    };
  }, [path]);

  const handleApplyPermissions = async () => {
    try {
      const mode = parseInt(modeInput, 8);
      const uid = parseInt(uidInput, 10);
      const gid = parseInt(gidInput, 10);

      if (!isNaN(mode)) {
        await chmodPath(path, mode);
      }
      if (!isNaN(uid) && !isNaN(gid)) {
        await chownPath(path, uid, gid);
      }
      setFeedback('Đã cập nhật thuộc tính thành công!');
    } catch (err) {
      setFeedback(`Lỗi: ${String(err)}`);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <Info size={18} color="#38bdf8" />
            <h3 style={{ fontSize: '1rem', margin: 0 }}>Thuộc tính: {fileName}</h3>
          </div>
          <button className="btn-icon" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body">
          {isLoading ? (
            <div style={{ textAlign: 'center', padding: '1.5rem', color: 'var(--text-muted)' }}>
              Đang tính toán thuộc tính...
            </div>
          ) : !stat ? (
            <div style={{ color: 'var(--danger)', fontSize: '0.85rem' }}>
              Không thể đọc thuộc tính đường dẫn này.
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.85rem' }}>
              <div className="form-group">
                <span className="form-label">Đường dẫn đầy đủ:</span>
                <span
                  style={{
                    fontSize: '0.8rem',
                    color: 'var(--text-primary)',
                    wordBreak: 'break-all',
                    fontFamily: 'var(--font-mono)',
                  }}
                >
                  {path}
                </span>
              </div>

              <div
                style={{
                  display: 'grid',
                  gridTemplateColumns: '1fr 1fr',
                  gap: '0.75rem',
                  background: 'rgba(0,0,0,0.2)',
                  padding: '0.75rem',
                  borderRadius: '8px',
                }}
              >
                <div>
                  <div className="form-label">Tổng dung lượng:</div>
                  <div style={{ fontWeight: 600, color: '#38bdf8' }}>{formatBytes(stat.size)}</div>
                </div>
                <div>
                  <div className="form-label">Tệp / Thư mục con:</div>
                  <div style={{ fontWeight: 600 }}>
                    {stat.file_count} tệp, {stat.dir_count} thư mục
                  </div>
                </div>
              </div>

              {isLocal && (
                <div
                  style={{
                    borderTop: '1px solid var(--border)',
                    paddingTop: '0.75rem',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.65rem',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                    <Shield size={14} color="#818cf8" />
                    <span style={{ fontSize: '0.85rem', fontWeight: 600 }}>Quyền POSIX (Linux)</span>
                  </div>

                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '0.5rem' }}>
                    <div className="form-group" style={{ margin: 0 }}>
                      <label className="form-label">Mode (Octal):</label>
                      <input
                        type="text"
                        className="input-text"
                        value={modeInput}
                        onChange={(e) => setModeInput(e.target.value)}
                      />
                    </div>
                    <div className="form-group" style={{ margin: 0 }}>
                      <label className="form-label">UID:</label>
                      <input
                        type="number"
                        className="input-text"
                        value={uidInput}
                        onChange={(e) => setUidInput(e.target.value)}
                      />
                    </div>
                    <div className="form-group" style={{ margin: 0 }}>
                      <label className="form-label">GID:</label>
                      <input
                        type="number"
                        className="input-text"
                        value={gidInput}
                        onChange={(e) => setGidInput(e.target.value)}
                      />
                    </div>
                  </div>

                  <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '0.35rem' }}>
                    <button className="btn btn-secondary btn-sm" onClick={handleApplyPermissions}>
                      Áp dụng quyền
                    </button>
                  </div>
                </div>
              )}

              {feedback && (
                <div
                  style={{
                    padding: '0.5rem',
                    borderRadius: '6px',
                    fontSize: '0.8rem',
                    background: feedback.startsWith('Lỗi')
                      ? 'rgba(239, 68, 68, 0.15)'
                      : 'rgba(16, 185, 129, 0.15)',
                    color: feedback.startsWith('Lỗi') ? '#fca5a5' : '#6ee7b7',
                  }}
                >
                  {feedback}
                </div>
              )}
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-secondary btn-sm" onClick={onClose}>
            Đóng
          </button>
        </div>
      </div>
    </div>
  );
};
