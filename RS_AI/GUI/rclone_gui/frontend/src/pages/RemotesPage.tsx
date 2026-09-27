/*
[INTEGRITY NOTES]
- Mục đích: Trang quản trị và giám sát các Remote Cloud (Remotes View).
- Trách nhiệm: Hiển thị thẻ các remote đã cấu hình, dung lượng, năng lực Features (52 cờ), thêm/xoá/xuất INI.
- Tương tác: Dùng `useRemotesStore` và `useExplorerStore`.
*/

import {
  CheckCircle2,
  Cloud,
  Edit2,
  FileText,
  HardDrive,
  Info,
  Plus,
  RefreshCw,
  Trash2,
  XCircle,
} from 'lucide-react';
import React, { useEffect, useState } from 'react';
import { CreateRemoteModal } from '../components/CreateRemoteModal';
import { EditRemoteModal } from '../components/EditRemoteModal';
import { useRemotesStore } from '../store/useRemotesStore';
import { useSettingsStore } from '../store/useSettingsStore';
import type { RemoteConfig } from '../../../bridge/types';
import { formatBytes } from '../utils/formatters';
import { useTranslation } from '../utils/i18n';

export const RemotesPage: React.FC = () => {
  const { t } = useTranslation();
  const remotes = useRemotesStore((state) => state.remotes);
  const selectedRemote = useRemotesStore((state) => state.selectedRemote);
  const setSelectedRemote = useRemotesStore((state) => state.setSelectedRemote);
  const loadRemotes = useRemotesStore((state) => state.loadRemotes);
  const loadProviders = useRemotesStore((state) => state.loadProviders);
  const deleteRemote = useRemotesStore((state) => state.deleteRemote);
  const exportRemote = useSettingsStore((state) => state.exportRemote);
  const featuresMap = useRemotesStore((state) => state.featuresMap);
  const aboutMap = useRemotesStore((state) => state.aboutMap);
  const sizeMap = useRemotesStore((state) => state.sizeMap);
  const isLoading = useRemotesStore((state) => state.isLoading);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [editingRemote, setEditingRemote] = useState<RemoteConfig | null>(null);
  const [exportedIni, setExportedIni] = useState<{ name: string; content: string } | null>(null);

  useEffect(() => {
    loadRemotes();
    loadProviders();
  }, [loadRemotes, loadProviders]);

  const handleDelete = async (name: string) => {
    if (confirm(`Bạn có chắc chắn muốn xoá remote "${name}"?`)) {
      await deleteRemote(name);
    }
  };

  const handleExport = async (name: string) => {
    try {
      const ini = await exportRemote(name);
      setExportedIni({ name, content: ini });
    } catch (err) {
      console.error('Lỗi export remote:', err);
    }
  };

  const activeFeatures = selectedRemote ? featuresMap[selectedRemote] : null;
  const activeAbout = selectedRemote ? aboutMap[selectedRemote] : null;
  const activeSize = selectedRemote ? sizeMap[selectedRemote] : null;

  return (
    <div className="page-container">
      <div className="page-header">
        <div className="page-title">
          <Cloud size={24} color="#818cf8" />
          <span data-lang-id="remotes_title">
            {t('remotes_title', 'Remote đám mây')} ({remotes.length})
          </span>
        </div>

        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <button
            className="btn btn-secondary btn-sm"
            onClick={() => loadRemotes()}
            disabled={isLoading}
            data-lang-id="remotes_refresh"
          >
            <RefreshCw size={14} />
            <span>{t('remotes_refresh', 'Làm mới')}</span>
          </button>
          <button
            className="btn btn-primary btn-sm"
            onClick={() => setShowCreateModal(true)}
            data-lang-id="remotes_add"
          >
            <Plus size={14} />
            <span>{t('remotes_add', 'Thêm Remote mới')}</span>
          </button>
        </div>
      </div>

      <div style={{ display: 'flex', gap: '1rem', flex: 1, overflow: 'hidden' }}>
        {/* Remotes Grid */}
        <div style={{ flex: 1, overflowY: 'auto' }}>
          {remotes.length === 0 ? (
            <div
              className="glass-panel"
              style={{
                alignItems: 'center',
                justifyContent: 'center',
                padding: '3rem',
                color: 'var(--text-muted)',
              }}
            >
              <Cloud size={48} color="rgba(255,255,255,0.1)" />
              <p style={{ marginTop: '1rem', fontSize: '0.9rem' }} data-lang-id="remotes_empty">
                {t('remotes_empty', 'Chưa có Remote nào được cấu hình.')}
              </p>
              <button
                className="btn btn-primary btn-sm"
                style={{ marginTop: '0.75rem' }}
                onClick={() => setShowCreateModal(true)}
                data-lang-id="remotes_add"
              >
                {t('remotes_add', 'Tạo cấu hình đầu tiên')}
              </button>
            </div>
          ) : (
            <div className="cards-grid">
              {remotes.map((remote) => {
                const isSelected = selectedRemote === remote.name;
                const abt = aboutMap[remote.name];

                return (
                  <div
                    key={remote.name}
                    className="card"
                    style={{
                      borderColor: isSelected ? 'var(--primary)' : undefined,
                      cursor: 'pointer',
                    }}
                    onClick={() => setSelectedRemote(remote.name)}
                  >
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <HardDrive size={18} color="#38bdf8" />
                        <h4 style={{ margin: 0, fontSize: '1rem', color: 'var(--text-primary)' }}>
                          {remote.name}
                        </h4>
                      </div>
                      <span className="badge badge-info">{remote.type}</span>
                    </div>

                    {abt && abt.total !== undefined ? (
                      <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                        <div>Đã dùng: {formatBytes(abt.used)} / {formatBytes(abt.total)}</div>
                        <div
                          style={{
                            height: '4px',
                            background: 'rgba(255,255,255,0.1)',
                            borderRadius: '2px',
                            marginTop: '0.35rem',
                            overflow: 'hidden',
                          }}
                        >
                          <div
                            style={{
                              height: '100%',
                              background: '#38bdf8',
                              width: `${Math.min(100, ((abt.used || 0) / abt.total) * 100)}%`,
                            }}
                          />
                        </div>
                      </div>
                    ) : (
                      <div style={{ fontSize: '0.78rem', color: 'var(--text-muted)' }}>
                        Nhấp để xem chi tiết dung lượng và cờ năng lực
                      </div>
                    )}

                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        marginTop: 'auto',
                        paddingTop: '0.5rem',
                        borderTop: '1px solid var(--border)',
                      }}
                    >
                      <button
                        className="btn btn-secondary btn-sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          setEditingRemote(remote);
                        }}
                        data-lang-id="remotes_edit"
                      >
                        <Edit2 size={12} />
                        <span>{t('remotes_edit', 'Sửa')}</span>
                      </button>

                      <div style={{ display: 'flex', gap: '0.35rem' }}>
                        <button
                          className="btn-icon"
                          title="Xuất cấu hình INI"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleExport(remote.name);
                          }}
                        >
                          <FileText size={14} />
                        </button>
                        <button
                          className="btn-icon"
                          title="Xoá Remote"
                          style={{ color: 'var(--danger)' }}
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDelete(remote.name);
                          }}
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Selected Remote Details Panel */}
        {selectedRemote && (
          <div
            className="glass-panel"
            style={{
              width: '340px',
              padding: '1rem',
              display: 'flex',
              flexDirection: 'column',
              gap: '0.85rem',
              flexShrink: 0,
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
              <Info size={16} color="#818cf8" />
              <h3 style={{ margin: 0, fontSize: '0.95rem' }}>Chi tiết: {selectedRemote}</h3>
            </div>

            {/* About / Storage Capacity */}
            <div
              style={{
                background: 'rgba(0,0,0,0.25)',
                padding: '0.75rem',
                borderRadius: '8px',
                fontSize: '0.8rem',
              }}
            >
              <div style={{ fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '0.35rem' }}>
                Dung lượng đám mây (rclone about):
              </div>
              {activeAbout && activeAbout.total !== undefined ? (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.2rem' }}>
                  <div>Tổng: {formatBytes(activeAbout.total)}</div>
                  <div>Đã sử dụng: {formatBytes(activeAbout.used)}</div>
                  <div>Còn trống: {formatBytes(activeAbout.free)}</div>
                  {activeAbout.trashed !== undefined && (
                    <div>Trong thùng rác: {formatBytes(activeAbout.trashed)}</div>
                  )}
                </div>
              ) : (
                <div style={{ color: 'var(--text-muted)' }}>Chưa có thông tin hoặc không hỗ trợ rclone about.</div>
              )}

              {activeSize && activeSize.count !== undefined && (
                <div style={{ marginTop: '0.45rem', paddingTop: '0.45rem', borderTop: '1px solid var(--border)' }}>
                  <div>Số lượng tệp: {activeSize.count}</div>
                  <div>Tổng dung lượng tệp: {formatBytes(activeSize.bytes)}</div>
                </div>
              )}
            </div>

            {/* Feature Flags (52 cờ) */}
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
              <div style={{ fontWeight: 600, fontSize: '0.8rem', color: 'var(--text-secondary)', marginBottom: '0.45rem' }}>
                Năng lực hỗ trợ (Feature Flags):
              </div>

              <div
                style={{
                  flex: 1,
                  overflowY: 'auto',
                  background: 'rgba(0,0,0,0.25)',
                  padding: '0.5rem',
                  borderRadius: '8px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '0.3rem',
                }}
              >
                {!activeFeatures ? (
                  <div style={{ color: 'var(--text-muted)', fontSize: '0.75rem', padding: '0.5rem' }}>
                    Đang nạp cờ năng lực...
                  </div>
                ) : (
                  Object.entries(activeFeatures).map(([flag, val]) => (
                    <div
                      key={flag}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        fontSize: '0.75rem',
                        padding: '0.2rem 0.35rem',
                        borderRadius: '4px',
                        background: val ? 'rgba(16, 185, 129, 0.05)' : 'transparent',
                      }}
                    >
                      <span style={{ color: val ? 'var(--text-primary)' : 'var(--text-muted)' }}>
                        {flag}
                      </span>
                      {val ? (
                        <CheckCircle2 size={12} color="#34d399" />
                      ) : (
                        <XCircle size={12} color="#64748b" />
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        )}
      </div>

      {showCreateModal && <CreateRemoteModal onClose={() => setShowCreateModal(false)} />}

      {/* Export INI Modal */}
      {exportedIni && (
        <div className="modal-overlay" onClick={() => setExportedIni(null)}>
          <div className="modal-content" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3 style={{ fontSize: '1rem', margin: 0 }}>Cấu hình INI: {exportedIni.name}</h3>
            </div>
            <div className="modal-body">
              <pre
                style={{
                  background: 'rgba(0,0,0,0.4)',
                  padding: '0.75rem',
                  borderRadius: '6px',
                  fontFamily: 'var(--font-mono)',
                  fontSize: '0.8rem',
                  color: '#34d399',
                  overflowX: 'auto',
                }}
              >
                {exportedIni.content}
              </pre>
            </div>
            <div className="modal-footer">
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => {
                  navigator.clipboard.writeText(exportedIni.content);
                  alert('Đã sao chép cấu hình vào Clipboard!');
                }}
              >
                Sao chép
              </button>
              <button className="btn btn-primary btn-sm" onClick={() => setExportedIni(null)}>
                Đóng
              </button>
            </div>
          </div>
        </div>
      )}

      {editingRemote && (
        <EditRemoteModal
          remote={editingRemote}
          onClose={() => setEditingRemote(null)}
        />
      )}
    </div>
  );
};
