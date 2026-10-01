import { create } from "zustand";
import type { AppEntry, DetectionReport, JobEvent, ManagerConfig, SearchReport } from "../utils/contract";
import { api, ipcError, JobClient } from "../utils/ipc";
import { useNotificationStore } from "./useNotificationStore";

const jobClient = new JobClient();

const notify = (
  level: "info" | "start" | "stop" | "warning" | "error" | "success" | "debug",
  msg: string,
  tag = "APP"
) => {
  useNotificationStore.getState().notify(level, msg, { tag });
};

interface AppState {
  config: ManagerConfig | null;
  configSaved: boolean;
  busy: boolean;
  isSyncing: boolean;
  lastSyncTime: Date | null;
  progress: string;
  error: string;
  detection: DetectionReport | null;
  search: SearchReport | null;

  loadConfig: () => Promise<void>;
  saveConfig: () => Promise<void>;
  setManagedDir: (path: string) => void;
  scanApps: (silent?: boolean) => Promise<void>;
  detectApp: (path: string) => Promise<void>;
  startApp: (appId: string) => Promise<void>;
  stopApp: (appId: string) => Promise<void>;
  restartApp: (appId: string) => Promise<void>;
  searchApps: (query: string) => Promise<void>;
  pickManagedDir: () => Promise<void>;
  pickAndDetect: () => Promise<void>;
  clearError: () => void;
  clearDetection: () => void;
  integrateApp: (app: AppEntry) => Promise<void>;
  dispose: () => void;
}

