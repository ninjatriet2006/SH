/*
[INTEGRITY NOTES]
- Mục đích: Hộp thoại tạo mới cấu hình Remote Cloud.
- Trách nhiệm: Cho phép chọn Cloud Provider (Google Drive, S3, OneDrive, Dropbox...), nhập các tham số bắt buộc và tuỳ chọn.
- Tương tác: Dùng `useRemotesStore`.
*/

import { Cloud, X } from 'lucide-react';
import React, { useState } from 'react';
import { useRemotesStore } from '../store/useRemotesStore';

interface CreateRemoteModalProps {
  onClose: () => void;
}

export const CreateRemoteModal: React.FC<CreateRemoteModalProps> = ({ onClose }) => {
  const providers = useRemotesStore((state) => state.providers);
  const createRemote = useRemotesStore((state) => state.createRemote);

  const [name, setName] = useState('');
  const [selectedProvider, setSelectedProvider] = useState(providers[0]?.Prefix || 'drive');
  const [options, setOptions] = useState<Record<string, string>>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const activeProviderInfo = providers.find((p) => p.Prefix === selectedProvider);

  const handleOptionChange = (key: string, val: string) => {
    setOptions((prev) => ({ ...prev, [key]: val }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setErrorMsg('Vui lòng nhập tên cho Remote.');
      return;
    }

    setIsSubmitting(true);
    setErrorMsg(null);
    try {
      await createRemote(name.trim(), selectedProvider, options);
      onClose();
    } catch (err) {
      setErrorMsg(`Lỗi tạo remote: ${String(err)}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-content"
        style={{ maxWidth: '580px' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <Cloud size={18} color="#818cf8" />
            <h3 style={{ fontSize: '1rem', margin: 0 }}>Thêm cấu hình Remote mới</h3>
          </div>
          <button className="btn-icon" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="modal-body" style={{ maxHeight: '60vh', overflowY: 'auto' }}>
            <div className="form-group">
              <label className="form-label">Tên Remote (viết liền không dấu):</label>
              <input
                type="text"
                className="input-text"
                placeholder="my_google_drive"
                autoFocus
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>

            <div className="form-group">
              <label className="form-label">Loại dịch vụ đám mây (Provider):</label>
              <select
                className="input-text"
                value={selectedProvider}
                onChange={(e) => {
                  setSelectedProvider(e.target.value);
                  setOptions({});
                }}
              >
                {providers.map((p) => (
                  <option key={p.Prefix} value={p.Prefix}>
                    {p.Description} ({p.Prefix})
                  </option>
                ))}
              </select>
            </div>

            {activeProviderInfo && (
              <div
                style={{
                  borderTop: '1px solid var(--border)',
                  paddingTop: '0.85rem',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '0.75rem',
                }}
              >
                <div style={{ fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)' }}>
                  Tuỳ chọn cấu hình:
                </div>

                {activeProviderInfo.Options.filter((opt) => !opt.Advanced)
                  .slice(0, 6)
                  .map((opt) => (
                    <div className="form-group" key={opt.Name} style={{ margin: 0 }}>
                      <label className="form-label">
                        {opt.Name} {opt.Required && <span style={{ color: 'var(--danger)' }}>*</span>}:
                      </label>
                      <input
                        type={opt.IsPassword ? 'password' : 'text'}
                        className="input-text"
                        placeholder={opt.Help?.substring(0, 60) || ''}
                        value={options[opt.Name] || ''}
                        onChange={(e) => handleOptionChange(opt.Name, e.target.value)}
                      />
                    </div>
                  ))}
              </div>
            )}

            {errorMsg && (
              <div
                style={{
                  marginTop: '0.75rem',
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
              {isSubmitting ? 'Đang tạo...' : 'Lưu Remote'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
