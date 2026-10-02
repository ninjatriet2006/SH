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

  const baseId = realId.startsWith("cli-") ? realId.slice(4) : realId;

  if (ptype === "cli") {
    return {
      configDir: `~/.config/${baseId}`,
      dataDir: `~/.local/share/${baseId}`,
      cacheDir: `~/.cache/${baseId}`,
    };
  }

  // APT, Local or System
  return {
    configDir: `~/.config/${baseId}`,
    dataDir: `~/.local/share/${baseId}`,
    cacheDir: `~/.cache/${baseId}`,
    shareDir: `/usr/share/${baseId}`,
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
  const isCli = app.package_type === "CLI" || app.id.startsWith("cli-");
  const hasExec = app.exec_path && app.exec_path.trim().length > 0;
  const hasDesktop = app.desktop_file && app.desktop_file.trim().length > 0;

  if (isCli) {
    if (hasExec) {
      const cmd = app.start_cmd || app.name.replace(/\s*\(CLI\)$/i, "");
      return {
        ok: true,
        statusText: `[OK] Xanh lá - Lệnh CLI sẵn sàng trong $PATH (Gõ '${cmd}' trong terminal).`,
        isWarning: false,
      };
    }
    return {
      ok: false,
      statusText: "[CRITICAL] Đỏ - Tệp binary lệnh trong $PATH không tồn tại.",
      isWarning: false,
    };
  }

  if (hasExec && hasDesktop) {
    return {
      ok: true,
      statusText: "[OK] Xanh lá - Cả launcher máy tính và đường dẫn lệnh liên kết tốt.",
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

export interface RelocationAnalysis {
  supported: boolean;
  status: "AlreadyManaged" | "Supported" | "Unsupported";
  badgeText: string;
  reason: string;
  isMonolithicOrHardcoded?: boolean;
}

export function checkRelocatability(app: AppEntry, managedDir?: string): RelocationAnalysis {
  const ptype = (app.package_type ?? "").toLowerCase();
  const installPath = app.install_path || "";
  const execPath = app.exec_path || "";
  const appName = (app.name || "").toLowerCase();
  const appId = (app.id || "").toLowerCase();
  const cleanManaged = (managedDir || "").trim().toLowerCase();

  // 1. Check if already inside managed directory (~/Applications)
  if (cleanManaged && installPath.toLowerCase().startsWith(cleanManaged)) {
    return {
      supported: false,
      status: "AlreadyManaged",
      badgeText: "ĐÃ Ở ~/APPLICATIONS",
      reason: "Ứng dụng này đã được lưu trữ trong thư mục quản lý tập trung.",
    };
  }

  // 2. System and containerized packages
  if (["flatpak", "snap", "apt", "system"].includes(ptype)) {
    return {
      supported: false,
      status: "Unsupported",
      badgeText: "UNSUPPORTED",
      reason: `Gói phần mềm thuộc hệ thống (${app.package_type || "System"}) được quản lý bởi trình quản lý gói hệ điều hành, không hỗ trợ di chuyển thư mục.`,
    };
  }

  // 3. System CLI tools (e.g. in /usr/bin, /usr/local/bin)
  if (installPath.startsWith("/usr/") || installPath.startsWith("/bin") || installPath.startsWith("/sbin")) {
    return {
      supported: false,
      status: "Unsupported",
      badgeText: "UNSUPPORTED",
      reason: "Tệp nhị phân thuộc phân vùng hệ thống root (/usr/bin), không thể di chuyển.",
    };
  }

  // 4. Heavy monolithic enterprise suites with hardcoded path dependencies (like MATLAB)
  if (
    appId.includes("matlab") ||
    appName.includes("matlab") ||
    installPath.toLowerCase().includes("matlab") ||
    execPath.toLowerCase().includes("matlab") ||
    appName.includes("simulink")
  ) {
    return {
      supported: false,
      status: "Unsupported",
      badgeText: "UNSUPPORTED (HARDCODED PATHS)",
      isMonolithicOrHardcoded: true,
      reason:
        "Phần mềm dạng Monolithic/Enterprise (như MATLAB) có cấu hình phụ thuộc đường dẫn tuyệt đối nội bộ (licenses, ServiceHost, glnxa64, .matlab7rc.sh). Việc di chuyển thư mục có nguy cơ làm hỏng bản quyền và runtime. Khuyến nghị duy trì In-Place.",
    };
  }

  // 5. Portable applications (AppImage or standalone user directory)
  if (ptype === "appimage" || ptype === "local" || ptype === "portable" || ptype === "cli") {
    if (ptype === "cli") {
      return {
        supported: false,
        status: "Unsupported",
        badgeText: "UNSUPPORTED (CLI BIN)",
        reason: "Lệnh dòng lệnh được quản lý trực tiếp qua biến môi trường $PATH, không cần di chuyển thư mục.",
      };
    }

    return {
      supported: true,
      status: "Supported",
      badgeText: "HỖ TRỢ RELOCATE",
      reason: "Ứng dụng Portable độc lập, có thể di chuyển an toàn vào thư mục quản lý tập trung (~/Applications).",
    };
  }

  return {
    supported: false,
    status: "Unsupported",
    badgeText: "UNSUPPORTED",
    reason: "Định dạng ứng dụng không hỗ trợ di chuyển tự động.",
  };
}
