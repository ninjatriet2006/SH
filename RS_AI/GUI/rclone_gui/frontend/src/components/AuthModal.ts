import { invoke } from '@tauri-apps/api/core';
import { appState } from '../store';
export class AuthModal {
  element: HTMLDivElement;
  private emailInput: HTMLInputElement;
  private passwordInput: HTMLInputElement;
  private keepLoggedInput: HTMLInputElement;
  private twofaInput: HTMLInputElement;
  private errorLabel: HTMLDivElement;
  private submitBtn: HTMLButtonElement;
  private pendingTwoFA: boolean = false;
  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'auth-modal';
    // Build UI
    const title = document.createElement('h2');
    title.textContent = 'Login';
    const email = document.createElement('input');
    email.placeholder = 'Email';
    this.emailInput = email;
    const password = document.createElement('input');
    password.type = 'password';
    password.placeholder = 'Password';
    this.passwordInput = password;
    const keep = document.createElement('input');
    keep.type = 'checkbox';
    keep.checked = true;
    this.keepLoggedInput = keep;
    const keepLabel = document.createElement('label');
    keepLabel.textContent = 'Keep logged in';
    keepLabel.appendChild(keep);
    const twofa = document.createElement('input');
    twofa.placeholder = '2FA code';
    twofa.style.display = 'none';
    this.twofaInput = twofa;
    const err = document.createElement('div');
    err.className = 'auth-error';
    this.errorLabel = err;
    const btn = document.createElement('button');
    btn.textContent = 'Login';
    this.submitBtn = btn;
    btn.addEventListener('click', () => this.submit());
    // Append
    this.element.append(title, email, password, keepLabel, twofa, err, btn);
  }
  async submit() {
    const email = this.emailInput.value.trim();
    const password = this.passwordInput.value;
    const keep = this.keepLoggedInput.checked;
    if (!email || !password) {
      this.errorLabel.textContent = 'Please fill email and password.';
      return;
    }
    try {
      if (!this.pendingTwoFA) {
        await invoke('auth_login_terminal', { email, password, twofaCode: null, keepLogged: keep });
        // success
        appState.auth = { user: email };
        this.close();
      } else {
        const code = this.twofaInput.value.trim();
        if (!code) {
          this.errorLabel.textContent = 'Enter 2FA code.';
          return;
        }
        await invoke('auth_login_twofa_terminal', { email, password, twofaCode: code, keepLogged: keep });
        appState.auth = { user: email };
        this.close();
      }
    } catch (e: any) {
      const msg = e as string;
      if (msg.includes('2FA_REQUIRED')) {
        this.pendingTwoFA = true;
        this.twofaInput.style.display = 'block';
        this.submitBtn.textContent = 'Verify 2FA';
        this.errorLabel.textContent = '';
      } else {
        this.errorLabel.textContent = msg;
      }
    }
  }
  close() {
    document.body.removeChild(this.element);
    // enable logout button, update UI
    const logoutBtn = document.getElementById('btn-logout') as HTMLButtonElement;
    if (logoutBtn) {
      logoutBtn.disabled = false;
      logoutBtn.classList.remove('disabled');
    }
    const loginBtn = document.getElementById('btn-login') as HTMLButtonElement;
    if (loginBtn) {
      loginBtn.disabled = true;
      loginBtn.classList.add('disabled');
    }
    const accountPill = document.getElementById('account-pill');
    if (accountPill && appState.auth?.user) {
      accountPill.textContent = `👤 ${appState.auth.user}`;
    }
    // Explorer đã mount sẵn từ khởi động — chỉ cần load lại cloud pane (right)
    const explorer = (window as any).__explorer;
    if (explorer) {
      explorer.loadPane('right', appState.explorer?.rightPath ?? '/');
    }
  }
  open() {
    document.body.appendChild(this.element);
  }
  getElement(): HTMLDivElement { return this.element; }
}

