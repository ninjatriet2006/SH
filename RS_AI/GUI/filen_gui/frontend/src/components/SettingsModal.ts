import { appState, saveSettings } from '../store';
import { setLanguage, applyLanguage } from '../i18n';
import { t, supportedLanguages } from '../i18n';
import { applyFont, applyTheme, availableFonts, availableThemes } from '../appearance';

export class SettingsModal {
  private element: HTMLDivElement;

  constructor() {
    this.element = document.createElement('div');
    this.element.className = 'auth-modal'; // Tạm dùng class auth-modal cho modal đơn giản
    this.element.style.width = '400px';
    
    const title = document.createElement('h2');
    title.textContent = t('settings_title');
    
    const s = appState.settings!;
    
    const langLabel = document.createElement('label');
    langLabel.textContent = t('settings_language');
    langLabel.htmlFor = 'settings-language';
    const langSelect = document.createElement('select');
    langSelect.id = 'settings-language';
    for (const language of supportedLanguages()) {
      langSelect.add(new Option(language === 'vi' ? 'Tiếng Việt' : 'English', language, false, s.language === language));
    }
    
    const themeLabel = document.createElement('label');
    themeLabel.textContent = t('settings_theme');
    themeLabel.htmlFor = 'settings-theme';
    const themeSelect = document.createElement('select');
    themeSelect.id = 'settings-theme';
    themeSelect.add(new Option(t('settings_default'), 'default', false, s.theme === 'default'));
    for (const theme of availableThemes()) {
      themeSelect.add(new Option(theme.name, theme.slug, false, s.theme === theme.slug));
    }

    const fontLabel = document.createElement('label');
    fontLabel.textContent = t('settings_font');
    fontLabel.htmlFor = 'settings-font';
    const fontSelect = document.createElement('select');
    fontSelect.id = 'settings-font';
    fontSelect.add(new Option(t('settings_default'), 'default', false, s.font === 'default'));
    for (const font of availableFonts()) {
      fontSelect.add(new Option(font.name, font.id, false, s.font === font.id));
    }
    
    const hiddenLabel = document.createElement('label');
    hiddenLabel.style.display = 'flex';
    hiddenLabel.style.alignItems = 'center';
    hiddenLabel.style.gap = '8px';
    const hiddenInput = document.createElement('input');
    hiddenInput.type = 'checkbox';
    hiddenInput.checked = s.showHiddenFiles;
    hiddenLabel.appendChild(hiddenInput);
    hiddenLabel.appendChild(document.createTextNode(t('settings_hidden')));
    
    const actions = document.createElement('div');
    actions.style.display = 'flex';
    actions.style.justifyContent = 'flex-end';
    actions.style.gap = '8px';
    actions.style.marginTop = '20px';
    
    const cancelBtn = document.createElement('button');
    cancelBtn.textContent = t('settings_cancel');
    cancelBtn.onclick = () => this.close();
    
    const saveBtn = document.createElement('button');
    saveBtn.textContent = t('settings_save');
    saveBtn.onclick = async () => {
      const lang = langSelect.value;
      const theme = themeSelect.value;
      const showHidden = hiddenInput.checked;
      const font = fontSelect.value;

      const oldLang = appState.settings!.language;
      const oldTheme = appState.settings!.theme;
      const oldHidden = appState.settings!.showHiddenFiles;

      appState.settings!.language = lang;
      appState.settings!.theme = theme;
      appState.settings!.showHiddenFiles = showHidden;
      appState.settings!.font = font;

      if (oldLang !== lang) {
        document.documentElement.lang = lang;
        setLanguage(lang);
        applyLanguage();
      }
      
      if (oldTheme !== theme) {
        appState.settings!.theme = applyTheme(theme);
      }

      try {
        appState.settings!.font = await applyFont(font);
      } catch (error) {
        console.warn('[fonts] failed to apply font', error);
        appState.settings!.font = await applyFont('default');
      }
      saveSettings();

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
    this.element.appendChild(fontLabel);
    this.element.appendChild(fontSelect);
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
