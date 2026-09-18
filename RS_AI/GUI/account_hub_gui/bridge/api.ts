import { invoke } from '@tauri-apps/api/core';
import { AppDatabase, EmailAccount, Website, RegistrationRecord, AppSettings, AccountStatus, Criterion, LoginMethod } from './types';

export const accountHubApi = {
  getAllData: async (): Promise<AppDatabase> => {
    return await invoke('get_all_data');
  },
  saveEmail: async (email: EmailAccount): Promise<EmailAccount> => {
    return await invoke('save_email', { email });
  },
  deleteEmail: async (id: string): Promise<void> => {
    return await invoke('delete_email', { id });
  },
  saveWebsite: async (website: Website): Promise<Website> => {
    return await invoke('save_website', { website });
  },
  deleteWebsite: async (id: string): Promise<void> => {
    return await invoke('delete_website', { id });
  },
  saveCriterion: async (criterion: Criterion): Promise<Criterion> => {
    return await invoke('save_criterion', { criterion });
  },
  deleteCriterion: async (id: string): Promise<void> => {
    return await invoke('delete_criterion', { id });
  },
  saveLoginMethod: async (loginMethod: LoginMethod): Promise<LoginMethod> => {
    return await invoke('save_login_method', { loginMethod });
  },
  deleteLoginMethod: async (id: string): Promise<void> => {
    return await invoke('delete_login_method', { id });
  },
  toggleRegistration: async (emailId: string, websiteId: string): Promise<RegistrationRecord> => {
    return await invoke('toggle_registration', { emailId, websiteId });
  },
  setRegistrationStatus: async (emailId: string, websiteId: string, status: AccountStatus): Promise<RegistrationRecord> => {
    return await invoke('set_registration_status', { emailId, websiteId, status });
  },
  toggleCheckin: async (emailId: string, websiteId: string): Promise<RegistrationRecord> => {
    return await invoke('toggle_checkin', { emailId, websiteId });
  },
  unlinkRegistration: async (emailId: string, websiteId: string): Promise<void> => {
    return await invoke('unlink_registration', { emailId, websiteId });
  },
  updateRegistration: async (record: RegistrationRecord): Promise<RegistrationRecord> => {
    return await invoke('update_registration', { record });
  },
  getSettings: async (): Promise<AppSettings> => {
    return await invoke('get_settings');
  },
  saveSettings: async (settings: AppSettings): Promise<void> => {
    return await invoke('save_settings', { settings });
  },
  getLanguages: async (): Promise<any[]> => {
    return await invoke('get_available_languages');
  },
  getThemes: async (): Promise<any[]> => {
    return await invoke('get_available_themes');
  }
};
