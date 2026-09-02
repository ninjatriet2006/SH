/*
[INTEGRITY NOTES]
- Mục đích: Thanh menu ngang (File / Edit / View / Go) ở đỉnh ứng dụng.
- Trách nhiệm: Hiển thị dropdown và phát các lệnh tác động lên pane đang active.
- Tương tác: Nhận `ExplorerCommands` do main.ts truyền vào (không truy cập biến
  toàn cục), gọi plugin-process để thoát ứng dụng.
*/

/** Tập lệnh tối thiểu MenuBar cần từ Explorer. */
export interface ExplorerCommands {
  selectAllActive: () => void;
  goHomeActive: () => void;
  refreshActive: () => void;
}

interface MenuItem {
  label: string;
  langId?: string;
  action?: string;
  disabled?: boolean;
}

interface MenuDropdown {
  label: string;
  langId?: string;
  items: (MenuItem | 'separator')[];
}

export class MenuBar {
  private element: HTMLElement;
  private activeDropdown: HTMLElement | null = null;
  private commands: ExplorerCommands;

  constructor(commands: ExplorerCommands) {
    this.commands = commands;
    this.element = document.createElement('nav');
    this.element.className = 'nemo-menubar';
    this.init();
  }

  private init() {
    const menus: MenuDropdown[] = [
      {
        label: 'File',
        langId: 'menu_file',
        items: [
          { label: 'Quit', langId: 'menu_file_quit', action: 'quit' }
        ]
      },
      {
        label: 'Edit',
        langId: 'menu_edit',
        items: [
          { label: 'Select All', langId: 'menu_edit_select_all', action: 'select_all' }
        ]
      },
      {
        label: 'View',
        langId: 'menu_view',
        items: [
          { label: 'Reload', langId: 'menu_view_reload', action: 'reload' }
        ]
      },
      {
        label: 'Go',
        langId: 'menu_go',
        items: [
          { label: 'Home', langId: 'menu_go_home', action: 'go_home' }
        ]
      }
    ];

    menus.forEach(menu => {
      const menuContainer = document.createElement('div');
      menuContainer.className = 'menubar-item-container';

      const btn = document.createElement('button');
      btn.className = 'menubar-item-btn';
      btn.textContent = menu.label;
      if (menu.langId) {
        btn.dataset.langId = menu.langId;
      }

      const dropdown = document.createElement('div');
      dropdown.className = 'menubar-dropdown';
      
      menu.items.forEach(item => {
        if (item === 'separator') {
          const sep = document.createElement('div');
          sep.className = 'menubar-separator';
          dropdown.appendChild(sep);
        } else {
          const actionBtn = document.createElement('button');
          actionBtn.className = 'menubar-action-btn';
          actionBtn.textContent = item.label;
          if (item.langId) actionBtn.dataset.langId = item.langId;
          if (item.disabled) actionBtn.disabled = true;
          
          actionBtn.addEventListener('click', () => {
            this.handleAction(item.action);
            this.closeDropdowns();
          });
          dropdown.appendChild(actionBtn);
        }
      });

      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        if (this.activeDropdown === dropdown) {
          this.closeDropdowns();
        } else {
          this.closeDropdowns();
          this.activeDropdown = dropdown;
          dropdown.classList.add('show');
          btn.classList.add('active');
        }
      });

      menuContainer.appendChild(btn);
      menuContainer.appendChild(dropdown);
      this.element.appendChild(menuContainer);
    });

    document.addEventListener('click', () => this.closeDropdowns());
  }

  private closeDropdowns() {
    if (this.activeDropdown) {
      this.activeDropdown.classList.remove('show');
      this.activeDropdown.parentElement?.querySelector('.menubar-item-btn')?.classList.remove('active');
      this.activeDropdown = null;
    }
  }

  private async handleAction(action?: string) {
    if (!action) return;

    switch (action) {
      case 'quit':
        try {
          // Tauri v2: window.close() của webview không có tác dụng; phải yêu cầu
          // cửa sổ native đóng. Cần quyền `core:window:allow-close` trong
          // backend/capabilities/main.json.
          const { getCurrentWindow } = await import('@tauri-apps/api/window');
          await getCurrentWindow().close();
        } catch (e) {
          console.error('Không thể đóng cửa sổ:', e);
        }
        break;
      case 'select_all':
        this.commands.selectAllActive();
        break;
      case 'reload':
        this.commands.refreshActive();
        break;
      case 'go_home':
        this.commands.goHomeActive();
        break;
    }
  }

  public getElement(): HTMLElement {
    return this.element;
  }
}
