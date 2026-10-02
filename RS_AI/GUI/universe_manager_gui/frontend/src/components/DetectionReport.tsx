import { useState, useEffect } from "react";
import { FolderPlus, Layers, Terminal, Sparkles, CheckCircle2 } from "lucide-react";
import type { AppEntry, DetectionReport as DetectionReportType, AppIntegrateRequest } from "../utils/contract";
import { useAppStore } from "../store/useAppStore";

interface DetectionReportProps {
  report: DetectionReportType;
  onComplete?: (entry: AppEntry) => void;
}

export function DetectionReport({ report, onComplete }: DetectionReportProps) {
  const integrateApp = useAppStore((s) => s.integrateApp);
  const clearDetection = useAppStore((s) => s.clearDetection);
  const busy = useAppStore((s) => s.busy);

  const [name, setName] = useState(report.suggested_name);
  const [selectedExec, setSelectedExec] = useState(report.executables[0]?.path ?? "");
  const [selectedIcon, setSelectedIcon] = useState(report.icons[0]?.path ?? "");

  // Options
  const [shouldRelocate, setShouldRelocate] = useState(true);
  const [createDesktop, setCreateDesktop] = useState(true);
  const [desktopCategories, setDesktopCategories] = useState("Utility;Application;");
  const [desktopArgs, setDesktopArgs] = useState("%U");
  const [startupWmClass, setStartupWmClass] = useState("");

  const [createSymlink, setCreateSymlink] = useState(false);
  const [symlinkName, setSymlinkName] = useState("");

  // Initialize defaults from detection
  useEffect(() => {
    if (report) {
      setName(report.suggested_name);
      const defaultExec = report.executables[0]?.path ?? "";
      setSelectedExec(defaultExec);
      setSelectedIcon(report.icons[0]?.path ?? "");

      const execStem = defaultExec ? (defaultExec.split("/").pop() ?? "") : report.suggested_name;
      const cleanStem = execStem.replace(/\.[^/.]+$/, "").toLowerCase();
      setSymlinkName(cleanStem);

      const isIde = report.suggested_name.toLowerCase().includes("ide") || cleanStem.includes("ide");
      setStartupWmClass(isIde ? "Antigravity IDE" : report.suggested_name);
    }
  }, [report]);

  const detectedSourcePath = useAppStore((s) => s.detectedSourcePath);

  const handleIntegrate = async () => {
    if (!selectedExec) return;
    const cleanName = name.trim() || report.suggested_name;
    const parentDir = selectedExec.includes("/")
      ? selectedExec.substring(0, selectedExec.lastIndexOf("/"))
      : "";
    const sourcePath = detectedSourcePath || parentDir;

    const req: AppIntegrateRequest = {
      name: cleanName,
      source_path: sourcePath,
      exec_path: selectedExec,
      icon_path: selectedIcon || null,
      should_relocate: shouldRelocate,
      create_desktop: createDesktop,
      desktop_categories: desktopCategories.trim() || null,
      desktop_arguments: desktopArgs.trim() || null,
      startup_wm_class: startupWmClass.trim() || null,
      create_symlink: createSymlink,
      symlink_name: symlinkName.trim() || null,
      package_type: report.is_appimage ? "AppImage" : "Local",
    };

    const result = await integrateApp(req);
    if (result && onComplete) {
      onComplete(result);
    }
  };

  return (
    <article className="glass-panel report" style={{ display: "flex", flexDirection: "column", gap: "1.1rem" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <h3 style={{ margin: 0, display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <Sparkles size={18} style={{ color: "var(--accent)" }} />
            <span>Phân tích & Tích hợp: <strong>{report.suggested_name}</strong></span>
            {report.is_appimage && (
              <span className="source-badge appimage" style={{ fontSize: "0.75rem" }}>
                AppImage
              </span>
            )}
          </h3>
          <p style={{ margin: "0.25rem 0 0", fontSize: "0.85rem", color: "var(--panel-foreground)" }}>
            Đã phát hiện {report.executables.length} tệp thực thi và {report.icons.length} biểu tượng. Thiết lập các tùy chọn để tích hợp hoàn chỉnh vào máy.
          </p>
        </div>
        <button className="btn btn-ghost btn-sm" onClick={clearDetection} title="Đóng">
          ✕
        </button>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr", gap: "0.9rem", background: "var(--input-bg)", border: "1px solid var(--border)", padding: "1.1rem", borderRadius: "10px" }}>
        {/* App Name */}
        <div style={{ display: "flex", flexDirection: "column", gap: "0.3rem" }}>
          <label className="form-label">Tên hiển thị ứng dụng (Display Name):</label>
          <input
            type="text"
            className="form-input"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Tên ứng dụng"
          />
        </div>

        {/* Executable Selection */}
        <div style={{ display: "flex", flexDirection: "column", gap: "0.3rem" }}>
          <label className="form-label">Tệp thực thi chính (Executable Binary):</label>
          {report.executables.length > 1 ? (
            <select
              className="form-select"
              value={selectedExec}
              onChange={(e) => setSelectedExec(e.target.value)}
            >
              {report.executables.map((e) => (
                <option key={e.path} value={e.path}>
                  {e.path.split("/").pop()} — {e.path}
                </option>
              ))}
            </select>
          ) : (
            <code style={{ fontSize: "0.82rem", wordBreak: "break-all", padding: "0.5rem 0.75rem", background: "var(--input-bg)", border: "1px solid var(--border)", borderRadius: "6px", color: "var(--foreground)" }}>
              {selectedExec || "Không tìm thấy tệp thực thi"}
            </code>
          )}
          <span className="form-hint">
            Hệ thống sẽ cấp quyền thực thi (chmod +x) và tạo các liên kết hệ thống tới tệp này.
          </span>
        </div>

        {/* Icon Selection */}
        <div style={{ display: "flex", flexDirection: "column", gap: "0.3rem" }}>
          <label className="form-label">Biểu tượng (Icon):</label>
          {report.icons.length > 1 ? (
            <select
              className="form-select"
              value={selectedIcon}
              onChange={(e) => setSelectedIcon(e.target.value)}
            >
              <option value="">(Không dùng biểu tượng / Dùng icon mặc định)</option>
              {report.icons.map((i) => (
                <option key={i.path} value={i.path}>
                  {i.path.split("/").pop()} — {i.path}
                </option>
              ))}
            </select>
          ) : (
            <span style={{ fontSize: "0.85rem" }}>
              {selectedIcon ? (
                <code style={{ fontSize: "0.8rem", wordBreak: "break-all", padding: "0.35rem 0.6rem", background: "var(--input-bg)", border: "1px solid var(--border)", borderRadius: "4px", color: "var(--foreground)" }}>{selectedIcon}</code>
              ) : (
                <span className="form-hint">Chưa tìm thấy biểu tượng trong gói (sẽ sử dụng icon hệ thống)</span>
              )}
            </span>
          )}
        </div>

        {/* Option 1: Storage / Relocation */}
        <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem", paddingTop: "0.75rem", borderTop: "1px solid var(--border)" }}>
          <label className="form-label" style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
            <Layers size={14} /> Chế độ lưu trữ & Quản lý vị trí:
          </label>
          <div style={{ display: "flex", gap: "1.25rem", flexWrap: "wrap" }}>
            <label style={{ display: "flex", alignItems: "center", gap: "0.45rem", cursor: "pointer", fontSize: "0.86rem", color: "var(--foreground)" }}>
              <input
                type="radio"
                name="relocateOpt"
                checked={shouldRelocate}
                onChange={() => setShouldRelocate(true)}
              />
              <span>
                <strong>Quản lý tập trung (~/Applications)</strong> (Khuyến nghị cho file tải về/giải nén trong Downloads/Documents)
              </span>
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: "0.45rem", cursor: "pointer", fontSize: "0.86rem", color: "var(--foreground)" }}>
              <input
                type="radio"
                name="relocateOpt"
                checked={!shouldRelocate}
                onChange={() => setShouldRelocate(false)}
              />
              <span>Giữ nguyên tại chỗ (In-Place / Giữ nguyên thư mục gốc hiện tại)</span>
            </label>
          </div>
        </div>

        {/* Option 2: Desktop Launcher */}
        <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", paddingTop: "0.75rem", borderTop: "1px solid var(--border)" }}>
          <label className="form-checkbox-label">
            <input
              type="checkbox"
              checked={createDesktop}
              onChange={(e) => setCreateDesktop(e.target.checked)}
            />
            <span style={{ fontWeight: 600 }}>
              <FolderPlus size={14} style={{ display: "inline", verticalAlign: "middle", marginRight: "4px" }} />
              Tạo Launcher Desktop (.desktop)
            </span>
          </label>
          <span className="form-hint">
            Tự động xuất hiện trong Menu ứng dụng hệ thống (App Launcher), thanh tìm kiếm Super/Windows và thanh tác vụ (Taskbar / Dock).
          </span>

          {createDesktop && (
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.6rem", marginTop: "0.2rem", paddingLeft: "1.5rem" }}>
              <div className="form-group">
                <label className="form-label">Danh mục (Categories):</label>
                <input
                  type="text"
                  className="form-input"
                  value={desktopCategories}
                  onChange={(e) => setDesktopCategories(e.target.value)}
                  placeholder="Utility;Application;"
                />
              </div>
              <div className="form-group">
                <label className="form-label">Tham số (Arguments):</label>
                <input
                  type="text"
                  className="form-input"
                  value={desktopArgs}
                  onChange={(e) => setDesktopArgs(e.target.value)}
                  placeholder="%U"
                />
              </div>
              <div className="form-group" style={{ gridColumn: "1 / -1" }}>
                <label className="form-label">Định danh cửa sổ (StartupWMClass):</label>
                <input
                  type="text"
                  className="form-input"
                  value={startupWmClass}
                  onChange={(e) => setStartupWmClass(e.target.value)}
                  placeholder="vd: Antigravity IDE"
                />
                <span className="form-hint">
                  Giúp hệ điều hành nhóm cửa sổ đang chạy với biểu tượng trên thanh tác vụ (Taskbar/Dock).
                </span>
              </div>
            </div>
          )}
        </div>

        {/* Option 3: Command Line / PATH */}
        <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", paddingTop: "0.75rem", borderTop: "1px solid var(--border)" }}>
          <label className="form-checkbox-label">
            <input
              type="checkbox"
              checked={createSymlink}
              onChange={(e) => setCreateSymlink(e.target.checked)}
            />
            <span style={{ fontWeight: 600 }}>
              <Terminal size={14} style={{ display: "inline", verticalAlign: "middle", marginRight: "4px" }} />
              Tạo lệnh gọi nhanh trong Terminal (Đưa vào $PATH tại ~/.local/bin)
            </span>
          </label>
          <span className="form-hint">
            Tạo liên kết tượng trưng (symlink) trong ~/.local/bin, cho phép mở terminal ở bất kỳ đâu và gõ lệnh chạy ngay lập tức.
          </span>

          {createSymlink && (
            <div style={{ marginTop: "0.2rem", paddingLeft: "1.5rem" }}>
              <div className="form-group">
                <label className="form-label">Tên lệnh gọi trong Terminal (Command Name):</label>
                <input
                  type="text"
                  className="form-input"
                  value={symlinkName}
                  onChange={(e) => setSymlinkName(e.target.value)}
                  placeholder="vd: grok, claude, my-app"
                />
                <span className="form-hint">
                  Lệnh sẽ được tạo tại: <code>~/.local/bin/{symlinkName || "app"}</code>
                </span>
              </div>
            </div>
          )}
        </div>
      </div>

      <div style={{ display: "flex", gap: "0.75rem", justifyContent: "flex-end", marginTop: "0.3rem" }}>
        <button className="btn btn-ghost" onClick={clearDetection} disabled={busy}>
          ✕ Huỷ bỏ
        </button>
        <button
          className="btn btn-primary"
          onClick={handleIntegrate}
          disabled={busy || !selectedExec}
          style={{ display: "flex", alignItems: "center", gap: "0.55rem", padding: "0.6rem 1.25rem", fontSize: "0.92rem", fontWeight: 600 }}
        >
          <CheckCircle2 size={16} />
          <span>{busy ? "Đang xử lý..." : "⚡ Cài đặt & Tích hợp vào hệ thống"}</span>
        </button>
      </div>
    </article>
  );
}
