/*
[INTEGRITY NOTES]
- Mục đích: Hộp thoại chỉnh sửa thông số cấu hình Remote Cloud (Edit Remote Modal).
- Trách nhiệm: Cho phép sửa các key-value options đã lưu, cập nhật token, client ID/secret hoặc thêm cờ rclone mới cho remote.
- Tương tác: Dùng `useRemotesStore`.
*/

import { Cloud, Plus, Trash2, X } from 'lucide-react';
import React, { useState } from 'react';
import type { RemoteConfig } from '../../../bridge/types';
import { useRemotesStore } from '../store/useRemotesStore';
import { useTranslation } from '../utils/i18n';

interface EditRemoteModalProps {
  remote: RemoteConfig;
  onClose: () => void;
}

export const EditRemoteModal: React.FC<EditRemoteModalProps> = ({ remote, onClose }) => {
  const { t } = useTranslation();
  const providers = useRemotesStore((state) => state.providers);
  const updateRemote = useRemotesStore((state) => state.updateRemote);

  // Khởi tạo options từ remote hiện có (bỏ name và type)
  const initialOptions: Record<string, string> = {};
  for (const [key, value] of Object.entries(remote)) {
    if (key !== 'name' && key !== 'type' && value !== undefined && value !== null) {
      initialOptions[key] = typeof value === 'object' ? JSON.stringify(value) : String(value);
    }
  }

  const [options, setOptions] = useState<Record<string, string>>(initialOptions);
  const [newKey, setNewKey] = useState('');
  const [newVal, setNewVal] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const activeProvider = providers.find((p) => p.Prefix === remote.type);

  const handleOptionChange = (key: string, val: string) => {
    setOptions((prev) => ({ ...prev, [key]: val }));
  };

  const handleRemoveOption = (key: string) => {
    setOptions((prev) => {
      const copy = { ...prev };
      delete copy[key];
      return copy;
    });
  };

  const handleAddCustomOption = () => {
    if (!newKey.trim()) return;
    setOptions((prev) => ({ ...prev, [newKey.trim()]: newVal }));
    setNewKey('');
    setNewVal('');
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    setErrorMsg(null);
    try {
      await updateRemote(remote.name, options);
      onClose();
    } catch (err) {
      setErrorMsg(`Lỗi cập nhật remote: ${String(err)}`);
    } finally {
      setIsSubmitting(false);
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
            <Cloud size={18} color="#818cf8" />
            <h3 style={{ fontSize: '1rem', margin: 0 }} data-lang-id="remotes_modal_edit_title">
              {t('remotes_modal_edit_title', 'Chỉnh sửa cấu hình Remote')}: {remote.name}
            </h3>
          </div>
          <button className="btn-icon" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="modal-body" style={{ maxHeight: '60vh', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.75rem' }}>
              <div className="form-group" style={{ margin: 0 }}>
                <label className="form-label">Tên Remote:</label>
                <input
                  type="text"
                  className="input-text"
                  value={remote.name}
                  disabled
                  style={{ opacity: 0.7 }}
                />
              </div>

              <div className="form-group" style={{ margin: 0 }}>
                <label className="form-label">Loại đám mây (Provider):</label>
                <input
                  type="text"
                  className="input-text"
                  value={activeProvider ? `${activeProvider.Description} (${remote.type})` : remote.type}
                  disabled
                  style={{ opacity: 0.7 }}
                />
              </div>
            </div>

            <div style={{ marginTop: '0.5rem' }}>
              <h4 style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                Các tham số cấu hình:
              </h4>

              {Object.keys(options).length === 0 ? (
                <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)', fontStyle: 'italic', padding: '0.5rem' }}>
                  Chưa có tham số nào được lưu hoặc provider dùng mặc định.
                </div>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                  {Object.entries(options).map(([k, v]) => (
                    <div key={k} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                      <label style={{ width: '35%', fontSize: '0.8rem', color: 'var(--text-secondary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={k}>
                        {k}:
                      </label>
                      <input
                        type="text"
                        className="input-text"
                        style={{ flex: 1 }}
                        value={v}
                        onChange={(e) => handleOptionChange(k, e.target.value)}
                      />
                      <button
                        type="button"
                        className="btn-icon"
                        title="Xoá tham số"
                        style={{ color: 'var(--danger)' }}
                        onClick={() => handleRemoveOption(k)}
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Thêm tham số tuỳ chỉnh */}
            <div style={{ borderTop: '1px solid var(--border)', paddingTop: '0.75rem', marginTop: '0.25rem' }}>
              <h4 style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '0.4rem' }}>
                Thêm tham số mới:
              </h4>
              <div style={{ display: 'flex', gap: '0.5rem' }}>
                <input
                  type="text"
                  className="input-text"
                  placeholder="Tên tham số (VD: scope, token, team_drive)"
                  style={{ width: '40%' }}
                  value={newKey}
                  onChange={(e) => setNewKey(e.target.value)}
                />
                <input
                  type="text"
                  className="input-text"
                  placeholder="Giá trị"
                  style={{ flex: 1 }}
                  value={newVal}
                  onChange={(e) => setNewVal(e.target.value)}
                />
                <button
                  type="button"
                  className="btn btn-secondary btn-sm"
                  onClick={handleAddCustomOption}
                  disabled={!newKey.trim()}
                >
                  <Plus size={14} />
                  <span>Thêm</span>
                </button>
              </div>
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
              {isSubmitting ? 'Đang lưu...' : 'Lưu thay đổi'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
