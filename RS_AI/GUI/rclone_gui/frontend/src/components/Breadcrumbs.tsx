/*
[INTEGRITY NOTES]
- Mục đích: Thanh Breadcrumb hiển thị và chỉnh sửa đường dẫn thư mục trực tiếp.
- Trách nhiệm: Hỗ trợ nhấn vào từng đoạn đường dẫn, hoặc gõ nhập đường dẫn tự do và nhấn Enter.
- Tương tác: Dùng `useExplorerStore`.
*/

import { Folder, HardDrive } from 'lucide-react';
import React, { useState } from 'react';
import { type PaneType, useExplorerStore } from '../store/useExplorerStore';

interface BreadcrumbsProps {
  pane: PaneType;
}

export const Breadcrumbs: React.FC<BreadcrumbsProps> = ({ pane }) => {
  const currentPath = useExplorerStore((state) => state[pane].path);
  const loadDirectory = useExplorerStore((state) => state.loadDirectory);
  const [isEditing, setIsEditing] = useState(false);
  const [inputVal, setInputVal] = useState(currentPath);

  const [prevPath, setPrevPath] = useState(currentPath);

  if (prevPath !== currentPath) {
    setPrevPath(currentPath);
    setInputVal(currentPath);
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      setIsEditing(false);
      if (inputVal.trim() && inputVal !== currentPath) {
        loadDirectory(pane, inputVal.trim());
      }
    } else if (e.key === 'Escape') {
      setIsEditing(false);
      setInputVal(currentPath);
    }
  };

  if (isEditing) {
    return (
      <input
        type="text"
        className="input-text"
        style={{ flex: 1, padding: '0.25rem 0.5rem', fontSize: '0.8rem' }}
        value={inputVal}
        autoFocus
        onChange={(e) => setInputVal(e.target.value)}
        onKeyDown={handleKeyDown}
        onBlur={() => {
          setIsEditing(false);
          setInputVal(currentPath);
        }}
      />
    );
  }

  // Phân tích các đoạn path
  const isRemote = currentPath.includes('::');
  let segments: Array<{ name: string; fullPath: string }> = [];

  if (isRemote) {
    const [remoteName, relPath] = currentPath.split('::');
    segments.push({ name: `${remoteName}:`, fullPath: `${remoteName}::/` });

    const parts = (relPath || '').split('/').filter(Boolean);
    let accumulated = '';
    for (const part of parts) {
      accumulated += `/${part}`;
      segments.push({ name: part, fullPath: `${remoteName}::${accumulated}` });
    }
  } else {
    const parts = currentPath.split('/').filter(Boolean);
    segments.push({ name: '/', fullPath: '/' });
    let accumulated = '';
    for (const part of parts) {
      accumulated += `/${part}`;
      segments.push({ name: part, fullPath: accumulated });
    }
  }

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '0.25rem',
        flex: 1,
        overflow: 'hidden',
        cursor: 'text',
        padding: '0.2rem 0.4rem',
        borderRadius: '6px',
        background: 'rgba(0,0,0,0.2)',
      }}
      onClick={() => setIsEditing(true)}
      title="Nhấn để nhập đường dẫn"
    >
      {isRemote ? (
        <HardDrive size={14} color="#38bdf8" style={{ flexShrink: 0 }} />
      ) : (
        <Folder size={14} color="#818cf8" style={{ flexShrink: 0 }} />
      )}

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '0.2rem',
          overflow: 'hidden',
          whiteSpace: 'nowrap',
          textOverflow: 'ellipsis',
        }}
      >
        {segments.map((seg, idx) => (
          <React.Fragment key={seg.fullPath}>
            <span
              style={{
                fontSize: '0.8rem',
                color: idx === segments.length - 1 ? 'var(--text-primary)' : 'var(--text-secondary)',
                cursor: 'pointer',
                fontWeight: idx === segments.length - 1 ? 600 : 400,
              }}
              onClick={(e) => {
                e.stopPropagation();
                loadDirectory(pane, seg.fullPath);
              }}
            >
              {seg.name}
            </span>
            {idx < segments.length - 1 && (
              <span style={{ color: 'var(--text-muted)', fontSize: '0.75rem' }}>/</span>
            )}
          </React.Fragment>
        ))}
      </div>
    </div>
  );
};
