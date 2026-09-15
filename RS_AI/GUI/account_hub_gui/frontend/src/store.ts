import { create } from 'zustand';
import { AppDatabase, EmailAccount, Website, RegistrationRecord, AppSettings, AccountStatus } from '../../bridge/types';
import { accountHubApi } from '../../bridge/api';

interface AppStore {
  data: AppDatabase | null;
  isLoading: boolean;
  error: string | null;
  loadData: () => Promise<void>;
  saveEmail: (email: EmailAccount) => Promise<void>;
  deleteEmail: (id: string) => Promise<void>;
  saveWebsite: (website: Website) => Promise<void>;
  deleteWebsite: (id: string) => Promise<void>;
  toggleRegistration: (emailId: string, websiteId: string) => Promise<void>;
  setRegistrationStatus: (emailId: string, websiteId: string, status: AccountStatus) => Promise<void>;
  toggleCheckin: (emailId: string, websiteId: string) => Promise<void>;
  unlinkRegistration: (emailId: string, websiteId: string) => Promise<void>;
  updateRegistration: (record: RegistrationRecord) => Promise<void>;
  saveSettings: (settings: AppSettings) => Promise<void>;
}

export const useAppStore = create<AppStore>((set, get) => ({
  data: null,
  isLoading: false,
  error: null,

  loadData: async () => {
    set({ isLoading: true, error: null });
    try {
      const db = await accountHubApi.getAllData();
      set({ data: db, isLoading: false });
    } catch (err: any) {
      set({ error: err.toString(), isLoading: false });
    }
  },

  saveEmail: async (email: EmailAccount) => {
    try {
      await accountHubApi.saveEmail(email);
      await get().loadData();
    } catch (err: any) {
      set({ error: err.toString() });
    }
  },

  deleteEmail: async (id: string) => {
    try {
      await accountHubApi.deleteEmail(id);
      await get().loadData();
    } catch (err: any) {
      set({ error: err.toString() });
    }
  },

  saveWebsite: async (website: Website) => {
    try {
      await accountHubApi.saveWebsite(website);
      await get().loadData();
    } catch (err: any) {
      set({ error: err.toString() });
    }
  },

  deleteWebsite: async (id: string) => {
    try {
      await accountHubApi.deleteWebsite(id);
      await get().loadData();
    } catch (err: any) {
      set({ error: err.toString() });
    }
  },

  toggleRegistration: async (emailId: string, websiteId: string) => {
    try {
      await accountHubApi.toggleRegistration(emailId, websiteId);
      await get().loadData();
    } catch (err: any) {
      set({ error: err.toString() });
    }
  },

  setRegistrationStatus: async (emailId: string, websiteId: string, status: AccountStatus) => {
    try {
      await accountHubApi.setRegistrationStatus(emailId, websiteId, status);
      await get().loadData();
    } catch (err: any) {
      set({ error: err.toString() });
    }
  },

  toggleCheckin: async (emailId: string, websiteId: string) => {
    try {
      await accountHubApi.toggleCheckin(emailId, websiteId);
      await get().loadData();
    } catch (err: any) {
      set({ error: err.toString() });
    }
  },

  unlinkRegistration: async (emailId: string, websiteId: string) => {
    try {
      await accountHubApi.unlinkRegistration(emailId, websiteId);
      await get().loadData();
    } catch (err: any) {
      set({ error: err.toString() });
    }
  },

  updateRegistration: async (record: RegistrationRecord) => {
    try {
      await accountHubApi.updateRegistration(record);
      await get().loadData();
    } catch (err: any) {
      set({ error: err.toString() });
    }
  },

  saveSettings: async (settings: AppSettings) => {
    try {
      await accountHubApi.saveSettings(settings);
      await get().loadData();
    } catch (err: any) {
      set({ error: err.toString() });
    }
  }
}));
