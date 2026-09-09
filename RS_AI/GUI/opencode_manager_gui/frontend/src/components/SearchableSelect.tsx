/* Searchable combobox for long or dynamic option lists. Short static lists
 * should remain native <select> controls for mobile and form semantics. */

import { useEffect, useId, useMemo, useRef, useState } from 'react';
import { ChevronDown, Search, X } from 'lucide-react';

export interface SearchableOption {
    value: string;
    label: string;
    hint?: string;
    disabled?: boolean;
}

interface SearchableSelectProps {
    options: SearchableOption[];
    value: string;
    onChange: (value: string) => void;
    placeholder?: string;
    disabled?: boolean;
    clearable?: boolean;
    ariaLabel?: string;
    style?: React.CSSProperties;
}

export function SearchableSelect({
    options,
    value,
    onChange,
    placeholder,
    disabled = false,
    clearable = false,
    ariaLabel,
    style,
}: SearchableSelectProps) {
    const id = useId().replace(/:/g, '');
    const listId = `${id}-listbox`;
    const [open, setOpen] = useState(false);
    const [query, setQuery] = useState('');
    const [highlight, setHighlight] = useState(0);
    const resetQueryOnOpenRef = useRef(true);
    const rootRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLDivElement>(null);
    const selected = options.find(option => option.value === value) ?? null;

    const filtered = useMemo(() => {
        const normalized = query.trim().toLowerCase();
        if (!normalized) return options;
        return options.filter(option => option.label.toLowerCase().includes(normalized)
            || option.value.toLowerCase().includes(normalized));
    }, [options, query]);

    useEffect(() => {
        if (!open) return;
        const closeOutside = (event: PointerEvent) => {
            if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
        };
        document.addEventListener('pointerdown', closeOutside);
        return () => document.removeEventListener('pointerdown', closeOutside);
    }, [open]);

    useEffect(() => {
        if (!open) return;
        const selectedIndex = options.findIndex(option => option.value === value && !option.disabled);
        if (resetQueryOnOpenRef.current) {
            setQuery('');
            setHighlight(Math.max(selectedIndex, 0));
        }
        resetQueryOnOpenRef.current = true;
        requestAnimationFrame(() => inputRef.current?.focus());
        // Opening is the only point where the query and initial row reset.
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [open]);

    useEffect(() => {
        if (highlight >= filtered.length) setHighlight(Math.max(filtered.length - 1, 0));
        const row = listRef.current?.children[highlight] as HTMLElement | undefined;
        row?.scrollIntoView({ block: 'nearest' });
    }, [filtered.length, highlight]);

    const close = () => {
        setOpen(false);
        setQuery('');
    };

    const pick = (option: SearchableOption) => {
        if (option.disabled) return;
        onChange(option.value);
        close();
    };

    const moveHighlight = (direction: 1 | -1) => {
        if (!filtered.some(option => !option.disabled)) {
            setHighlight(-1);
            return;
        }
        let next = highlight;
        for (let count = 0; count < filtered.length; count += 1) {
            next = (next + direction + filtered.length) % filtered.length;
            if (!filtered[next].disabled) break;
        }
        setHighlight(next);
    };

    const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
        if (disabled) return;
        if (!open && ['Enter', ' ', 'ArrowDown', 'ArrowUp'].includes(event.key)) {
            event.preventDefault();
            resetQueryOnOpenRef.current = true;
            setOpen(true);
            return;
        }
        if (!open) return;
        switch (event.key) {
            case 'ArrowDown':
                event.preventDefault();
                moveHighlight(1);
                break;
            case 'ArrowUp':
                event.preventDefault();
                moveHighlight(-1);
                break;
            case 'Home':
                event.preventDefault();
                setHighlight(filtered.findIndex(option => !option.disabled));
                break;
            case 'End':
                event.preventDefault();
                setHighlight(-1);
                for (let index = filtered.length - 1; index >= 0; index -= 1) {
                    if (!filtered[index].disabled) {
                        setHighlight(index);
                        break;
                    }
                }
                break;
            case 'Enter':
                event.preventDefault();
                if (filtered[highlight]) pick(filtered[highlight]);
                break;
            case 'Escape':
                event.preventDefault();
                close();
                break;
            case 'Tab':
                close();
                break;
        }
    };

    return (
        <div ref={rootRef} className="searchable-select" style={{ position: 'relative', ...style }}>
            <div style={{ position: 'relative' }}>
                <Search
                    size={14}
                    aria-hidden="true"
                    style={{ position: 'absolute', left: '0.8rem', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-secondary)', pointerEvents: 'none' }}
                />
                <input
                    ref={inputRef}
                    className="input-field"
                    role="combobox"
                    aria-label={ariaLabel ?? placeholder}
                    aria-autocomplete="list"
                    aria-expanded={open}
                    aria-controls={open ? listId : undefined}
                    aria-activedescendant={open && filtered[highlight] ? `${id}-option-${highlight}` : undefined}
                    autoComplete="off"
                    disabled={disabled}
                    value={open ? query : (selected?.label ?? '')}
                    placeholder={placeholder}
                    onFocus={event => {
                        if (!open) event.currentTarget.select();
                    }}
                    onChange={event => {
                        setQuery(event.target.value);
                        setHighlight(0);
                        if (!open) {
                            resetQueryOnOpenRef.current = false;
                            setOpen(true);
                        }
                    }}
                    onClick={() => {
                        if (!open) {
                            resetQueryOnOpenRef.current = true;
                            setOpen(true);
                        }
                    }}
                    onKeyDown={onKeyDown}
                    style={{ paddingLeft: '2.2rem', paddingRight: clearable && selected ? '4rem' : '2.5rem' }}
                />
                {clearable && selected && !disabled && (
                    <button
                        type="button"
                        aria-label="Clear selection"
                        onClick={() => {
                            onChange('');
                            close();
                            inputRef.current?.focus();
                        }}
                        style={{ position: 'absolute', right: '2rem', top: '50%', transform: 'translateY(-50%)', display: 'grid', placeItems: 'center', padding: 0, border: 0, background: 'transparent', color: 'var(--text-secondary)', cursor: 'pointer' }}
                    >
                        <X size={14} />
                    </button>
                )}
                <ChevronDown
                    size={15}
                    aria-hidden="true"
                    style={{ position: 'absolute', right: '0.75rem', top: '50%', transform: `translateY(-50%) rotate(${open ? 180 : 0}deg)`, color: 'var(--text-secondary)', pointerEvents: 'none', transition: 'transform 0.15s' }}
                />
            </div>

            {open && (
                <div
                    ref={listRef}
                    id={listId}
                    role="listbox"
                    className="searchable-select-list"
                    style={{ position: 'absolute', top: 'calc(100% + 2px)', left: 0, right: 0, maxHeight: '260px', overflowY: 'auto', background: 'var(--bg-panel, #14192a)', border: '1px solid var(--border)', borderRadius: '4px', zIndex: 1000, boxShadow: '0 8px 16px rgba(0,0,0,0.5)' }}
                >
                    {filtered.length === 0 ? (
                        <div style={{ padding: '0.7rem 0.75rem', color: 'var(--text-secondary)' }}>—</div>
                    ) : filtered.map((option, index) => (
                        <div
                            id={`${id}-option-${index}`}
                            key={option.value}
                            role="option"
                            aria-selected={option.value === value}
                            aria-disabled={option.disabled || undefined}
                            style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '0.5rem', minHeight: '42px', padding: '0.45rem 0.75rem', cursor: option.disabled ? 'not-allowed' : 'pointer', fontSize: '0.88rem', opacity: option.disabled ? 0.5 : 1, background: index === highlight ? 'rgba(99,102,241,0.18)' : 'transparent', color: option.value === value ? 'var(--primary)' : 'inherit', fontWeight: option.value === value ? 600 : 400 }}
                            onMouseEnter={() => !option.disabled && setHighlight(index)}
                            onMouseDown={event => {
                                event.preventDefault();
                                pick(option);
                            }}
                        >
                            <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{option.label}</span>
                            {option.hint && <span style={{ flexShrink: 0, fontSize: '0.72rem', color: 'var(--text-secondary)' }}>{option.hint}</span>}
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
