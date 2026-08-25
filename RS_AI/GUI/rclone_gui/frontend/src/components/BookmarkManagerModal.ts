import { appState } from '../store';

export class BookmarkManagerModal {
  private element: HTMLDivElement;

  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'auth-modal'; // Tạm dùng class auth-modal cho modal đơn giản
    this.element.style.width = '400px';
    
    this.render();
  }

  private render() {
    this.element.innerHTML = '';
    
    const title = document.createElement('h2');
    title.textContent = 'Manage Bookmarks';
    this.element.appendChild(title);
    
    const bookmarks = appState.bookmarks || [];
    if (bookmarks.length === 0) {
      const empty = document.createElement('div');
      empty.style.padding = '20px';
      empty.style.textAlign = 'center';
      empty.textContent = 'No bookmarks yet.';
      this.element.appendChild(empty);
    } else {
      const list = document.createElement('div');
      list.className = 'bookmark-list';
      list.style.maxHeight = '300px';
      list.style.overflowY = 'auto';
      
      bookmarks.forEach((b, i) => {
        const item = document.createElement('div');
        item.style.display = 'flex';
        item.style.alignItems = 'center';
        item.style.gap = '8px';
        item.style.marginBottom = '8px';
        item.style.padding = '8px';
        item.style.background = 'var(--colors-surface-glass, rgba(255,255,255,0.05))';
        item.style.borderRadius = '6px';
        
        const input = document.createElement('input');
        input.type = 'text';
        input.value = b.name;
        input.style.flex = '1';
        input.style.padding = '4px';
        input.onchange = () => {
          if (appState.bookmarks) {
            appState.bookmarks[i].name = input.value;
            this.saveAndRefresh();
          }
        };
        
        const pathDiv = document.createElement('div');
        pathDiv.style.fontSize = '10px';
        pathDiv.style.color = 'var(--colors-text-muted, gray)';
        pathDiv.style.maxWidth = '100px';
        pathDiv.style.overflow = 'hidden';
        pathDiv.style.textOverflow = 'ellipsis';
        pathDiv.style.whiteSpace = 'nowrap';
        pathDiv.textContent = b.path;
        
        const upBtn = document.createElement('button');
        upBtn.textContent = '⬆️';
        upBtn.style.padding = '4px';
        upBtn.disabled = i === 0;
        upBtn.onclick = () => {
          if (appState.bookmarks && i > 0) {
            const temp = appState.bookmarks[i];
            appState.bookmarks[i] = appState.bookmarks[i - 1];
            appState.bookmarks[i - 1] = temp;
            this.saveAndRefresh();
          }
        };
        
        const downBtn = document.createElement('button');
        downBtn.textContent = '⬇️';
        downBtn.style.padding = '4px';
        downBtn.disabled = i === bookmarks.length - 1;
        downBtn.onclick = () => {
          if (appState.bookmarks && i < appState.bookmarks.length - 1) {
            const temp = appState.bookmarks[i];
            appState.bookmarks[i] = appState.bookmarks[i + 1];
            appState.bookmarks[i + 1] = temp;
            this.saveAndRefresh();
          }
        };
        
        const delBtn = document.createElement('button');
        delBtn.textContent = '❌';
        delBtn.style.padding = '4px';
        delBtn.onclick = () => {
          if (appState.bookmarks) {
            appState.bookmarks.splice(i, 1);
            this.saveAndRefresh();
          }
        };
        
        item.appendChild(input);
        item.appendChild(pathDiv);
        item.appendChild(upBtn);
        item.appendChild(downBtn);
        item.appendChild(delBtn);
        list.appendChild(item);
      });
      
      this.element.appendChild(list);
    }
    
    const actions = document.createElement('div');
    actions.style.display = 'flex';
    actions.style.justifyContent = 'flex-end';
    actions.style.marginTop = '20px';
    
    const closeBtn = document.createElement('button');
    closeBtn.textContent = 'Close';
    closeBtn.onclick = () => this.close();
    actions.appendChild(closeBtn);
    
    this.element.appendChild(actions);
  }

  private saveAndRefresh() {
    try {
      localStorage.setItem('filen_bookmarks', JSON.stringify(appState.bookmarks));
      window.dispatchEvent(new CustomEvent('filen-bookmarks-changed'));
    } catch (e) {
      console.warn('Failed to save bookmarks', e);
    }
    this.render();
  }

  open() {
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay';
    overlay.id = 'bookmark-manager-overlay';
    overlay.style.position = 'fixed';
    overlay.style.inset = '0';
    overlay.style.backgroundColor = 'rgba(0,0,0,0.5)';
    overlay.style.display = 'flex';
    overlay.style.alignItems = 'center';
    overlay.style.justifyContent = 'center';
    overlay.style.zIndex = '9999';
    
    overlay.appendChild(this.element);
    document.body.appendChild(overlay);
  }

  close() {
    const overlay = document.getElementById('bookmark-manager-overlay');
    if (overlay) {
      overlay.remove();
    }
  }
}
