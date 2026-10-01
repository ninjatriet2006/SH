import { create } from "zustand";
import type { AppEntry, DetectionReport, JobEvent, ManagerConfig, SearchReport } from "../utils/contract";
import { api, ipcError, JobClient } from "../utils/ipc";

const jobClient = new JobClient();

interface AppState {
  config: ManagerConfig | null;
  configSaved: boolean;
  busy: boolean;
  progress: string;
  error: string;
  detection: DetectionReport | null;
  search: SearchReport | null;

  loadConfig: () => Promise<void>;
  saveConfig: () => Promise<void>;
  setManagedDir: (path: string) => void;
  scanApps: () => Promise<void>;
  detectApp: (path: string) => Promise<void>;
  startApp: (appId: string) => Promise<void>;
  stopApp: (appId: string) => Promise<void>;
  searchApps: (query: string) => Promise<void>;
  pickManagedDir: () => Promise<void>;
  pickAndDetect: () => Promise<void>;
  clearError: () => void;
  clearDetection: () => void;
  dispose: () => void;
}

export const useAppStore = create<AppState>((set, get) => {
  async function runJob(
    command: "scan_apps" | "detect_app" | "start_app" | "stop_app" | "search_apps",
    payload: object,
  ): Promise<unknown> {
    set({ busy: true, error: "", progress: "Starting…" });
    try {
      const response = await jobClient.run(command, payload as never, (event: JobEvent<unknown>) => {
        set({ progress: event.payload.message ?? event.state });
        const result = event.payload.result;
        if (result?.status === "failed" || result?.status === "cancelled") {
          set({ error: result.error.message });
        }
      });
      set({ busy: false });
      return response.data;
    } catch (error) {
      set({ error: ipcError(error).message, busy: false });
      return null;
    }
  }

  return {
    config: null,
    configSaved: false,
    busy: false,
    progress: "",
    error: "",
    detection: null,
    search: null,

    loadConfig: async () => {
      try {
        const config = await api.loadConfig();
        set({ config, configSaved: true, error: "" });
      } catch (error) {
        set({ error: ipcError(error).message });
      }
    },

    saveConfig: async () => {
      const { config } = get();
      if (!config) return;
      set({ busy: true, error: "" });
      try {
        const saved = await api.saveConfig(config);
        set({ config: saved, configSaved: true, busy: false, progress: "Saved" });
      } catch (error) {
        set({ configSaved: false, error: ipcError(error).message, busy: false });
      }
    },

    setManagedDir: (path: string) => {
      const { config } = get();
      const updated = config
        ? { ...config, settings: { ...config.settings, managed_dir: path } }
        : { settings: { managed_dir: path }, apps: [] };
      set({ config: updated, configSaved: false });
    },

    scanApps: async () => {
      const result = await runJob("scan_apps", {});
      if (Array.isArray(result)) {
        const { config } = get();
        if (config) {
          set({ config: { ...config, apps: result as AppEntry[] }, configSaved: false });
          await get().saveConfig();
        }
      }
    },

    detectApp: async (path: string) => {
      const result = await runJob("detect_app", { path: { path } });
      if (result) set({ detection: result as DetectionReport });
    },

    startApp: async (appId: string) => {
      await runJob("start_app", { app_id: appId, confirmed: true });
    },

    stopApp: async (appId: string) => {
      await runJob("stop_app", { app_id: appId, confirmed: true });
    },

    searchApps: async (query: string) => {
      const result = await runJob("search_apps", { query });
      if (result) set({ search: result as SearchReport });
    },

    pickManagedDir: async () => {
      try {
        const selected = await api.pickerSelect("managed");
        get().setManagedDir(selected.path.path);
      } catch (error) {
        set({ error: ipcError(error).message });
      }
    },

    pickAndDetect: async () => {
      try {
        const selected = await api.pickerSelect("source");
        await get().detectApp(selected.path.path);
      } catch (error) {
        set({ error: ipcError(error).message });
      }
    },

    clearError: () => set({ error: "" }),
    clearDetection: () => set({ detection: null }),
    dispose: () => jobClient.dispose(),
  };
});
