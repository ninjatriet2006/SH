/*
[INTEGRITY NOTES]
- Mục đích: Bảng hiển thị danh sách tệp tin và thư mục cho từng Pane.
- Trách nhiệm: Hỗ trợ chọn đơn/đa chọn, đổi thứ tự sắp xếp, nhấp đúp để duyệt/mở, hiển thị icon phân loại file.
- Tương tác: Dùng `useExplorerStore` và `sys_bridge.ts`.
*/

import {
  Archive,
  ArrowDown,
  ArrowUp,
  File,
  FileCode,
  FileImage,
  FileMusic,
  FileText,
  FileVideo,
  Folder,
} from 'lucide-react';
import React, { useMemo } from 'react';
import { sysOpenWith } from '../../../bridge/sys_bridge';
import type { FileItem } from '../../../bridge/types';
import { type PaneType, useExplorerStore } from '../store/useExplorerStore';
import { formatBytes, formatDate, joinPath } from '../utils/formatters';

interface FileTableProps {
  pane: PaneType;
  onContextMenu?: (e: React.MouseEvent, item: FileItem) => void;
}

export const FileTable: React.FC<FileTableProps> = ({ pane, onContextMenu }) => {
  const paneState = useExplorerStore((state) => state[pane]);
  const setActivePane = useExplorerStore((state) => state.setActivePane);
  const loadDirectory = useExplorerStore((state) => state.loadDirectory);
  const toggleSelect = useExplorerStore((state) => state.toggleSelect);
  const selectAll = useExplorerStore((state) => state.selectAll);
  const clearSelection = useExplorerStore((state) => state.clearSelection);
  const setSort = useExplorerStore((state) => state.setSort);

  const { files, selectedIds, sortKey, sortDir, searchQuery, isLoading } = paneState;

  // Lọc và sắp xếp
  const processedFiles = useMemo(() => {
    let result = [...files];

    // Lọc theo search query nếu có
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      result = result.filter((f) => f.name.toLowerCase().includes(q));
    }

    // Luôn ưu tiên folder lên trên
    result.sort((a, b) => {
      if (a.is_dir !== b.is_dir) {
        return a.is_dir ? -1 : 1;
      }

      let cmp = 0;
      if (sortKey === 'name') {
        cmp = a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' });
      } else if (sortKey === 'size') {
        cmp = a.size - b.size;
      } else if (sortKey === 'mod_time') {
        cmp = (a.mod_time || '').localeCompare(b.mod_time || '');
      }

      return sortDir === 'asc' ? cmp : -cmp;
    });

    return result;
  }, [files, searchQuery, sortKey, sortDir]);

  const handleRowClick = (e: React.MouseEvent, item: FileItem) => {
    setActivePane(pane);
    const isMulti = e.ctrlKey || e.metaKey || e.shiftKey;
    toggleSelect(pane, item.uuid, isMulti);
  };

  const handleDoubleClick = (item: FileItem) => {
    setActivePane(pane);
    if (item.is_dir) {
      const targetPath = joinPath(paneState.path, item.name);
      loadDirectory(pane, targetPath);
    } else {
      // Mở file bằng ứng dụng mặc định
      const targetPath = joinPath(paneState.path, item.name);
      sysOpenWith(targetPath).catch((err) => {
        console.error(`Không thể mở file ${targetPath}:`, err);
      });
    }
  };

  const renderFileIcon = (item: FileItem) => {
    if (item.is_dir) {
      return <Folder size={16} color="#fbbf24" fill="#fbbf24" style={{ fillOpacity: 0.2 }} />;
    }
    const ext = item.name.split('.').pop()?.toLowerCase() || '';
    if (['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp', 'bmp'].includes(ext)) {
      return <FileImage size={16} color="#38bdf8" />;
    }
    if (['mp4', 'mkv', 'avi', 'mov', 'webm'].includes(ext)) {
      return <FileVideo size={16} color="#f43f5e" />;
    }
    if (['mp3', 'flac', 'wav', 'aac', 'ogg'].includes(ext)) {
      return <FileMusic size={16} color="#a855f7" />;
    }
    if (['zip', 'tar', 'gz', 'bz2', '7z', 'rar', 'xz'].includes(ext)) {
      return <Archive size={16} color="#f59e0b" />;
    }
    if (['ts', 'tsx', 'js', 'jsx', 'rs', 'py', 'json', 'toml', 'html', 'css'].includes(ext)) {
      return <FileCode size={16} color="#34d399" />;
    }
    if (['txt', 'md', 'pdf', 'doc', 'docx'].includes(ext)) {
      return <FileText size={16} color="#94a3b8" />;
    }
    return <File size={16} color="#94a3b8" />;
  };

  const allSelected = processedFiles.length > 0 && selectedIds.size === processedFiles.length;

  return (
    <div
      className="file-table-container"
      onClick={() => setActivePane(pane)}
      onContextMenu={(e) => {
        e.preventDefault();
        setActivePane(pane);
      }}
    >
      {isLoading ? (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            color: 'var(--text-secondary)',
            fontSize: '0.85rem',
          }}
        >
          Đang nạp dữ liệu...
        </div>
      ) : processedFiles.length === 0 ? (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            color: 'var(--text-muted)',
            gap: '0.5rem',
            fontSize: '0.85rem',
          }}
        >
          <Folder size={36} color="rgba(255,255,255,0.1)" />
          <span>Thư mục trống</span>
        </div>
      ) : (
        <table className="file-table">
          <thead>
            <tr>
              <th style={{ width: '36px', textAlign: 'center' }}>
                <input
                  type="checkbox"
                  checked={allSelected}
                  onChange={(e) => {
                    if (e.target.checked) selectAll(pane);
                    else clearSelection(pane);
                  }}
                />
              </th>
              <th onClick={() => setSort(pane, 'name')}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
                  <span>Tên</span>
                  {sortKey === 'name' && (
                    sortDir === 'asc' ? <ArrowUp size={12} /> : <ArrowDown size={12} />
                  )}
                </div>
              </th>
              <th style={{ width: '100px' }} onClick={() => setSort(pane, 'size')}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
                  <span>Kích thước</span>
                  {sortKey === 'size' && (
                    sortDir === 'asc' ? <ArrowUp size={12} /> : <ArrowDown size={12} />
                  )}
                </div>
              </th>
              <th style={{ width: '160px' }} onClick={() => setSort(pane, 'mod_time')}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
                  <span>Ngày sửa đổi</span>
                  {sortKey === 'mod_time' && (
                    sortDir === 'asc' ? <ArrowUp size={12} /> : <ArrowDown size={12} />
                  )}
                </div>
              </th>
            </tr>
          </thead>
          <tbody>
            {processedFiles.map((item) => {
              const isSelected = selectedIds.has(item.uuid);
              return (
                <tr
                  key={item.uuid}
                  className={`file-row ${isSelected ? 'selected' : ''}`}
                  onClick={(e) => handleRowClick(e, item)}
                  onDoubleClick={() => handleDoubleClick(item)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    if (!selectedIds.has(item.uuid)) {
                      toggleSelect(pane, item.uuid, false);
                    }
                    onContextMenu?.(e, item);
                  }}
                >
                  <td
                    style={{ textAlign: 'center' }}
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleSelect(pane, item.uuid, true);
                    }}
                  >
                    <input type="checkbox" checked={isSelected} readOnly />
                  </td>
                  <td>
                    <div className="file-icon-name">
                      {renderFileIcon(item)}
                      <span
                        style={{
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                          maxWidth: '350px',
                        }}
                      >
                        {item.name}
                      </span>
                    </div>
                  </td>
                  <td>{item.is_dir ? '—' : formatBytes(item.size)}</td>
                  <td>{formatDate(item.mod_time)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
};
