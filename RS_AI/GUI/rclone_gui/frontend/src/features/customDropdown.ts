/*
[INTEGRITY NOTES]
- Mục đích: Khởi tạo custom dropdown thay thế cho thẻ <select> mặc định của HTML.
- Trách nhiệm:
  - Ẩn thẻ <select> gốc, tạo UI thay thế với input và danh sách xổ xuống.
  - Đồng bộ giá trị được chọn về lại thẻ <select> gốc và kích hoạt sự kiện change.
  - Hỗ trợ tìm kiếm nhanh nếu truyền tham số searchable = true.
- Tương tác: Được dùng trong remotesManager.ts, mountManager.ts, v.v.
*/

export function upgradeSelectToCustomDropdown(selectEl: HTMLSelectElement, searchable: boolean = false) {
    if ((selectEl as any)._hasCustomDropdown) return;
    (selectEl as any)._hasCustomDropdown = true;

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
    
    // Vô hiệu hóa bàn phím ảo trên mobile hoặc chỉ cho phép click nếu không bật tính năng tìm kiếm (searchable)
    if (!searchable) {
        input.readOnly = true;
        input.style.cursor = 'pointer';
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

    // Xây dựng danh sách lựa chọn
    const updateOptions = () => {
        list.innerHTML = '';
        
        // Cập nhật lại giá trị input và placeholder
        if (selectEl.selectedIndex >= 0) {
            const opt = selectEl.options[selectEl.selectedIndex];
            if (opt && opt.value !== "") {
                input.value = opt.text;
            } else {
                input.value = '';
                if (opt) input.placeholder = opt.text;
            }
        }
        
        Array.from(selectEl.options).forEach(opt => {
            if (opt.value === "") return; // Bỏ qua lựa chọn rỗng (placeholder)
            const item = document.createElement('div');
            item.className = 'custom-dropdown-item';
            item.style.padding = '8px 10px';
            item.style.cursor = 'pointer';
            item.style.borderBottom = '1px solid var(--colors-border-muted, #333)';
            item.style.color = 'var(--colors-text-primary, #fff)';
            item.style.transition = 'background 0.2s';
            
            item.innerHTML = opt.innerHTML; // Giữ nguyên HTML nếu có (vd: provider có thẻ span)
            item.dataset.value = opt.value;
            item.dataset.text = opt.text;
            
            item.addEventListener('mouseenter', () => {
                item.style.backgroundColor = 'var(--colors-primary, #3b82f6)';
            });
            item.addEventListener('mouseleave', () => {
                item.style.backgroundColor = 'transparent';
            });
            
            item.addEventListener('click', (e) => {
                e.stopPropagation();
                selectEl.value = opt.value;
                input.value = opt.text;
                list.style.display = 'none';
                selectEl.dispatchEvent(new Event('change'));
            });
            
            list.appendChild(item);
        });
    };
    
    updateOptions();

    // Gắn sự kiện (Event listeners)
    input.addEventListener('click', () => {
        const isOpening = list.style.display === 'none';
        
        // Đóng các dropdown khác trước khi mở
        document.querySelectorAll('.custom-dropdown-list').forEach(el => {
            (el as HTMLElement).style.display = 'none';
        });
        
        if (isOpening) {
            list.style.display = 'block';
            if (searchable) {
                input.value = ''; // Xóa trắng để hiển thị toàn bộ list cho tìm kiếm
                Array.from(list.children).forEach(child => {
                    (child as HTMLElement).style.display = 'block';
                });
            }
        }
    });

    if (searchable) {
        input.addEventListener('input', () => {
            list.style.display = 'block';
            const query = input.value.toLowerCase();
            Array.from(list.children).forEach(child => {
                const text = (child as HTMLElement).dataset.text?.toLowerCase() || '';
                (child as HTMLElement).style.display = text.includes(query) ? 'block' : 'none';
            });
        });
        
        // Khôi phục giá trị nếu người dùng không chọn gì (blur out)
        input.addEventListener('blur', () => {
            setTimeout(() => {
                if (selectEl.selectedIndex >= 0) {
                    const opt = selectEl.options[selectEl.selectedIndex];
                    if (opt && opt.value !== "") {
                        input.value = opt.text;
                    } else {
                        input.value = '';
                        if (opt) input.placeholder = opt.text;
                    }
                }
            }, 200);
        });
    }

    document.addEventListener('click', (e) => {
        if (!wrapper.contains(e.target as Node)) {
            list.style.display = 'none';
        }
    });
    
    // Cho phép xây dựng lại danh sách khi thẻ select bị thay đổi từ bên ngoài (dynamically)
    (selectEl as any)._updateCustomDropdown = () => {
        updateOptions();
    };
    
    // Hàm đồng bộ nội bộ (Sync) khi giá trị thẻ select thay đổi bằng Javascript
    (selectEl as any)._syncCustomDropdown = () => {
        if (selectEl.selectedIndex >= 0) {
            const opt = selectEl.options[selectEl.selectedIndex];
            if (opt && opt.value !== "") {
                input.value = opt.text;
            } else {
                input.value = '';
                if (opt) input.placeholder = opt.text;
            }
        }
    };

    wrapper.appendChild(input);
    wrapper.appendChild(list);
    
    if (selectEl.parentNode) {
        selectEl.parentNode.insertBefore(wrapper, selectEl);
        wrapper.appendChild(selectEl); // Đưa select vào trong wrapper để chuẩn hóa Layout
    }
}
