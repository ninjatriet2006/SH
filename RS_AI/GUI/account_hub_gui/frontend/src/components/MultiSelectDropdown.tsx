import React, { useState, useRef, useEffect } from 'react';

export interface MultiSelectOption {
  id: string;
  name: string;
}

interface MultiSelectDropdownProps {
  placeholder: string;
  options: MultiSelectOption[];
  selectedIds: string[];
  onChange: (ids: string[]) => void;
  emptyHint?: string;
}

export const MultiSelectDropdown: React.FC<MultiSelectDropdownProps> = ({
  placeholder,
  options,
  selectedIds,
  onChange,
  emptyHint,
}) => {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleEscape);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [open ]);

  const toggle = (id: string) => {
    onChange(selectedIds.includes(id) ? selectedIds.filter(s => s !== id) : [...selectedIds, id]);
  };

  const selectedNames = options.filter(o => selectedIds.includes(o.id)).map(o => o.name);
  const buttonLabel = selectedNames.length === 0
    ? placeholder
    : `Đã chọn ${selectedNames.length}: ${selectedNames.slice(0, 3).join(', ')}${selectedNames.length > 3 ? '…' : ''}`;

  return (
    <div ref={containerRef} style={{ position: 'relative', width: '100%' }}>
      <button
        type="button"
        className="filter-select"
        onClick={() => setOpen(o => !o)}
        style={{ width: '100%', display: 'flex', justifyContent: 'space-between', alignItems: 'center', textAlign: 'left' }}
        title={selectedNames.join(', ') || placeholder}
      >
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', color: selectedNames.length === 0 ? 'var(--text-secondary)' : '#fff' }}>
          {buttonLabel}
        </span>
        <span style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', marginLeft: '0.5rem' }}>{open ? '▲' : '▼'}</span>
      </button>

      {open && (
        <div style={{
          position: 'absolute', top: 'calc(100% + 4px)', left: 0, right: 0, zIndex: 50,
          background: '#0f172a', border: '1px solid var(--border)', borderRadius: '6px',
          maxHeight: '220px', overflowY: 'auto', boxShadow: '0 8px 24px rgba(0,0,0,0.5)'
        }}>
          {options.length === 0 ? (
            <div style={{ padding: '0.7rem 0.9rem', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
              {emptyHint || 'Chưa có mục nào.'}
            </div>
          ) : (
            <>
              <div style={{ display: 'flex', gap: '0.5rem', padding: '0.5rem 0.9rem', borderBottom: '1px solid var(--border)', position: 'sticky', top: 0, background: '#0f172a' }}>
                <button type="button" className="btn btn-secondary" style={{ padding: '0.2rem 0.6rem', fontSize: '0.75rem' }} onClick={() => onChange(options.map(o => o.id))}>
                  Chọn tất cả
                </button>
                <button type="button" className="btn btn-secondary" style={{ padding: '0.2rem 0.6rem', fontSize: '0.75rem' }} onClick={() => onChange([])}>
                  Xóa hết
                </button>
              </div>
              {options.map(o => (
                <label
                  key={o.id}
                  className="checkbox-label"
                  style={{ padding: '0.5rem 0.9rem', fontSize: '0.85rem', cursor: 'pointer' }}
                >
                  <input
                    type="checkbox"
                    checked={selectedIds.includes(o.id)}
                    onChange={() => toggle(o.id)}
                  />
                  <span>{o.name}</span>
                </label>
              ))}
            </>
          )}
        </div>
      )}
    </div>
  );
};
