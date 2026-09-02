/*
[INTEGRITY NOTES]
- Mục đích: Cây thư mục ở sidebar, tải con theo yêu cầu (lazy) khi bung nhánh.
- Trách nhiệm: Render node, gọi `fileOps.listLocal` để lấy thư mục con.
- Tương tác: Nhận callback `onSelect` từ main.ts để điều hướng pane bên trái,
  không truy cập biến toàn cục.
*/
import * as fileOps from '../services/fileOps';
import { joinPath } from '../features/dragDrop';

export class TreeView {
  private element: HTMLElement;
  private rootPath: string;
  private onSelect: (path: string) => void;

  constructor(rootPath: string, onSelect: (path: string) => void) {
    this.rootPath = rootPath;
    this.onSelect = onSelect;
    this.element = document.createElement('div');
    this.element.className = 'tree-view';
    this.renderNode(this.element, this.rootPath, 'Cục bộ');
  }

  public getElement(): HTMLElement {
    return this.element;
  }

  private async renderNode(container: HTMLElement, path: string, label?: string) {
    const nodeEl = document.createElement('div');
    nodeEl.className = 'tree-node';

    const itemEl = document.createElement('div');
    itemEl.className = 'tree-item';
    
    const toggleEl = document.createElement('span');
    toggleEl.className = 'tree-toggle';
    toggleEl.textContent = '▶'; // Mặc định là đóng
    
    const iconEl = document.createElement('span');
    iconEl.className = 'tree-icon';
    iconEl.textContent = '📁';
    
    const labelEl = document.createElement('span');
    labelEl.className = 'tree-label';
    labelEl.textContent = label || path.split('/').filter(Boolean).pop() || path;
    labelEl.title = path;

    itemEl.appendChild(toggleEl);
    itemEl.appendChild(iconEl);
    itemEl.appendChild(labelEl);
    nodeEl.appendChild(itemEl);
    
    const childrenContainer = document.createElement('div');
    childrenContainer.className = 'tree-children';
    childrenContainer.style.display = 'none'; // Ẩn mặc định
    nodeEl.appendChild(childrenContainer);
    
    container.appendChild(nodeEl);

    let loaded = false;

    // Toggle event
    toggleEl.addEventListener('click', async (e) => {
      e.stopPropagation();
      const isExpanded = childrenContainer.style.display === 'block';
      if (isExpanded) {
        childrenContainer.style.display = 'none';
        toggleEl.textContent = '▶';
        itemEl.classList.remove('expanded');
      } else {
        childrenContainer.style.display = 'block';
        toggleEl.textContent = '▼';
        itemEl.classList.add('expanded');
        
        if (!loaded) {
          loaded = true;
          toggleEl.textContent = '↻'; // Loading
          try {
            const files = await fileOps.listLocal(path);
            const dirs = files.filter(f => f.is_dir).sort((a, b) => a.name.localeCompare(b.name));
            if (dirs.length === 0) {
              const empty = document.createElement('div');
              empty.className = 'tree-item tree-empty';
              empty.textContent = '(Trống)';
              childrenContainer.appendChild(empty);
            } else {
              for (const dir of dirs) {
                const childPath = joinPath(path, dir.name);
                this.renderNode(childrenContainer, childPath);
              }
            }
            toggleEl.textContent = '▼';
          } catch (err) {
            console.warn('TreeView load fail:', err);
            toggleEl.textContent = '⚠️';
          }
        }
      }
    });

    // Click nhãn → điều hướng pane (do main.ts quyết định pane nào).
    labelEl.addEventListener('click', (e) => {
      e.stopPropagation();
      this.onSelect(path);
      // Đảm bảo đang ở tab Explorer để thấy kết quả.
      const tab = document.querySelector<HTMLButtonElement>('.nav-tab[data-view="explorer"]');
      if (tab && !tab.classList.contains('active')) tab.click();
    });
  }
}
