/*
[INTEGRITY NOTES]
- Mục đích: Dropdown uniform cho toàn app — thay thẻ <select> gốc trong các
  form React. Mẫu hành vi lấy từ `rclone_gui` `features/customDropdown.ts`
  (upgradeSelectToCustomDropdown: input + danh xổ + tìm kiếm nhanh + điều
  hướng bàn phím + sync về select gốc); viết lại thành component React vì
  bản rclone là vanilla-DOM thao tác trực tiếp DOM — không dùng được trong
  React (state/render do React quản).
- Trách nhiệm: HIỂN THỊ + TÌM KIẾM + CHỌN. Không tự gọi API, không đổi state
  của cha ngoài onChange đã truyền.
- Tương tác: các trang dùng `<select className="input-field">` hiện tại —
  ModelsPage (chọn arbiter, chọn provider lọc), CkeyPage (đích import,
  khoảng thời gian thống kê), ProvidersPage/ProviderModal (preset, npm),
  SettingsPage (ngôn ngữ, theme).

Vì SAO để chung app này (không nhét vào libs/ dùng chung 3 GUI):
  - rclone_gui là vanilla TS (không React) — dùng chung component là không
    thể; hợp đồng hành vi của nó đã được "mượn" vào đây.
  - subscription_manager_gui cùng stack React + CSS token giống nhau
    (.input-field) — MỘT LÚC cần thêm dropdown ngoài app này thì tách file
    này sang chỗ chung, KHÔNG copy-paste (lệnh cấm trùng lặp logic UI).
*/

import { useEffect, useMemo, useRef, useState } from 'react';
import { ChevronDown, Search } from 'lucide-react';

export interface SearchableOption {
    value: string;
    label: string;
    /** Gợi ý hiển thị mờ sau label (vd "⭐", số đếm). */
    hint?: string;
}

interface SearchableSelectProps {
    options: SearchableOption[];
    value: string;
    onChange: (value: string) => void;
    /** Chữ gợi ý khi chưa chọn / ô tìm kiếm trống. */
    placeholder?: string;
    /** Bật ô tìm kiếm (danh sách dài). */
    searchable?: boolean;
    disabled?: boolean;
    /** Bố cục dọc (label riêng) — khớp form-group các trang khác. */
    style?: React.CSSProperties;
}

