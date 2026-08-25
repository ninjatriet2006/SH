import { PaneView, type PaneViewOptions } from './PaneView';
import { type Pane, getPanePath } from '../../services/explorerStore';

export interface PaneContainerTab {
  id: string;
  view: PaneView;
  path: string;
}

export class PaneContainer {
  container: HTMLDivElement;
  tabBar: HTMLDivElement;
  contentArea: HTMLDivElement;
  
  tabs: PaneContainerTab[] = [];
  activeTabId: string | null = null;
  
  private config: PaneViewOptions;
  private paneSide: Pane;
  
  // Callback when a tab is activated or created, so DualPaneExplorer can reload it
  public onTabSwitch?: (path: string) => void;

  constructor(config: PaneViewOptions) {
    this.config = config;
    this.paneSide = config.side;
    
    this.container = document.createElement('div');
    this.container.className = 'pane-container';
    this.container.style.display = 'flex';
    this.container.style.flexDirection = 'column';
    this.container.style.height = '100%';
    this.container.style.minWidth = '0'; // prevent flex overflow
    this.container.style.position = 'relative';
    
    this.tabBar = document.createElement('div');
    this.tabBar.className = 'tab-bar';
    
    this.contentArea = document.createElement('div');
    this.contentArea.className = 'pane-content-area';
    this.contentArea.style.flex = '1';
    this.contentArea.style.minHeight = '0';
    this.contentArea.style.display = 'flex';
    this.contentArea.style.flexDirection = 'column';
    
    this.container.appendChild(this.tabBar);
    this.container.appendChild(this.contentArea);
    
    // Create initial tab
    this.addTab(this.paneSide === 'left' ? '/' : 'trash://remote'); // Initial paths
  }
  
  public getElement(): HTMLElement {
    return this.container;
  }
  
  public addTab(initialPath: string = '/') {
    const id = Date.now().toString() + Math.random().toString(36).substr(2, 5);
    const view = new PaneView(this.config);
    this.tabs.push({ id, view, path: initialPath });
    this.setActiveTab(id);
  }
  
  public closeTab(id: string) {
    const idx = this.tabs.findIndex(t => t.id === id);
    if (idx === -1) return;
    
    this.tabs.splice(idx, 1);
    
    if (this.tabs.length === 0) {
      // User requested: Mở lại một tab Home mặc định
      this.addTab(this.paneSide === 'left' ? '/' : 'trash://remote');
      return;
    }
    
    if (this.activeTabId === id) {
      // Switch to the previous tab or next tab
      const nextIdx = Math.max(0, idx - 1);
      this.setActiveTab(this.tabs[nextIdx].id);
    } else {
      this.renderTabBar();
    }
  }
  
  public setActiveTab(id: string) {
    // 1. Save current state to the old active tab
    if (this.activeTabId) {
      const oldTab = this.tabs.find(t => t.id === this.activeTabId);
      if (oldTab) {
        oldTab.path = getPanePath(this.paneSide);
        // We could save files/selection here, but DualPaneExplorer will just reload the path
      }
    }
    
    this.activeTabId = id;
    const tab = this.tabs.find(t => t.id === id);
    if (!tab) return;
    
    // 2. Mount view
    this.contentArea.innerHTML = '';
    this.contentArea.appendChild(tab.view.getElement());
    
    // 3. Render tab bar
    this.renderTabBar();
    
    // 4. Notify DualPaneExplorer to load this path
    if (this.onTabSwitch) {
      this.onTabSwitch(tab.path);
    }
  }
  
  public updateActiveTabPath(path: string) {
    const tab = this.tabs.find(t => t.id === this.activeTabId);
    if (tab) {
      tab.path = path;
      this.renderTabBar(); // Update title
    }
  }
  
  private renderTabBar() {
    this.tabBar.innerHTML = '';
    
    this.tabs.forEach(tab => {
      const el = document.createElement('div');
      el.className = `tab-item ${tab.id === this.activeTabId ? 'active' : ''}`;
      
      const title = document.createElement('span');
      title.className = 'tab-title';
      // extract basename
      let name = tab.path.split('/').pop() || tab.path;
      if (tab.path === '/' || tab.path === '') name = this.paneSide === 'left' ? 'Computer' : 'Cloud';
      if (tab.path === 'trash://remote') name = 'Trash (Cloud)';
      if (tab.path === 'trash://local') name = 'Trash (Local)';
      
      title.textContent = name;
      title.title = tab.path;
      
      const closeBtn = document.createElement('span');
      closeBtn.className = 'tab-close';
      closeBtn.innerHTML = '&times;';
      closeBtn.onclick = (e) => {
        e.stopPropagation();
        this.closeTab(tab.id);
      };
      
      el.onclick = () => {
        if (tab.id !== this.activeTabId) {
          this.setActiveTab(tab.id);
        }
      };
      
      el.appendChild(title);
      if (this.tabs.length > 1) {
        el.appendChild(closeBtn);
      }
      this.tabBar.appendChild(el);
    });
    
    const addBtn = document.createElement('div');
    addBtn.className = 'tab-add';
    addBtn.innerHTML = '+';
    addBtn.title = 'New Tab (Ctrl+T)';
    addBtn.onclick = () => {
      this.addTab(this.paneSide === 'left' ? '/' : 'trash://remote');
    };
    this.tabBar.appendChild(addBtn);
  }
  
  // -- Proxy methods to active PaneView --
  
  public get activeView(): PaneView | null {
    const tab = this.tabs.find(t => t.id === this.activeTabId);
    return tab ? tab.view : null;
  }
  
  public get table() {
    return this.activeView?.table;
  }
  
  public get toolbar() {
    return this.activeView?.toolbar;
  }
  
  public renderPlaceholder(msg: string, path: string, action?: any) {
    this.activeView?.renderPlaceholder(msg, path, action);
  }
  
  public render(files: any[], path: string, sort: any, widths: any) {
    this.activeView?.render(files, path, sort, widths);
    this.updateActiveTabPath(path);
  }
}
