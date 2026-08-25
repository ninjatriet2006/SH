import { invoke } from '@tauri-apps/api/core';
import type { FileItem } from '../store';

export interface CustomAction {
  id: string;
  name: string;
  exec: string;
  icon: string;
  selection: string;
  extensions: string[];
}

class ActionStore {
  private actions: CustomAction[] = [];

  constructor() {
    this.fetchActions();
  }

  public async fetchActions() {
    try {
      this.actions = await invoke('sys_get_custom_actions');
      console.log('Loaded custom actions:', this.actions);
    } catch (e) {
      console.error('Failed to load custom actions:', e);
    }
  }

  public getValidActionsForSelection(files: FileItem[]): CustomAction[] {
    if (this.actions.length === 0) return [];
    
    // Evaluate rules
    const selCount = files.length;
    
    return this.actions.filter(action => {
      // 1. Check selection rule
      if (action.selection === 's' && selCount !== 1) return false;
      if (action.selection === 'm' && selCount < 2) return false;
      // if 'any', bypass count check (except maybe > 0)
      if (selCount === 0) return false;

      // 2. Check extensions rule
      // If extensions has "any", it matches anything
      if (action.extensions.includes('any')) return true;

      // Check if all selected files match the extensions
      return files.every(f => {
        if (f.is_dir && action.extensions.includes('dir')) return true;
        
        const ext = f.name.split('.').pop()?.toLowerCase() || '';
        return action.extensions.includes(ext);
      });
    });
  }

  public async executeAction(action: CustomAction, files: FileItem[], basePath: string) {
    const paths = files.map(f => {
      let p = '';
      if (basePath.startsWith('trash://')) {
        p = `${basePath}/${f.name}`;
      } else {
        p = basePath === '/' ? `/${f.name}` : `${basePath}/${f.name}`;
      }
      return p;
    });

    try {
      await invoke('sys_execute_custom_action', { 
        execTemplate: action.exec, 
        filePaths: paths 
      });
    } catch (e) {
      console.error(`Failed to execute custom action ${action.name}:`, e);
      alert(`Error executing action: ${e}`);
    }
  }
}

export const actionStore = new ActionStore();
