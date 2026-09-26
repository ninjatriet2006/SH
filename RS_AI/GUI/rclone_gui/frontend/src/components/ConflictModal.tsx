/*
[INTEGRITY NOTES]
- Mục đích: Hộp thoại giải quyết xung đột khi sao chép hoặc di chuyển tệp tin.
- Trách nhiệm: Liệt kê các tệp bị trùng lặp, cho phép người dùng chọn Ghi đè toàn bộ hoặc Bỏ qua các tệp chỉ định.
- Tương tác: Trả về mảng `skipPaths` cho hàm `jobEnqueue`.
*/

import { AlertTriangle, CheckSquare, Square, X } from 'lucide-react';
import React, { useState } from 'react';
import type { ConflictInfo } from '../../../bridge/types';

interface ConflictModalProps {
  conflicts: ConflictInfo[];
  onClose: () => void;
  onProceed: (skipPaths: string[]) => void;
}

export const ConflictModal: React.FC<ConflictModalProps> = ({
  conflicts,
  onClose,
  onProceed,
}) => {
  const [selectedSkips, setSelectedSkips] = useState<Set<string>>(new Set());

  const toggleSkip = (relPath: string) => {
    setSelectedSkips((prev) => {
      const next = new Set(prev);
      if (next.has(relPath)) next.delete(relPath);
      else next.add(relPath);
      return next;
    });
  };

  const handleSelectAllSkips = () => {
    if (selectedSkips.size === conflicts.length) {
      setSelectedSkips(new Set());
    } else {
      setSelectedSkips(new Set(conflicts.map((c) => c.relative_path)));
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-content"
        style={{ maxWidth: '600px' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <AlertTriangle size={18} color="#f59e0b" />
            <h3 style={{ fontSize: '1rem', margin: 0 }}>Xung đột tệp tin ({conflicts.length})</h3>
          </div>
          <button className="btn-icon" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body">
          <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '0.75rem' }}>
            Phát hiện các tệp tin đã tồn tại ở thư mục đích. Hãy chọn các tệp bạn muốn <b>Bỏ qua</b>{' '}
            (không chép đè):
          </p>

          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginBottom: '0.5rem',
            }}
          >
            <button
              className="btn btn-secondary btn-sm"
              style={{ fontSize: '0.75rem' }}
              onClick={handleSelectAllSkips}
            >
              {selectedSkips.size === conflicts.length ? 'Bỏ chọn tất cả' : 'Bỏ qua tất cả'}
            </button>
            <span style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
              Đã chọn bỏ qua: {selectedSkips.size}/{conflicts.length}
            </span>
          </div>

          <div
            style={{
              maxHeight: '300px',
              overflowY: 'auto',
              border: '1px solid var(--border)',
              borderRadius: '8px',
              background: 'rgba(0,0,0,0.2)',
            }}
          >
            {conflicts.map((c) => {
              const isSkipped = selectedSkips.has(c.relative_path);
              return (
                <div
                  key={c.relative_path}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.65rem',
                    padding: '0.5rem 0.75rem',
                    borderBottom: '1px solid rgba(255,255,255,0.03)',
                    cursor: 'pointer',
                    background: isSkipped ? 'rgba(245, 158, 11, 0.1)' : 'transparent',
                  }}
                  onClick={() => toggleSkip(c.relative_path)}
                >
                  {isSkipped ? (
                    <CheckSquare size={16} color="#f59e0b" />
                  ) : (
                    <Square size={16} color="var(--text-muted)" />
                  )}
                  <div style={{ flex: 1, overflow: 'hidden' }}>
                    <div
                      style={{
                        fontSize: '0.85rem',
                        fontWeight: 500,
                        color: 'var(--text-primary)',
                        textOverflow: 'ellipsis',
                        overflow: 'hidden',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {c.relative_path}
                    </div>
                    <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                      Đích: {c.dest_full_path}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        <div className="modal-footer">
          <button className="btn btn-secondary btn-sm" onClick={onClose}>
            Huỷ tác vụ
          </button>
          <button
            className="btn btn-primary btn-sm"
            onClick={() => onProceed(Array.from(selectedSkips))}
          >
            Tiếp tục thực thi
          </button>
        </div>
      </div>
    </div>
  );
};
