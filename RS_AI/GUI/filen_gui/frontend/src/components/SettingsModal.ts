import { appState, saveSettings } from '../store';
import { setLanguage, applyLanguage } from '../i18n';

export class SettingsModal {
  private element: HTMLDivElement;

  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'auth-modal'; // Tạm dùng class auth-modal cho modal đơn giản
    this.element.style.width = '400px';
    
    const title = document.createElement('h2');
    title.textContent = 'Settings';
    
    const s = appState.settings!;
    
    const langLabel = document.createElement('label');
    langLabel.textContent = 'Language';
    const langSelect = document.createElement('select');
    langSelect.innerHTML = `
      <option value="vi" ${s.language === 'vi' ? 'selected' : ''}>Tiếng Việt</option>
      <option value="en" ${s.language === 'en' ? 'selected' : ''}>English</option>
    `;
    
    const themeLabel = document.createElement('label');
    themeLabel.textContent = 'Theme';
    const themeSelect = document.createElement('select');
    themeSelect.innerHTML = `
      <option value="light" ${s.theme === 'light' ? 'selected' : ''}>Light</option>
      <option value="dark" ${s.theme === 'dark' ? 'selected' : ''}>Dark</option>
      <option value="system" ${s.theme === 'system' ? 'selected' : ''}>System</option>
    `;
    
    const hiddenLabel = document.createElement('label');
    hiddenLabel.style.display = 'flex';
    hiddenLabel.style.alignItems = 'center';
    hiddenLabel.style.gap = '8px';
    const hiddenInput = document.createElement('input');
    hiddenInput.type = 'checkbox';
    hiddenInput.checked = s.showHiddenFiles;
    hiddenLabel.appendChild(hiddenInput);
    hiddenLabel.appendChild(document.createTextNode('Show hidden files'));
    
    const actions = document.createElement('div');
    actions.style.display = 'flex';
    actions.style.justifyContent = 'flex-end';
    actions.style.gap = '8px';
    actions.style.marginTop = '20px';
    
    const cancelBtn = document.createElement('button');
    cancelBtn.textContent = 'Cancel';
    cancelBtn.onclick = () => this.close();
    
    const saveBtn = document.createElement('button');
    saveBtn.textContent = 'Save';
    saveBtn.onclick = () => {
      const lang = langSelect.value;
      const theme = themeSelect.value;
      const showHidden = hiddenInput.checked;

      const oldLang = appState.settings!.language;
      const oldTheme = appState.settings!.theme;
      const oldHidden = appState.settings!.showHiddenFiles;

      appState.settings!.language = lang;
      appState.settings!.theme = theme;
      appState.settings!.showHiddenFiles = showHidden;
      saveSettings();

      if (oldLang !== lang) {
        document.body.dataset.langId = lang;
        setLanguage(lang);
        applyLanguage();
      }
      
      if (oldTheme !== theme) {
        document.documentElement.setAttribute('data-theme', theme);
      }

      if (oldHidden !== showHidden) {
        window.dispatchEvent(new CustomEvent('filen-settings-changed'));
      }

      this.close();
    };
    
    actions.appendChild(cancelBtn);
    actions.appendChild(saveBtn);
    
    this.element.appendChild(title);
    this.element.appendChild(langLabel);
    this.element.appendChild(langSelect);
    this.element.appendChild(themeLabel);
    this.element.appendChild(themeSelect);
    this.element.appendChild(hiddenLabel);
    this.element.appendChild(actions);
  }

  open() {
    // Append to body with overlay
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay';
    overlay.id = 'settings-overlay';
    overlay.style.position = 'fixed';
    overlay.style.inset = '0';
    overlay.style.backgroundColor = 'rgba(0,0,0,0.5)';
    overlay.style.display = 'flex';
    overlay.style.alignItems = 'center';
    overlay.style.justifyContent = 'center';
    overlay.style.zIndex = '9999';
    
    overlay.appendChild(this.element);
    document.body.appendChild(overlay);
  }

  close() {
    const overlay = document.getElementById('settings-overlay');
    if (overlay) {
      overlay.remove();
    }
  }
}
