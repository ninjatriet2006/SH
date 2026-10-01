import type { AppEntry } from "./contract";

export interface AppPaths {
  configDir: string;
  dataDir: string;
  cacheDir: string;
  shareDir?: string;
}

export function getRealId(id: string): string {
  if (id.endsWith("-flatpak")) {
    return id.slice(0, -"-flatpak".length);
  }
  if (id.endsWith("-snap")) {
    return id.slice(0, -"-snap".length);
  }
  return id;
}

export function getAppPaths(app: AppEntry): AppPaths {
  const realId = getRealId(app.id);
  const ptype = (app.package_type ?? "APT").toLowerCase();

  if (ptype === "flatpak") {
    return {
      configDir: `~/.var/app/${realId}/config`,
      dataDir: `~/.var/app/${realId}/data`,
      cacheDir: `~/.var/app/${realId}/cache`,
      shareDir: `/var/lib/flatpak/app/${realId}`,
    };
  }

  if (ptype === "snap") {
    return {
      configDir: `~/snap/${realId}/current/.config`,
      dataDir: `~/snap/${realId}/current`,
      cacheDir: `~/snap/${realId}/common/.cache`,
      shareDir: `/snap/${realId}`,
    };
  }

  // APT, Local or System
  return {
    configDir: `~/.config/${realId}`,
    dataDir: `~/.local/share/${realId}`,
    cacheDir: `~/.cache/${realId}`,
    shareDir: `/usr/share/${realId}`,
  };
}

export function detectFramework(app: AppEntry): string {
  const haystack = `${app.exec_path} ${app.install_path} ${app.desktop_file} ${app.name} ${app.id}`.toLowerCase();

  if (haystack.includes("app.asar") || haystack.includes("electron")) {
    return "Electron";
  }
  if (haystack.includes("flutter")) {
    return "Flutter";
  }
  if (
    haystack.includes("qt5") ||
    haystack.includes("qt6") ||
    haystack.includes("cinnamon") ||
    haystack.includes("hotcorner") ||
    haystack.includes("vlc") ||
    haystack.includes("kde") ||
    haystack.includes("dolphin") ||
    haystack.includes("wireshark")
  ) {
    return "Qt (C++)";
  }
  if (haystack.includes(".jar") || haystack.includes("jvm") || haystack.includes("java")) {
    return "Java";
  }
  if (haystack.includes("python") || haystack.includes("idle")) {
    return "Python";
  }
  if (
    haystack.includes("gtk") ||
    haystack.includes("gnome") ||
    haystack.includes("gimp") ||
    haystack.includes("evince") ||
    haystack.includes("gedit")
  ) {
    return "GTK (C/Python)";
  }
  if (haystack.includes("rust") || haystack.includes("cargo")) {
    return "Rust";
  }
  if ((app.package_type ?? "").toLowerCase() === "flatpak") {
    return "Flatpak (Container)";
  }
  if ((app.package_type ?? "").toLowerCase() === "snap") {
    return "Snap (AppArmor)";
  }
  return "Native / C++";
}

export function getInstallTypeLabel(app: AppEntry): string {
  if (app.install_type === "InPlace") {
    return "Tại chỗ (In-Place / Giữ nguyên thư mục gốc)";
  }
  return "Đã chuyển (Moved / Lưu tại thư mục quản lý tập trung)";
}

export function getSystemLinkageStatus(app: AppEntry): {
  ok: boolean;
  statusText: string;
  isWarning: boolean;
} {
  const hasExec = app.exec_path && app.exec_path.trim().length > 0;
  const hasDesktop = app.desktop_file && app.desktop_file.trim().length > 0;

  if (hasExec && hasDesktop) {
    return {
      ok: true,
      statusText: "[OK] Xanh lá - launcher và đường dẫn command line liên kết tốt.",
      isWarning: false,
    };
  }

  if (hasExec && !hasDesktop) {
    return {
      ok: false,
      statusText: "[WARNING] Vàng - Thiếu file launcher (.desktop).",
      isWarning: true,
    };
  }

  return {
    ok: false,
    statusText: "[CRITICAL] Đỏ - Thiếu file chạy gốc hoặc cấu hình.",
    isWarning: false,
  };
}
