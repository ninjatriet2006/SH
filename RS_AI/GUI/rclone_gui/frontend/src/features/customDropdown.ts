/*
[INTEGRITY NOTES]
- Mục đích: Khởi tạo custom dropdown thay thế cho thẻ <select> mặc định của HTML.
- Trách nhiệm:
  - Ẩn thẻ <select> gốc, tạo UI thay thế với input và danh sách xổ xuống.
  - Đồng bộ giá trị được chọn về lại thẻ <select> gốc và kích hoạt sự kiện change.
  - Hỗ trợ tìm kiếm nhanh nếu truyền tham số searchable = true.
- Tương tác: Được dùng trong remotesManager.ts, mountManager.ts, v.v.
*/

let customDropdownId = 0;

export function upgradeSelectToCustomDropdown(selectEl: HTMLSelectElement, searchable: boolean = false) {
    if ((selectEl as any)._hasCustomDropdown) return;
    (selectEl as any)._hasCustomDropdown = true;

    const originalDisplay = selectEl.style.display;
    selectEl.style.display = 'none';

    const wrapper = document.createElement('div');
    wrapper.style.position = 'relative';
    wrapper.style.width = '100%';
    wrapper.className = 'custom-dropdown-wrapper';

    // Khởi tạo thẻ Input hiển thị
    const input = document.createElement('input');
    input.type = 'text';
    input.className = selectEl.className;
    
    // Khôi phục một vài style quan trọng
    input.style.width = '100%';
    input.style.boxSizing = 'border-box';
    input.style.padding = '8px';
    input.style.border = '1px solid var(--colors-border-muted, #555)';
    input.style.borderRadius = '4px';
    input.style.background = 'var(--colors-surface-input, #0e1422)';
    input.style.color = 'var(--colors-text-primary, #fff)';
    input.style.colorScheme = 'dark';
    input.autocomplete = 'off';
    input.setAttribute('role', 'combobox');
    input.setAttribute('aria-haspopup', 'listbox');
    input.setAttribute('aria-expanded', 'false');
    input.setAttribute('aria-autocomplete', searchable ? 'list' : 'none');
    ['aria-label', 'aria-labelledby', 'aria-describedby'].forEach(attribute => {
        const value = selectEl.getAttribute(attribute);
        if (value) input.setAttribute(attribute, value);
    });
    if (!input.hasAttribute('aria-label') && !input.hasAttribute('aria-labelledby')) {
        const label = selectEl.labels?.[0]?.textContent?.trim();
        if (label) input.setAttribute('aria-label', label);
    }
    
    // Vô hiệu hóa bàn phím ảo trên mobile hoặc chỉ cho phép click nếu không bật tính năng tìm kiếm (searchable)
    if (!searchable) {
        input.readOnly = true;
        input.style.cursor = 'pointer';
        input.setAttribute('aria-readonly', 'true');
    }
    
    // Sao chép placeholder mặc định
    const firstOption = selectEl.options[0];
    if (firstOption && firstOption.value === "") {
        input.placeholder = firstOption.text;
    } else {
        input.placeholder = "-- Chọn --";
    }

    // Khởi tạo container chứa danh sách xổ xuống
    const list = document.createElement('div');
    list.className = 'custom-dropdown-list';
    list.id = `custom-dropdown-list-${++customDropdownId}`;
    list.setAttribute('role', 'listbox');
    list.hidden = true;
    input.setAttribute('aria-controls', list.id);
    list.style.display = 'none';
    list.style.position = 'absolute';
    list.style.top = '100%';
    list.style.left = '0';
    list.style.right = '0';
    list.style.maxHeight = '250px';
    list.style.overflowY = 'auto';
    list.style.background = 'var(--colors-surface-overlay, #1a2333)';
    list.style.zIndex = '1000';
    list.style.border = '1px solid var(--colors-border-muted, #555)';
    list.style.borderTop = 'none';
    list.style.borderRadius = '0 0 4px 4px';
    list.style.boxShadow = '0 8px 16px rgba(0,0,0,0.7)';

    let isOpen = false;
    let highlightedItem: HTMLElement | null = null;

    const syncValue = () => {
        if (selectEl.selectedIndex >= 0) {
            const opt = selectEl.options[selectEl.selectedIndex];
            if (opt && opt.value !== "") {
                input.value = opt.text;
            } else {
                input.value = '';
                if (opt) input.placeholder = opt.text;
            }
        } else {
            input.value = '';
        }
    };

    const paintItems = () => {
        Array.from(list.children).forEach(child => {
            const item = child as HTMLElement;
            const option = selectEl.options[Number(item.dataset.index)];
            const isSelected = Boolean(option?.selected);
            const isHighlighted = item === highlightedItem;
            item.classList.toggle('is-selected', isSelected);
            item.classList.toggle('is-highlighted', isHighlighted);
            item.setAttribute('aria-selected', String(isSelected));
            item.style.backgroundColor = isHighlighted
                ? 'var(--colors-primary, #3b82f6)'
                : isSelected ? 'var(--colors-surface-selected, #24324a)' : 'transparent';
            item.style.fontWeight = isSelected ? '600' : 'normal';
        });
    };

    const setHighlightedItem = (item: HTMLElement | null) => {
        highlightedItem = item;
        if (item) {
            input.setAttribute('aria-activedescendant', item.id);
            item.scrollIntoView?.({ block: 'nearest' });
        } else {
            input.removeAttribute('aria-activedescendant');
        }
        paintItems();
    };

    const getNavigableItems = () => Array.from(list.children).filter(child => {
        const item = child as HTMLElement;
        return item.style.display !== 'none' && item.getAttribute('aria-disabled') !== 'true';
    }) as HTMLElement[];

    const closeDropdown = (restoreValue = true) => {
        isOpen = false;
        list.hidden = true;
        list.style.display = 'none';
        input.setAttribute('aria-expanded', 'false');
        setHighlightedItem(null);
        if (restoreValue) syncValue();
    };

    const openDropdown = (clearSearch = false) => {
        if (selectEl.disabled) return;
        document.dispatchEvent(new CustomEvent('custom-dropdown-open', { detail: wrapper }));
        isOpen = true;
        list.hidden = false;
        list.style.display = 'block';
        input.setAttribute('aria-expanded', 'true');
        if (searchable && clearSearch) {
            input.value = '';
            Array.from(list.children).forEach(child => {
                (child as HTMLElement).style.display = 'block';
            });
        }
        const items = getNavigableItems();
        const selected = items.find(item => selectEl.options[Number(item.dataset.index)]?.selected);
        setHighlightedItem(selected || items[0] || null);
    };

    const selectItem = (item: HTMLElement) => {
        if (item.getAttribute('aria-disabled') === 'true') return;
        const option = selectEl.options[Number(item.dataset.index)];
        if (!option) return;
        selectEl.value = option.value;
        syncValue();
        paintItems();
        closeDropdown(false);
        selectEl.dispatchEvent(new Event('input', { bubbles: true }));
        selectEl.dispatchEvent(new Event('change', { bubbles: true }));
    };

    const filterOptions = (query: string) => {
        const normalizedQuery = query.toLowerCase();
        Array.from(list.children).forEach(child => {
            const item = child as HTMLElement;
            const text = item.dataset.text?.toLowerCase() || '';
            item.style.display = text.includes(normalizedQuery) ? 'block' : 'none';
        });
        if (!highlightedItem || highlightedItem.style.display === 'none') {
            setHighlightedItem(getNavigableItems()[0] || null);
        }
    };

    // Xây dựng danh sách lựa chọn
    const updateOptions = () => {
        list.replaceChildren();
        highlightedItem = null;
        syncValue();

        Array.from(selectEl.options).forEach((opt, index) => {
            if (opt.value === "") return; // Bỏ qua lựa chọn rỗng (placeholder)
            const item = document.createElement('div');
            item.className = 'custom-dropdown-item';
            item.id = `${list.id}-option-${index}`;
            item.setAttribute('role', 'option');
            item.style.padding = '8px 10px';
            item.style.borderBottom = '1px solid var(--colors-border-muted, #333)';
            item.style.color = 'var(--colors-text-primary, #fff)';
            item.style.transition = 'background 0.2s';

            const isDisabled = opt.disabled || (opt.parentElement instanceof HTMLOptGroupElement && opt.parentElement.disabled);
            item.textContent = opt.text;
            item.dataset.value = opt.value;
            item.dataset.text = opt.text;
            item.dataset.index = String(index);
            item.setAttribute('aria-disabled', String(isDisabled));
            item.style.cursor = isDisabled ? 'not-allowed' : 'pointer';
            item.style.opacity = isDisabled ? '0.55' : '1';

            item.addEventListener('mouseenter', () => {
                if (!isDisabled) setHighlightedItem(item);
            });

            item.addEventListener('mousedown', (e) => {
                e.preventDefault();
                e.stopPropagation();
                selectItem(item);
            });

            list.appendChild(item);
        });
        paintItems();
    };

    updateOptions();

    // Gắn sự kiện (Event listeners)
    const handleInputClick = () => {
        if (searchable) {
            if (!isOpen) openDropdown(true);
        } else if (isOpen) closeDropdown();
        else openDropdown(false);
    };
    input.addEventListener('click', handleInputClick);

    if (searchable) {
        input.addEventListener('input', () => {
            if (!isOpen) openDropdown(false);
            filterOptions(input.value);
        });
    }

    input.addEventListener('keydown', event => {
        const items = getNavigableItems();
        const currentIndex = highlightedItem ? items.indexOf(highlightedItem) : -1;
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            if (!isOpen) {
                openDropdown(searchable);
                const openItems = getNavigableItems();
                const selected = openItems.find(item => selectEl.options[Number(item.dataset.index)]?.selected);
                setHighlightedItem(selected || (event.key === 'ArrowDown' ? openItems[0] : openItems[openItems.length - 1]) || null);
            } else if (items.length) {
                const offset = event.key === 'ArrowDown' ? 1 : -1;
                const nextIndex = currentIndex < 0
                    ? (offset > 0 ? 0 : items.length - 1)
                    : Math.max(0, Math.min(items.length - 1, currentIndex + offset));
                setHighlightedItem(items[nextIndex]);
            }
        } else if (event.key === 'Home' || event.key === 'End') {
            event.preventDefault();
            if (!isOpen) openDropdown(searchable);
            const openItems = getNavigableItems();
            setHighlightedItem(event.key === 'Home' ? openItems[0] || null : openItems[openItems.length - 1] || null);
        } else if (event.key === 'Enter' || (event.key === ' ' && !searchable)) {
            event.preventDefault();
            if (isOpen && highlightedItem) selectItem(highlightedItem);
            else openDropdown(searchable);
        } else if (event.key === 'Escape' && isOpen) {
            event.preventDefault();
            closeDropdown();
        } else if (event.key === 'Tab' && isOpen) {
            closeDropdown();
        }
    });

    const handleBlur = () => {
        setTimeout(() => {
            if (!wrapper.contains(document.activeElement)) closeDropdown();
        }, 0);
    };
    input.addEventListener('blur', handleBlur);

    const handleDocumentClick = (event: Event) => {
        const target = event.target;
        if (target instanceof Node && !wrapper.contains(target)) closeDropdown();
    };
    document.addEventListener('click', handleDocumentClick);

    const handleOtherDropdownOpen = (event: Event) => {
        if ((event as CustomEvent).detail !== wrapper) closeDropdown();
    };
    document.addEventListener('custom-dropdown-open', handleOtherDropdownOpen);

    const syncDisabled = () => {
        input.disabled = selectEl.disabled;
        input.setAttribute('aria-disabled', String(selectEl.disabled));
        if (selectEl.disabled) closeDropdown();
    };
    syncDisabled();

    const handleSelectValueChange = () => {
        syncValue();
        paintItems();
    };
    selectEl.addEventListener('input', handleSelectValueChange);
    selectEl.addEventListener('change', handleSelectValueChange);

    const observer = new MutationObserver(records => {
        if (records.some(record => record.type === 'childList' || record.target !== selectEl || record.attributeName !== 'disabled')) {
            updateOptions();
        }
        syncDisabled();
    });
    observer.observe(selectEl, {
        attributes: true,
        attributeFilter: ['disabled', 'selected', 'label', 'value'],
        characterData: true,
        childList: true,
        subtree: true,
    });

    const destroy = () => {
        closeDropdown();
        observer.disconnect();
        document.removeEventListener('click', handleDocumentClick);
        document.removeEventListener('custom-dropdown-open', handleOtherDropdownOpen);
        selectEl.removeEventListener('input', handleSelectValueChange);
        selectEl.removeEventListener('change', handleSelectValueChange);
        if (wrapper.parentNode) wrapper.parentNode.insertBefore(selectEl, wrapper);
        wrapper.remove();
        selectEl.style.display = originalDisplay;
        delete (selectEl as any)._hasCustomDropdown;
        delete (selectEl as any)._updateCustomDropdown;
        delete (selectEl as any)._syncCustomDropdown;
        delete (selectEl as any)._destroyCustomDropdown;
    };

    (selectEl as any)._destroyCustomDropdown = destroy;
    
    // Cho phép xây dựng lại danh sách khi thẻ select bị thay đổi từ bên ngoài (dynamically)
    (selectEl as any)._updateCustomDropdown = () => {
        updateOptions();
    };
    
    // Hàm đồng bộ nội bộ (Sync) khi giá trị thẻ select thay đổi bằng Javascript
    (selectEl as any)._syncCustomDropdown = () => {
        syncValue();
        paintItems();
        syncDisabled();
    };

    wrapper.appendChild(input);
    wrapper.appendChild(list);
    
    if (selectEl.parentNode) {
        selectEl.parentNode.insertBefore(wrapper, selectEl);
        wrapper.appendChild(selectEl); // Đưa select vào trong wrapper để chuẩn hóa Layout
    }
}
