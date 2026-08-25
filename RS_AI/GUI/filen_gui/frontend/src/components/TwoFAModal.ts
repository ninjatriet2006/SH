export class TwoFAModal {
  element: HTMLDivElement;
  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'twofa-modal';
    this.element.innerHTML = `<h2>2FA</h2><input placeholder='Code'/><button>Verify</button>`;
  }
  getElement(): HTMLDivElement { return this.element; }
}
