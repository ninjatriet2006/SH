import * as fileOps from '../services/fileOps';

function joinPath(dir: string, name: string): string {
  return dir.endsWith('/') ? dir + name : dir + '/' + name;
}

export class TreeView {
  private element: HTMLElement;
  private rootPath: string;

  constructor(rootPath: string) {
    this.rootPath = rootPath;
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

    // Click event to open in DualPaneExplorer
    labelEl.addEventListener('click', async (e) => {
      e.stopPropagation();
      // Dynamically import switchView doesn't work if switchView is not exported.
      // Let's dispatch a custom event instead, or rely on window.__explorer.
      const explorer = (window as any).__explorer;
      if (explorer) {
        explorer.loadPane('left', path);
        // We also need to activate the 'explorer' view.
        // We can click the nav-tab.
        const tab = document.querySelector<HTMLButtonElement>('.nav-tab[data-view="explorer"]');
        if (tab) tab.click();
      }
    });
  }
}
