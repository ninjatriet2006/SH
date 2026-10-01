import { create } from "zustand";

export type NotificationLevel =
  | "info"
  | "start"
  | "stop"
  | "warning"
  | "error"
  | "success"
  | "debug";

export interface LogItem {
  id: string;
  level: NotificationLevel;
  tag?: string;
  message: string;
  timestamp: Date;
}

interface NotificationState {
  logs: LogItem[];
  latestLog: LogItem | null;
  notify: (
    level: NotificationLevel,
    message: string,
    options?: { tag?: string; autoDismiss?: boolean; durationMs?: number }
  ) => void;
  removeLog: (id: string) => void;
  dismissLatest: () => void;
  clearLogs: () => void;
  exportLogs: () => void;
}

export const useNotificationStore = create<NotificationState>((set, get) => ({
  logs: [],
  latestLog: null,

  notify: (level, message, options = {}) => {
    const id = `log-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    const now = new Date();
    const tag = options.tag ?? "APP";

    const logEntry: LogItem = {
      id,
      level,
      tag,
      message,
      timestamp: now,
    };

    set((state) => ({
      logs: [...state.logs, logEntry].slice(-1000), // Chronological order: oldest at top, newest at bottom (Terminal standard)
      latestLog: logEntry,
    }));
  },

  removeLog: (id: string) => {
    set((state) => {
      const nextLogs = state.logs.filter((n) => n.id !== id);
      const nextLatest = state.latestLog?.id === id ? (nextLogs[0] ?? null) : state.latestLog;
      return { logs: nextLogs, latestLog: nextLatest };
    });
  },

  dismissLatest: () => {
    set({ latestLog: null });
  },

  clearLogs: () => {
    set({ logs: [], latestLog: null });
  },

  exportLogs: () => {
    const { logs } = get();
    const content = logs
      .map(
        (l) =>
          `[${l.timestamp.toISOString()}][${l.level.toUpperCase()}][${l.tag ?? "APP"}] ${l.message}`
      )
      .reverse()
      .join("\n");

    const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `universe_manager_logs_${Date.now()}.txt`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  },
}));
