export class NeonProgressBar {
  container: HTMLDivElement;
  bar: HTMLDivElement;
  constructor() {
    this.container = document.createElement('div');
    this.container.className = 'neon-progress-container';
    this.bar = document.createElement('div');
    this.bar.className = 'neon-progress-bar';
    this.container.appendChild(this.bar);
  }
  setProgress(percent: number) {
    this.bar.style.width = `${percent}%`;
  }
  getElement(): HTMLDivElement { return this.container; }
}
