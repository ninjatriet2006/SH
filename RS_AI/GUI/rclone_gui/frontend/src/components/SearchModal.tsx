/*
[INTEGRITY NOTES]
- Mục đích: Hộp thoại tìm kiếm sâu đệ quy toàn bộ cây thư mục.
- Trách nhiệm: Nhận query, gọi `searchFiles`, hiển thị kết quả và điều hướng Pane tới vị trí tệp khi nhấp đúp.
- Tương tác: Dùng `files_bridge.ts` và `useExplorerStore`.
*/

import { Folder, Search, X } from 'lucide-react';
import React, { useState } from 'react';
import { searchFiles } from '../../../bridge/files_bridge';
import type { SearchResultItem } from '../../../bridge/types';
import { useExplorerStore } from '../store/useExplorerStore';
import { formatBytes, getParentPath } from '../utils/formatters';

interface SearchModalProps {
  basePath: string;
  onClose: () => void;
}

export const SearchModal: React.FC<SearchModalProps> = ({ basePath, onClose }) => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResultItem[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [searched, setSearched] = useState(false);

  const activePane = useExplorerStore((state) => state.activePane);
  const loadDirectory = useExplorerStore((state) => state.loadDirectory);

  const handleSearch = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!query.trim()) return;

    setIsSearching(true);
    setSearched(true);
    try {
      const items = await searchFiles(basePath, query.trim());
      setResults(items);
    } catch (err) {
      console.error('Lỗi tìm kiếm:', err);
    } finally {
      setIsSearching(false);
    }
  };

  const handleJumpToItem = (item: SearchResultItem) => {
    const parent = getParentPath(item.path);
    loadDirectory(activePane, parent);
    onClose();
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-content"
        style={{ maxWidth: '650px' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <Search size={18} color="#38bdf8" />
            <h3 style={{ fontSize: '1rem', margin: 0 }}>Tìm kiếm trong: {basePath}</h3>
          </div>
          <button className="btn-icon" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body">
          <form onSubmit={handleSearch} style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
            <input
              type="text"
              className="input-text"
              style={{ flex: 1 }}
              placeholder="Nhập tên tệp hoặc thư mục cần tìm..."
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <button className="btn btn-primary btn-sm" type="submit" disabled={isSearching}>
              {isSearching ? 'Đang quét...' : 'Tìm kiếm'}
            </button>
          </form>

          <div
            style={{
              maxHeight: '340px',
              overflowY: 'auto',
              border: '1px solid var(--border)',
              borderRadius: '8px',
              background: 'rgba(0,0,0,0.2)',
            }}
          >
            {isSearching ? (
              <div style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-muted)' }}>
                Đang quét cây thư mục...
              </div>
            ) : searched && results.length === 0 ? (
              <div style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-muted)' }}>
                Không tìm thấy kết quả nào phù hợp.
              </div>
            ) : (
              results.map((r) => (
                <div
                  key={r.path}
                  style={{
                    padding: '0.55rem 0.75rem',
                    borderBottom: '1px solid rgba(255,255,255,0.03)',
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                  }}
                  onClick={() => handleJumpToItem(r)}
                  title="Nhấp để chuyển tới thư mục chứa tệp này"
                >
                  <div style={{ overflow: 'hidden' }}>
                    <div style={{ fontWeight: 500, fontSize: '0.85rem', color: 'var(--text-primary)' }}>
                      {r.item.name}
                    </div>
                    <div
                      style={{
                        fontSize: '0.75rem',
                        color: 'var(--text-muted)',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {r.path}
                    </div>
                  </div>
                  <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', flexShrink: 0 }}>
                    {r.item.is_dir ? <Folder size={14} color="#fbbf24" /> : formatBytes(r.item.size)}
                  </div>
                </div>
              ))
            )}
          </div>
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
