export class FloatingStatusBar {
  private element: HTMLElement | null = null;
  private timeoutId: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    this.element = document.getElementById('floating-status-bar');
  }

  show(text: string) {
    if (!this.element) {
      this.element = document.getElementById('floating-status-bar');
    }
    if (!this.element) return;
    
    this.element.textContent = text;
    this.element.classList.add('visible');
    
    // Auto-hide slightly delayed if needed, but mostly managed by mouseleave
    if (this.timeoutId) clearTimeout(this.timeoutId);
  }

  hide() {
    if (!this.element) return;
    
    // We use a small delay to avoid flickering when moving between rows
    if (this.timeoutId) clearTimeout(this.timeoutId);
    this.timeoutId = setTimeout(() => {
      if (this.element) {
        this.element.classList.remove('visible');
      }
    }, 100);
  }
}

export const floatingStatusBar = new FloatingStatusBar();