export const useAppStore = create<AppState>((set, get) => {
  async function runJob(
    command: "scan_apps" | "detect_app" | "start_app" | "stop_app" | "search_apps",
    payload: object,
    silent = false,
  ): Promise<unknown> {
    if (!silent) {
      set({ busy: true, error: "", progress: "Starting…" });
    }
    try {
      const response = await jobClient.run(command, payload as never, (event: JobEvent<unknown>) => {
        if (!silent) {
          set({ progress: event.payload.message ?? event.state });
        }
        const result = event.payload.result;
        if (result?.status === "failed" || result?.status === "cancelled") {
          set({ error: result.error.message });
          notify("error", result.error.message, command.toUpperCase());
        }
      });
      if (!silent) {
        set({ busy: false });
      }
      return response.data;
    } catch (error) {
      const msg = ipcError(error).message;
      set({ error: msg, busy: false, isSyncing: false });
      notify("error", msg, command.toUpperCase());
      return null;
    }
  }

  return {
    config: null,
    configSaved: false,
    busy: false,
    isSyncing: false,
    lastSyncTime: null,
    progress: "",
    error: "",
    detection: null,
    search: null,

    loadConfig: async () => {
      try {
        notify("info", "Đang tải cấu hình ứng dụng...", "CONFIG");
        const config = await api.loadConfig();
        set({ config, configSaved: true, error: "" });
        notify(
          "success",
          `Cấu hình đã tải thành công. (${config.apps.length} ứng dụng đã đăng ký)`,
          "CONFIG"
        );
      } catch (error) {
        const msg = ipcError(error).message;
        set({ error: msg });
        notify("error", `Không thể tải cấu hình: ${msg}`, "CONFIG");
      }
    },

    saveConfig: async () => {
      const { config } = get();
      if (!config) return;
      set({ busy: true, error: "" });
      notify("info", "Đang lưu cấu hình...", "CONFIG");
      try {
        const saved = await api.saveConfig(config);
        set({ config: saved, configSaved: true, busy: false, progress: "Saved" });
        notify("success", "Cấu hình đã được lưu an toàn.", "CONFIG");
      } catch (error) {
        const msg = ipcError(error).message;
        set({ configSaved: false, error: msg, busy: false });
        notify("error", `Lưu cấu hình thất bại: ${msg}`, "CONFIG");
      }
    },

    setManagedDir: (path: string) => {
      const { config } = get();
      const updated = config
        ? { ...config, settings: { ...config.settings, managed_dir: path } }
        : { settings: { managed_dir: path }, apps: [] };
      set({ config: updated, configSaved: false });
      notify("info", `Đã thay đổi đường dẫn thư mục quản lý: "${path}"`, "CONFIG");
    },

    scanApps: async (silent = false) => {
      if (silent && get().isSyncing) return;
      if (silent) {
        set({ isSyncing: true });
      } else {
        notify("start", "Bắt đầu quét toàn diện các ứng dụng hệ thống & portable...", "SCAN");
      }
      const result = await runJob("scan_apps", {}, silent);
      if (Array.isArray(result)) {
        const { config } = get();
        const nextApps = result as AppEntry[];
        if (config) {
          set({
            config: { ...config, apps: nextApps },
            isSyncing: false,
            lastSyncTime: new Date(),
          });
          if (!silent) {
            notify("success", `Quét hoàn tất: đã phát hiện ${nextApps.length} ứng dụng.`, "SCAN");
          }
        } else {
          set({
            config: { settings: { managed_dir: "" }, apps: nextApps },
            isSyncing: false,
            lastSyncTime: new Date(),
          });
          if (!silent) {
            notify("success", `Quét hoàn tất: đã phát hiện ${nextApps.length} ứng dụng.`, "SCAN");
          }
        }
      } else {
        set({ isSyncing: false });
        if (!silent) {
          notify("error", "Quá trình quét ứng dụng bị gián đoạn hoặc thất bại.", "SCAN");
        }
      }
    },

    detectApp: async (path: string) => {
      notify("start", `Đang kiểm tra gói ứng dụng tại "${path}"...`, "DETECT");
      const result = await runJob("detect_app", { path: { path } });
      if (result) {
        const report = result as DetectionReport;
        set({ detection: report });
        notify(
          "success",
          `Phát hiện: ${report.suggested_name} (${report.executables.length} tệp thực thi, ${report.icons.length} biểu tượng)`,
          "DETECT"
        );
      } else {
        notify("error", `Kiểm tra ứng dụng tại "${path}" thất bại.`, "DETECT");
      }
    },

    integrateApp: async (app: AppEntry) => {
      const { config, saveConfig, scanApps } = get();
      if (!config) return;
      notify("start", `Đang cài đặt và tích hợp ứng dụng "${app.name}"...`, "INSTALL");
      const existingIdx = config.apps.findIndex(a => a.id === app.id);
      let updatedApps: AppEntry[];
      if (existingIdx >= 0) {
        updatedApps = [...config.apps];
        updatedApps[existingIdx] = app;
      } else {
        updatedApps = [...config.apps, app];
      }
      set({ config: { ...config, apps: updatedApps }, detection: null });
      await saveConfig();
      notify("success", `Ứng dụng "${app.name}" đã được tích hợp thành công!`, "INSTALL");
      await scanApps(true);
    },

    startApp: async (appId: string) => {
      notify("start", `Đang gửi lệnh khởi chạy ứng dụng "${appId}"...`, "LIFECYCLE");
      const res = await runJob("start_app", { app_id: appId, confirmed: true });
      if (res) {
        notify("success", `Ứng dụng "${appId}" đã được khởi chạy thành công.`, "LIFECYCLE");
      } else {
        notify("error", `Không thể khởi chạy ứng dụng "${appId}".`, "LIFECYCLE");
      }
      setTimeout(() => void get().scanApps(true), 600);
    },

    stopApp: async (appId: string) => {
      notify("stop", `Đang gửi lệnh dừng tiến trình "${appId}"...`, "LIFECYCLE");
      const res = await runJob("stop_app", { app_id: appId, confirmed: true });
      if (res) {
        notify("success", `Ứng dụng "${appId}" đã được dừng an toàn.`, "LIFECYCLE");
      } else {
        notify("error", `Không thể dừng ứng dụng "${appId}".`, "LIFECYCLE");
      }
      setTimeout(() => void get().scanApps(true), 600);
    },

    restartApp: async (appId: string) => {
      notify("start", `Đang khởi động lại ứng dụng "${appId}"...`, "LIFECYCLE");
      await get().stopApp(appId);
      await new Promise((r) => setTimeout(r, 600));
      await get().startApp(appId);
      await new Promise((r) => setTimeout(r, 400));
      await get().scanApps(true);
    },

    searchApps: async (query: string) => {
      notify("info", `Đang tìm kiếm ứng dụng với từ khóa "${query}"...`, "SEARCH");
      const result = await runJob("search_apps", { query });
      if (result) {
        const report = result as SearchReport;
        set({ search: report });
        notify("info", `Tìm thấy ${report.results.length} kết quả phù hợp cho "${query}".`, "SEARCH");
      } else {
        notify("error", `Tìm kiếm cho từ khóa "${query}" thất bại.`, "SEARCH");
      }
    },

    pickManagedDir: async () => {
      try {
        notify("info", "Đang mở hộp thoại chọn thư mục quản lý...", "PICKER");
        const selected = await api.pickerSelect("managed");
        get().setManagedDir(selected.path.path);
        notify("success", `Đã chọn thư mục quản lý: "${selected.path.path}"`, "PICKER");
      } catch (error) {
        const msg = ipcError(error).message;
        set({ error: msg });
        notify("warning", `Hủy hoặc lỗi chọn thư mục quản lý: ${msg}`, "PICKER");
      }
    },

    pickAndDetect: async () => {
      try {
        notify("info", "Đang mở hộp thoại chọn thư mục nguồn để kiểm tra...", "PICKER");
        const selected = await api.pickerSelect("source");
        notify("info", `Đã chọn thư mục nguồn: "${selected.path.path}"`, "PICKER");
        await get().detectApp(selected.path.path);
      } catch (error) {
        const msg = ipcError(error).message;
        set({ error: msg });
        notify("warning", `Hủy hoặc lỗi chọn thư mục nguồn: ${msg}`, "PICKER");
      }
    },

    clearError: () => set({ error: "" }),
    clearDetection: () => set({ detection: null }),
    dispose: () => jobClient.dispose(),
  };
});