export function SearchableSelect({
    options,
    value,
    onChange,
    placeholder,
    searchable = false,
    disabled = false,
    style,
}: SearchableSelectProps) {
    const [open, setOpen] = useState(false);
    const [query, setQuery] = useState('');
    const [highlight, setHighlight] = useState(0);
    const rootRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLDivElement>(null);

    const selected = options.find(o => o.value === value) ?? null;

    const filtered = useMemo(() => {
        const q = query.trim().toLowerCase();
        if (!q || !searchable) return options;
        return options.filter(o => o.label.toLowerCase().includes(q) || o.value.toLowerCase().includes(q));
    }, [options, query, searchable]);

    // Đóng khi click ngoài (giống dropdown native).
    useEffect(() => {
        if (!open) return;
        const onDown = (e: MouseEvent) => {
            if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
                setOpen(false);
            }
        };
        document.addEventListener('mousedown', onDown);
        return () => document.removeEventListener('mousedown', onDown);
    }, [open]);

    // Mở là focus ô tìm kiếm + reset lọc + highlight mục đang chọn.
    useEffect(() => {
        if (open) {
            setQuery('');
            const idx = filtered.findIndex(o => o.value === value);
            setHighlight(idx >= 0 ? idx : 0);
            // Focus sau khi render danh sách.
            requestAnimationFrame(() => inputRef.current?.focus());
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [open]);

    // Giữ dòng highlight trong tầm nhìn khi điều hướng bàn phím.
    useEffect(() => {
        const el = listRef.current?.children[highlight] as HTMLElement | undefined;
        el?.scrollIntoView({ block: 'nearest' });
    }, [highlight]);

    const pick = (v: string) => {
        onChange(v);
        setOpen(false);
    };

    const onKey = (e: React.KeyboardEvent) => {
        if (disabled) return;
        if (!open) {
            if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
                e.preventDefault();
                setOpen(true);
            }
            return;
        }
        switch (e.key) {
            case 'ArrowDown':
                e.preventDefault();
                // max(-1) khi danh sách rỗng — Enter đã có guard, không crash.
                setHighlight(h => Math.max(0, Math.min(h + 1, filtered.length - 1)));
                break;
            case 'ArrowUp':
                e.preventDefault();
                setHighlight(h => Math.max(h - 1, 0));
                break;
            case 'Enter':
                e.preventDefault();
                if (filtered[highlight]) pick(filtered[highlight].value);
                break;
            case 'Escape':
                e.preventDefault();
                setOpen(false);
                break;
            case 'Tab':
                setOpen(false);
                break;
        }
    };

    return (
        <div
            ref={rootRef}
            className="searchable-select"
            style={{ position: 'relative', ...style }}
            onKeyDown={onKey}
        >
            {/* Ô hiển thị: click mở; khi có search thì gõ thẳng vào ô này. */}
            <div
                className="input-field"
                role="combobox"
                aria-expanded={open}
                aria-disabled={disabled}
                tabIndex={disabled ? -1 : 0}
                style={{
                    display: 'flex', alignItems: 'center', gap: '0.4rem',
                    cursor: disabled ? 'not-allowed' : 'pointer',
                    opacity: disabled ? 0.5 : 1,
                    background: open && searchable ? 'var(--bg-panel, rgba(0,0,0,0.25))' : undefined,
                }}
                onClick={() => !disabled && setOpen(o => !o)}
            >
                {searchable && open ? (
                    <>
                        <Search size={14} style={{ flexShrink: 0, color: 'var(--text-secondary)' }} />
                        <input
                            ref={inputRef}
                            type="text"
                            value={query}
                            onChange={e => {
                                setQuery(e.target.value);
                                setHighlight(0);
                            }}
                            placeholder={placeholder ?? ''}
                            style={{
                                flex: 1, background: 'transparent', border: 'none', outline: 'none',
                                color: 'inherit', fontFamily: 'inherit', fontSize: 'inherit',
                                padding: 0, minWidth: 0,
                            }}
                            onClick={e => e.stopPropagation()}
                        />
                    </>
                ) : (
                    <span
                        style={{
                            flex: 1,
                            overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                            color: selected ? 'inherit' : 'var(--text-secondary)',
                        }}
                    >
                        {selected?.label ?? placeholder ?? '—'}
                    </span>
                )}
                <ChevronDown
                    size={15}
                    style={{ flexShrink: 0, color: 'var(--text-secondary)', transform: open ? 'rotate(180deg)' : 'none', transition: 'transform 0.15s' }}
                />
            </div>

            {open && (
                <div
                    ref={listRef}
                    className="searchable-select-list"
                    style={{
                        position: 'absolute', top: 'calc(100% + 2px)', left: 0, right: 0,
                        maxHeight: '260px', overflowY: 'auto',
                        background: 'var(--bg-panel, #14192a)',
                        border: '1px solid var(--border)', borderRadius: '4px',
                        zIndex: 1000, boxShadow: '0 8px 16px rgba(0,0,0,0.5)',
                    }}
                >
                    {filtered.length === 0 ? (
                        <div style={{ padding: '0.6rem 0.75rem', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                            {placeholder ? `${placeholder}` : '—'}
                        </div>
                    ) : (
                        filtered.map((o, i) => (
                            <div
                                key={o.value}
                                role="option"
                                aria-selected={o.value === value}
                                style={{
                                    display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '0.5rem',
                                    padding: '0.45rem 0.75rem', cursor: 'pointer', fontSize: '0.88rem',
                                    background: i === highlight ? 'rgba(99,102,241,0.18)' : 'transparent',
                                    color: o.value === value ? 'var(--primary)' : 'inherit',
                                    fontWeight: o.value === value ? 600 : 400,
                                }}
                                onMouseEnter={() => setHighlight(i)}
                                onMouseDown={e => { e.preventDefault(); pick(o.value); }}
                            >
                                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{o.label}</span>
                                {o.hint && (
                                    <span style={{ flexShrink: 0, fontSize: '0.72rem', color: 'var(--text-secondary)' }}>{o.hint}</span>
                                )}
                            </div>
                        ))
                    )}
                </div>
            )}
        </div>
    );
}
