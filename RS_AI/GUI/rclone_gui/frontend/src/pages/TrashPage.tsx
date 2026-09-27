/*
[INTEGRITY NOTES]
- Mục đích: Trang quản trị Thùng rác (Trash View) cho cả Local và Remote Cloud.
- Trách nhiệm: Liệt kê các mục bị xoá tạm thời, khôi phục mục về chỗ cũ hoặc dọn sạch vĩnh viễn.
- Tương tác: Dùng `useTrashStore` và `useRemotesStore`.
*/

import {
  File,
  Folder,
  HardDrive,
  Laptop,
  RotateCcw,
  Trash2,
  XCircle,
} from 'lucide-react';
import React, { useEffect, useState } from 'react';
import { useRemotesStore } from '../store/useRemotesStore';
import { useTrashStore } from '../store/useTrashStore';
import { formatBytes, formatDate } from '../utils/formatters';
import { useTranslation } from '../utils/i18n';

export const TrashPage: React.FC = () => {
  const { t } = useTranslation();
  const [tab, setTab] = useState<'local' | 'remote'>('local');

  const remotes = useRemotesStore((state) => state.remotes);
  const loadRemotes = useRemotesStore((state) => state.loadRemotes);

  const localItems = useTrashStore((state) => state.localItems);
  const remoteItems = useTrashStore((state) => state.remoteItems);
  const selectedRemote = useTrashStore((state) => state.selectedRemote);
  const setSelectedRemote = useTrashStore((state) => state.setSelectedRemote);
  const loadLocalTrash = useTrashStore((state) => state.loadLocalTrash);
  const restoreLocal = useTrashStore((state) => state.restoreLocal);
  const deleteLocal = useTrashStore((state) => state.deleteLocal);
  const emptyLocal = useTrashStore((state) => state.emptyLocal);
  const loadRemoteTrash = useTrashStore((state) => state.loadRemoteTrash);
  const restoreRemote = useTrashStore((state) => state.restoreRemote);
  const deleteRemote = useTrashStore((state) => state.deleteRemote);
  const emptyRemote = useTrashStore((state) => state.emptyRemote);
  const isLoading = useTrashStore((state) => state.isLoading);

  useEffect(() => {
    loadRemotes();
    loadLocalTrash();
  }, [loadRemotes, loadLocalTrash]);

  useEffect(() => {
    if (tab === 'remote' && remotes.length > 0 && !selectedRemote) {
      setSelectedRemote(remotes[0].name);
    }
  }, [tab, remotes, selectedRemote, setSelectedRemote]);

  const handleEmpty = async () => {
    if (tab === 'local') {
      if (confirm('Bạn có chắc chắn muốn dọn sạch thùng rác cục bộ?')) {
        await emptyLocal();
      }
    } else {
      if (confirm(`Bạn có chắc muốn dọn sạch thùng rác trên remote "${selectedRemote}"?`)) {
        await emptyRemote();
      }
    }
  };

  return (
    <div className="page-container">
      <div className="page-header">
        <div className="page-title">
          <Trash2 size={24} color="#f43f5e" />
          <span data-lang-id="trash_title">{t('trash_title', 'Thùng Rác')}</span>
        </div>

        <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
          <div style={{ display: 'flex', gap: '0.25rem', background: 'rgba(0,0,0,0.25)', padding: '0.2rem', borderRadius: '6px' }}>
            <button
              className={`btn btn-sm ${tab === 'local' ? 'btn-primary' : 'btn-secondary'}`}
              onClick={() => {
                setTab('local');
                loadLocalTrash();
              }}
            >
              <Laptop size={14} />
              <span>Thùng rác Local</span>
            </button>
            <button
              className={`btn btn-sm ${tab === 'remote' ? 'btn-primary' : 'btn-secondary'}`}
              onClick={() => {
                setTab('remote');
                if (selectedRemote) loadRemoteTrash();
              }}
            >
              <HardDrive size={14} />
              <span>Thùng rác Cloud</span>
            </button>
          </div>

          {tab === 'remote' && (
            <select
              className="input-text"
              style={{ padding: '0.35rem 0.65rem', fontSize: '0.8rem' }}
              value={selectedRemote}
              onChange={(e) => setSelectedRemote(e.target.value)}
            >
              {remotes.map((r) => (
                <option key={r.name} value={r.name}>
                  {r.name} ({r.type})
                </option>
              ))}
            </select>
          )}

          <button
            className="btn btn-danger btn-sm"
            onClick={handleEmpty}
            disabled={tab === 'local' ? localItems.length === 0 : remoteItems.length === 0}
            data-lang-id="trash_empty_button"
          >
            <Trash2 size={13} />
            <span>{t('trash_empty_button', 'Dọn sạch thùng rác')}</span>
          </button>
        </div>
      </div>

      <div className="glass-panel" style={{ flex: 1, overflowY: 'auto' }}>
        {isLoading ? (
          <div style={{ textAlign: 'center', padding: '3rem', color: 'var(--text-muted)' }}>
            Đang tải dữ liệu thùng rác...
          </div>
        ) : tab === 'local' ? (
          localItems.length === 0 ? (
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                padding: '3rem',
                color: 'var(--text-muted)',
                gap: '0.5rem',
              }}
            >
              <Trash2 size={48} color="rgba(255,255,255,0.1)" />
              <p data-lang-id="trash_is_empty">{t('trash_is_empty', 'Thùng rác trống')}</p>
            </div>
          ) : (
            <table className="file-table">
              <thead>
                <tr>
                  <th>Tên mục</th>
                  <th>Đường dẫn ban đầu</th>
                  <th style={{ width: '180px' }}>Thời gian xoá</th>
                  <th style={{ width: '160px', textAlign: 'right' }}>Thao tác</th>
                </tr>
              </thead>
              <tbody>
                {localItems.map((item) => (
                  <tr key={item.id}>
                    <td>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.45rem', fontWeight: 500 }}>
                        <File size={15} color="#94a3b8" />
                        <span style={{ color: 'var(--text-primary)' }}>{item.name}</span>
                      </div>
                    </td>
                    <td style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                      {item.original_path}
                    </td>
                    <td style={{ fontSize: '0.8rem' }}>{formatDate(item.time_deleted)}</td>
                    <td style={{ textAlign: 'right' }}>
                      <div style={{ display: 'inline-flex', gap: '0.35rem' }}>
                        <button
                          className="btn btn-secondary btn-sm"
                          title="Khôi phục về chỗ cũ"
                          onClick={() => restoreLocal(item.id)}
                        >
                          <RotateCcw size={12} color="#34d399" />
                          <span>Khôi phục</span>
                        </button>
                        <button
                          className="btn-icon"
                          title="Xoá vĩnh viễn"
                          style={{ color: 'var(--danger)' }}
                          onClick={() => {
                            if (confirm(`Xoá vĩnh viễn "${item.name}"?`)) {
                              deleteLocal(item.id);
                            }
                          }}
                        >
                          <XCircle size={14} />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )
        ) : (
          /* Remote Trash */
          remoteItems.length === 0 ? (
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                padding: '3rem',
                color: 'var(--text-muted)',
                gap: '0.5rem',
              }}
            >
              <Trash2 size={48} color="rgba(255,255,255,0.1)" />
              <p>Thùng rác đám mây trên "{selectedRemote}" rỗng hoặc remote không hỗ trợ tính năng này.</p>
            </div>
          ) : (
            <table className="file-table">
              <thead>
                <tr>
                  <th>Tên mục</th>
                  <th style={{ width: '120px' }}>Dung lượng</th>
                  <th style={{ width: '180px' }}>Ngày sửa đổi</th>
                  <th style={{ width: '160px', textAlign: 'right' }}>Thao tác</th>
                </tr>
              </thead>
              <tbody>
                {remoteItems.map((item) => (
                  <tr key={item.uuid}>
                    <td>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.45rem', fontWeight: 500 }}>
                        {item.is_dir ? <Folder size={15} color="#fbbf24" /> : <File size={15} color="#94a3b8" />}
                        <span style={{ color: 'var(--text-primary)' }}>{item.name}</span>
                      </div>
                    </td>
                    <td style={{ fontSize: '0.8rem' }}>{item.is_dir ? '—' : formatBytes(item.size)}</td>
                    <td style={{ fontSize: '0.8rem' }}>{formatDate(item.mod_time)}</td>
                    <td style={{ textAlign: 'right' }}>
                      <div style={{ display: 'inline-flex', gap: '0.35rem' }}>
                        <button
                          className="btn btn-secondary btn-sm"
                          title="Khôi phục"
                          onClick={() => restoreRemote(item.name)}
                        >
                          <RotateCcw size={12} color="#34d399" />
                          <span>Khôi phục</span>
                        </button>
                        <button
                          className="btn-icon"
                          title="Xoá vĩnh viễn"
                          style={{ color: 'var(--danger)' }}
                          onClick={() => {
                            if (confirm(`Xoá vĩnh viễn "${item.name}" trên đám mây?`)) {
                              deleteRemote(item.name);
                            }
                          }}
                        >
                          <XCircle size={14} />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )
        )}
      </div>
    </div>
  );
};
