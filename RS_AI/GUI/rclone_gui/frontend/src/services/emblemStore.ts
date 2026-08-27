/*
[INTEGRITY NOTES]
- Mục đích: Quản lý biểu tượng tùy chỉnh (Custom Emblems) được ghim trên files/folders.
- Trách nhiệm: Lưu trữ trạng thái Emblem thành Dictionary dạng `{'đường_dẫn_tuyệt_đối': ['⭐', '🔒', ...]}` và đồng bộ với localStorage.
- Tương tác: Gọi từ FileList UI để hiển thị, ContextMenu để toggle.
*/

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
      // Bỏ gỡ (Remove) nếu đã tồn tại biểu tượng
      this.emblems[path].splice(idx, 1);
      if (this.emblems[path].length === 0) {
        delete this.emblems[path];
      }
    } else {
      // Thêm mới (Add) nếu chưa có
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
