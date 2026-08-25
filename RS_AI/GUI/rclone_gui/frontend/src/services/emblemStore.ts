// emblemStore.ts - Quản lý custom emblems cho files/folders
// Lưu trạng thái: Dictionary { 'absolute_path': ['⭐', '🔒', ...] }

type EmblemDict = Record<string, string[]>;

class EmblemStore {
  private emblems: EmblemDict = {};

  constructor() {
    try {
      const saved = localStorage.getItem('filen_emblems');
      if (saved) {
        this.emblems = JSON.parse(saved);
      }
    } catch (e) {
      console.warn('Failed to parse emblems from localStorage', e);
    }
  }

  private save() {
    localStorage.setItem('filen_emblems', JSON.stringify(this.emblems));
    window.dispatchEvent(new Event('filen-emblems-changed'));
  }

  public getEmblems(path: string): string[] {
    return this.emblems[path] || [];
  }

  public toggleEmblem(path: string, emblem: string) {
    if (!this.emblems[path]) {
      this.emblems[path] = [];
    }
    
    const idx = this.emblems[path].indexOf(emblem);
    if (idx >= 0) {
      // Remove
      this.emblems[path].splice(idx, 1);
      if (this.emblems[path].length === 0) {
        delete this.emblems[path];
      }
    } else {
      // Add
      this.emblems[path].push(emblem);
    }
    this.save();
  }

  public clearEmblems(path: string) {
    if (this.emblems[path]) {
      delete this.emblems[path];
      this.save();
    }
  }
}

export const emblemStore = new EmblemStore();
