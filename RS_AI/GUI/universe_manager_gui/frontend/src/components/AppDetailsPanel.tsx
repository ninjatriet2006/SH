import { useState, useEffect } from "react";
import {
  Play,
  Square,
  RotateCw,
  Folder,
  Layers,
  Terminal,
  Cpu,
  Power,
  HardDrive,
  CheckCircle2,
  AlertTriangle,
  FolderOpen,
  Pencil,
  Trash2,
  Plus,
  ArrowRightLeft,
  X,
} from "lucide-react";
import type { AppEntry, ExecutableItem } from "../utils/contract";
import { useAppStore } from "../store/useAppStore";
import {
  getAppPaths,
  detectFramework,
  getInstallTypeLabel,
  getSystemLinkageStatus,
  checkRelocatability,
} from "../utils/appInspector";

interface AppDetailsPanelProps {
  app: AppEntry | null;
  disabled?: boolean;
  onStart?: (appId: string) => void;
  onStop?: (appId: string) => void;
  onRestart?: (appId: string) => void;
}

export function AppDetailsPanel({
  app,
  disabled = false,
  onStart,
  onStop,
  onRestart,
}: AppDetailsPanelProps) {
  const updateLauncher = useAppStore((s) => s.updateLauncher);
  const deleteLauncher = useAppStore((s) => s.deleteLauncher);
  const listExecutables = useAppStore((s) => s.listExecutables);
  const relocateApp = useAppStore((s) => s.relocateApp);
  const managedDir = useAppStore((s) => s.config?.settings.managed_dir);

  const [launcherModalOpen, setLauncherModalOpen] = useState(false);
  const [relocateModalOpen, setRelocateModalOpen] = useState(false);

  // Launcher modal form state
  const [candidateExecs, setCandidateExecs] = useState<ExecutableItem[]>([]);
  const [loadingExecs, setLoadingExecs] = useState(false);
  const [launcherName, setLauncherName] = useState("");
  const [selectedExec, setSelectedExec] = useState("");
  const [launcherIcon, setLauncherIcon] = useState("");
  const [launcherArgs, setLauncherArgs] = useState("%U");
  const [launcherTerminal, setLauncherTerminal] = useState(false);
  const [launcherCategories, setLauncherCategories] = useState("Utility;Application;");
  const [launcherWmClass, setLauncherWmClass] = useState("");
  const [isSavingLauncher, setIsSavingLauncher] = useState(false);
  const [isRelocating, setIsRelocating] = useState(false);

  useEffect(() => {
    if (app) {
      setLauncherName(app.name);
      setSelectedExec(app.exec_path);
      setLauncherIcon(app.icon_path || "");
      setLauncherCategories(app.category ? `${app.category};Application;` : "Utility;Application;");
      const isAntigravityIde = app.exec_path.includes("antigravity-ide") || app.name.toLowerCase().includes("antigravity ide");
      setLauncherWmClass(isAntigravityIde ? "Antigravity IDE" : (app.exec_path.split("/").pop() || app.name));
    }
  }, [app]);

  useEffect(() => {
    if (launcherModalOpen && app?.install_path) {
      setLoadingExecs(true);
      listExecutables(app.install_path)
        .then((execs) => {
          setCandidateExecs(execs);
        })
        .finally(() => setLoadingExecs(false));
    }
  }, [launcherModalOpen, app?.install_path, listExecutables]);

  if (!app) {
    return (
      <aside className="glass-panel app-details-panel empty">
        <Layers size={36} className="empty-icon" />
        <p>Chọn một ứng dụng từ danh sách bên trái để xem đầy đủ thông tin chi tiết.</p>
      </aside>
    );
  }

  const isRunning = app.status === "Running";
  const paths = getAppPaths(app);
  const framework = detectFramework(app);
  const installType = getInstallTypeLabel(app);
  const linkage = getSystemLinkageStatus(app);
  const relocation = checkRelocatability(app, managedDir);

  const isCli = app.package_type === "CLI" || app.id.startsWith("cli-");
  const rawCommand = app.start_cmd
    ? app.start_cmd.replace(/\s+--help$/i, "").trim()
    : app.symlink_file
    ? app.symlink_file.split("/").pop()
    : isCli
    ? app.name.replace(/\s*\(CLI\)$/i, "").trim()
    : null;
  const commandName = rawCommand?.trim();
  const symlinkPath = app.symlink_file || (isCli ? app.exec_path : null);
  const targetPath = app.source_path || (symlinkPath && symlinkPath !== app.exec_path ? app.exec_path : null);
  const isSymlinkToDifferent = Boolean(
    targetPath && targetPath !== symlinkPath
  );
  const hasCli = isCli || Boolean(app.symlink_file) || Boolean(app.start_cmd);

  const handleSaveLauncher = async () => {
    if (!selectedExec) return;
    setIsSavingLauncher(true);
    await updateLauncher({
      app_id: app.id,
      exec_path: selectedExec,
      name: launcherName.trim() || app.name,
      icon_path: launcherIcon.trim() || null,
      terminal: launcherTerminal,
      categories: launcherCategories.trim() || null,
      arguments: launcherArgs.trim() || null,
      startup_wm_class: launcherWmClass.trim() || null,
    });
    setIsSavingLauncher(false);
    setLauncherModalOpen(false);
  };

  const handleDeleteLauncher = async () => {
    if (!app.desktop_file) return;
    if (window.confirm(`Bạn có chắc chắn muốn xoá file desktop launcher này không?\n${app.desktop_file}`)) {
      await deleteLauncher(app.id, app.desktop_file);
    }
  };

  const handleConfirmRelocate = async () => {
    setIsRelocating(true);
    await relocateApp(app.id, managedDir || undefined);
    setIsRelocating(false);
    setRelocateModalOpen(false);
  };

  return (
    <aside className="glass-panel app-details-panel">
      <div className="details-header">
        <div className="details-title-row">
          <div className="app-avatar-box">
            {app.icon_path ? (
              <img
                src={app.icon_path.startsWith("/") ? `tauri://localhost/${app.icon_path}` : app.icon_path}
                alt={app.name}
                className="app-icon-img"
                onError={(e) => {
                  (e.target as HTMLImageElement).style.display = "none";
                }}
              />
            ) : (
              <Terminal size={20} />
            )}
          </div>
          <div>
            <h3 className="details-app-name">{app.name}</h3>
            <span className="details-app-category">{app.category || "Tiện ích (Utility)"}</span>
          </div>
        </div>

        <div className="details-quick-actions">
          {!isRunning ? (
            <button
              type="button"
              className="btn btn-sm btn-primary"
              disabled={disabled}
              onClick={() => onStart?.(app.id)}
            >
              <Play size={13} /> Khởi động
            </button>
          ) : (
            <>
              <button
                type="button"
                className="btn btn-sm btn-danger"
                disabled={disabled}
                onClick={() => onStop?.(app.id)}
              >
                <Square size={13} /> Dừng
              </button>
              {onRestart && (
                <button
                  type="button"
                  className="btn btn-sm btn-ghost"
                  disabled={disabled}
                  onClick={() => onRestart?.(app.id)}
                  title="Khởi động lại tiến trình"
                >
                  <RotateCw size={13} />
                </button>
              )}
            </>
          )}
        </div>
      </div>

      <div className="details-content-scroll">
        {/* Core Metadata */}
        <div className="details-section">
          <div className="detail-meta-row">
            <span className="meta-label">Tên ứng dụng:</span>
            <span className="meta-value font-semibold">{app.name}</span>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">App ID:</span>
            <code className="meta-code">{app.id}</code>
          </div>

          {app.version && (
            <div className="detail-meta-row">
              <span className="meta-label">Phiên bản:</span>
              <span className="meta-value">{app.version}</span>
            </div>
          )}

          {app.publisher && (
            <div className="detail-meta-row">
              <span className="meta-label">Nhà phát triển:</span>
              <span className="meta-value">{app.publisher}</span>
            </div>
          )}

          <div className="detail-meta-row">
            <span className="meta-label">Trạng thái chạy:</span>
            <span className={`status-badge-compact ${isRunning ? "running" : "stopped"}`}>
              <span className="dot" />
              {isRunning ? "ĐANG CHẠY (RUNNING)" : "ĐÃ DỪNG (STOPPED)"}
            </span>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">Khởi động hệ thống:</span>
            <span className="meta-badge-autostart">
              <Power size={12} /> ĐÃ TẮT (DISABLED)
            </span>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">Kiểu cài đặt:</span>
            <span className="meta-value">{installType}</span>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">Công nghệ/ Framework:</span>
            <span className="meta-badge-framework">
              <Cpu size={12} /> {framework}
            </span>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">File chạy chính:</span>
            <code className="meta-code" title={app.exec_path}>{app.exec_path}</code>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">Thư mục lưu:</span>
            <code className="meta-code" title={app.install_path}>{app.install_path}</code>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">Nguồn nhận diện:</span>
            <span className="meta-value font-bold uppercase">
              {app.inventory_sources?.join(", ") || (isCli ? "PATH" : "LOCAL")}
            </span>
          </div>

          <div className="detail-meta-row" style={{ marginTop: "0.25rem" }}>
            <span className="meta-label">Lệnh thực thi (CLI):</span>
            {hasCli && commandName ? (
              <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem", width: "100%" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
                  <code
                    className="meta-code"
                    style={{
                      background: "rgba(52, 152, 219, 0.15)",
                      color: "var(--accent-hover)",
                      fontWeight: 700,
                      fontSize: "0.85rem",
                    }}
                  >
                    {commandName}
                  </code>
                  <span style={{ fontSize: "0.75rem", opacity: 0.8 }}>
                    (Gõ trực tiếp vào terminal bất kỳ)
                  </span>
                </div>
                {symlinkPath && (
                  <div style={{ fontSize: "0.75rem", opacity: 0.75, display: "flex", alignItems: "center", gap: "0.3rem" }}>
                    <span>Vị trí tệp lệnh:</span>
                    <code style={{ fontSize: "0.72rem" }}>{symlinkPath}</code>
                  </div>
                )}
                {isSymlinkToDifferent && targetPath && (
                  <div style={{ fontSize: "0.75rem", color: "var(--badge-local-text, #2ecc71)", display: "flex", alignItems: "center", gap: "0.3rem" }}>
                    <span>→ Đích liên kết thực tế:</span>
                    <code style={{ fontSize: "0.72rem" }}>{targetPath}</code>
                  </div>
                )}
              </div>
            ) : (
              <span className="meta-value" style={{ opacity: 0.55, fontStyle: "italic", fontSize: "0.82rem" }}>
                Chưa gán lệnh CLI / symlink trong ~/.local/bin
              </span>
            )}
          </div>
        </div>

        <div className="details-divider" />

        {/* Configuration & Data Paths */}
        <div className="details-section">
          <h4 className="section-title">
            <HardDrive size={14} /> THƯ MỤC CẤU HÌNH & DỮ LIỆU:
          </h4>
          <ul className="paths-list">
            <li>
              <span className="path-kind">• Cấu hình (Config):</span>
              <code className="path-text" title={paths.configDir}>{paths.configDir}</code>
            </li>
            <li>
              <span className="path-kind">• Dữ liệu (Data):</span>
              <code className="path-text" title={paths.dataDir}>{paths.dataDir}</code>
            </li>
            <li>
              <span className="path-kind">• Bộ nhớ đệm (Cache):</span>
              <code className="path-text" title={paths.cacheDir}>{paths.cacheDir}</code>
            </li>
            {paths.shareDir && (
              <li>
                <span className="path-kind">• Tài nguyên (Share):</span>
                <code className="path-text" title={paths.shareDir}>{paths.shareDir}</code>
              </li>
            )}
          </ul>
        </div>

        <div className="details-divider" />

        {/* System Linkage Integrity & Launcher Management */}
        <div className="details-section">
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap", gap: "0.4rem" }}>
            <h4 className="section-title" style={{ margin: 0 }}>
              <Folder size={14} /> TÌNH TRẠNG LIÊN KẾT HỆ THỐNG:
            </h4>
            {!isCli && (
              <div className="launcher-btn-group">
                {app.desktop_file ? (
                  <>
                    <button
                      type="button"
                      className="btn btn-xs btn-outline"
                      onClick={() => setLauncherModalOpen(true)}
                      title="Chỉnh sửa hoặc chọn lại file binary cho Launcher Desktop"
                    >
                      <Pencil size={11} /> Chỉnh sửa Launcher
                    </button>
                    <button
                      type="button"
                      className="btn btn-xs btn-danger-outline"
                      onClick={handleDeleteLauncher}
                      title="Xoá file Desktop Launcher"
                    >
                      <Trash2 size={11} /> Xoá
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    className="btn btn-xs btn-outline-primary"
                    onClick={() => setLauncherModalOpen(true)}
                    title="Tạo file desktop launcher cho ứng dụng này"
                  >
                    <Plus size={11} /> Tạo Launcher (.desktop)
                  </button>
                )}
              </div>
            )}
          </div>

          <div className={`linkage-banner ${linkage.ok ? "link-ok" : linkage.isWarning ? "link-warn" : "link-bad"}`}>
            {linkage.ok ? <CheckCircle2 size={15} /> : <AlertTriangle size={15} />}
            <span>{linkage.statusText}</span>
          </div>

          {app.desktop_file && (
            <div className="details-footer-path" style={{ marginTop: "0.2rem" }}>
              <FolderOpen size={13} />
              <span>Launcher: {app.desktop_file}</span>
            </div>
          )}
          {app.symlink_file && (
            <div className="details-footer-path" style={{ marginTop: "0.25rem" }}>
              <Terminal size={13} />
              <span>CLI / PATH: {app.symlink_file}</span>
            </div>
          )}
        </div>

        <div className="details-divider" />

        {/* Relocation Capability Analysis & Action */}
        <div className="details-section">
          <h4
            className="section-title"
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              flexWrap: "wrap",
              gap: "0.4rem",
            }}
          >
            <span style={{ display: "flex", alignItems: "center", gap: "0.4rem" }}>
              <Layers size={14} /> KHẢ NĂNG DI CHUYỂN (RELOCATION):
            </span>
            <span
              style={{
                fontSize: "0.7rem",
                padding: "0.15rem 0.5rem",
                borderRadius: "4px",
                fontWeight: 600,
                background: relocation.supported
                  ? "rgba(46, 204, 113, 0.2)"
                  : relocation.status === "AlreadyManaged"
                  ? "rgba(52, 152, 219, 0.2)"
                  : "rgba(231, 76, 60, 0.2)",
                color: relocation.supported
                  ? "var(--badge-local-text, #2ecc71)"
                  : relocation.status === "AlreadyManaged"
                  ? "var(--accent, #3498db)"
                  : "var(--danger, #e74c3c)",
              }}
            >
              {relocation.badgeText}
            </span>
          </h4>
          <p
            style={{
              margin: "0.35rem 0 0",
              fontSize: "0.82rem",
              lineHeight: "1.4",
              opacity: 0.85,
            }}
          >
            {relocation.reason}
          </p>

          {relocation.supported && (
            <div className="relocate-action-box">
              <div className="relocate-dest-hint">
                <span style={{ fontWeight: 600 }}>Thư mục chuyển đến:</span>
                <code>{(managedDir || "~/Applications")}/{app.install_path.split("/").filter(Boolean).pop()}</code>
              </div>
              <button
                type="button"
                className="btn btn-sm btn-primary"
                style={{ width: "fit-content", display: "flex", alignItems: "center", gap: "0.4rem", marginTop: "0.2rem" }}
                onClick={() => setRelocateModalOpen(true)}
              >
                <ArrowRightLeft size={13} /> Chuyển vào ~/Applications
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Modal: Setup / Edit Desktop Launcher */}
      {launcherModalOpen && (
        <div className="modal-overlay" onClick={() => !isSavingLauncher && setLauncherModalOpen(false)}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3 className="modal-title">
                <FolderOpen size={16} /> {app.desktop_file ? "Chỉnh sửa Launcher (.desktop)" : "Tạo Launcher mới (.desktop)"}
              </h3>
              <button
                type="button"
                className="modal-close-btn"
                onClick={() => setLauncherModalOpen(false)}
                disabled={isSavingLauncher}
              >
                <X size={16} />
              </button>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label className="form-label">Tên hiển thị (Display Name):</label>
                <input
                  type="text"
                  className="form-input"
                  value={launcherName}
                  onChange={(e) => setLauncherName(e.target.value)}
                  placeholder="Tên ứng dụng"
                />
              </div>

              <div className="form-group">
                <label className="form-label">Tệp thực thi khởi chạy chính (Binary Executable):</label>
                {loadingExecs ? (
                  <span style={{ fontSize: "0.8rem", opacity: 0.7 }}>Đang quét các tệp thực thi trong thư mục...</span>
                ) : candidateExecs.length > 0 ? (
                  <select
                    className="form-select"
                    value={selectedExec}
                    onChange={(e) => setSelectedExec(e.target.value)}
                  >
                    {candidateExecs.map((exec) => (
                      <option key={exec.path} value={exec.path}>
                        {exec.name} ({(exec.size_bytes / 1024 / 1024).toFixed(1)} MB) — {exec.path}
                      </option>
                    ))}
                  </select>
                ) : (
                  <input
                    type="text"
                    className="form-input"
                    value={selectedExec}
                    onChange={(e) => setSelectedExec(e.target.value)}
                    placeholder="/đường/dẫn/tới/binary"
                  />
                )}
                <span className="form-hint">
                  Chọn đúng binary khởi động chính (tránh các tệp phụ trợ như QtWebEngineProcess).
                </span>
              </div>

              <div className="form-group">
                <label className="form-label">Đường dẫn biểu tượng (Icon):</label>
                <input
                  type="text"
                  className="form-input"
                  value={launcherIcon}
                  onChange={(e) => setLauncherIcon(e.target.value)}
                  placeholder="Đường dẫn icon (.png, .svg)"
                />
              </div>

              <div className="form-group">
                <label className="form-label">Tham số dòng lệnh (Arguments):</label>
                <input
                  type="text"
                  className="form-input"
                  value={launcherArgs}
                  onChange={(e) => setLauncherArgs(e.target.value)}
                  placeholder="%U"
                />
              </div>

              <div className="form-group">
                <label className="form-label">Danh mục (Categories):</label>
                <input
                  type="text"
                  className="form-input"
                  value={launcherCategories}
                  onChange={(e) => setLauncherCategories(e.target.value)}
                  placeholder="Utility;Application;"
                />
              </div>

              <div className="form-group">
                <label className="form-label">Định danh cửa sổ (StartupWMClass):</label>
                <input
                  type="text"
                  className="form-input"
                  value={launcherWmClass}
                  onChange={(e) => setLauncherWmClass(e.target.value)}
                  placeholder="StartupWMClass (vd: Antigravity IDE)"
                />
                <span className="form-hint">
                  Dùng để hệ điều hành liên kết cửa sổ đang chạy với biểu tượng trên thanh tác vụ (Taskbar / Dock).
                </span>
              </div>

              <div className="form-group">
                <label className="form-checkbox-label">
                  <input
                    type="checkbox"
                    checked={launcherTerminal}
                    onChange={(e) => setLauncherTerminal(e.target.checked)}
                  />
                  <span>Chạy trong cửa sổ terminal (Terminal = true)</span>
                </label>
              </div>
            </div>
            <div className="modal-footer">
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => setLauncherModalOpen(false)}
                disabled={isSavingLauncher}
              >
                Hủy
              </button>
              <button
                type="button"
                className="btn btn-primary"
                onClick={handleSaveLauncher}
                disabled={isSavingLauncher || !selectedExec}
              >
                {isSavingLauncher ? "Đang lưu..." : "Lưu Launcher"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modal: Relocate Application */}
      {relocateModalOpen && (
        <div className="modal-overlay" onClick={() => !isRelocating && setRelocateModalOpen(false)}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h3 className="modal-title">
                <ArrowRightLeft size={16} /> Di chuyển vào Thư mục Quản lý (~/Applications)
              </h3>
              <button
                type="button"
                className="modal-close-btn"
                onClick={() => setRelocateModalOpen(false)}
                disabled={isRelocating}
              >
                <X size={16} />
              </button>
            </div>
            <div className="modal-body">
              <p style={{ fontSize: "0.85rem", lineHeight: 1.5, margin: 0 }}>
                Ứng dụng <strong>{app.name}</strong> sẽ được di chuyển an toàn vào thư mục quản lý tập trung:
              </p>

              <div className="form-group">
                <span className="form-label">Thư mục nguồn (Hiện tại):</span>
                <code className="path-text">{app.install_path}</code>
              </div>

              <div className="form-group">
                <span className="form-label">Thư mục đích (Sau khi di chuyển):</span>
                <code className="path-text">
                  {(managedDir || "~/Applications")}/{app.install_path.split("/").filter(Boolean).pop()}
                </code>
              </div>

              <div style={{ background: "rgba(52, 152, 219, 0.1)", border: "1px solid rgba(52, 152, 219, 0.25)", borderRadius: "4px", padding: "0.75rem", fontSize: "0.8rem", color: "var(--foreground)" }}>
                <strong>Tự động xử lý liên kết:</strong>
                <ul style={{ margin: "0.35rem 0 0", paddingLeft: "1.2rem", display: "flex", flexDirection: "column", gap: "0.2rem" }}>
                  <li>Di chuyển toàn bộ thư mục và tệp nhị phân sang vị trí mới.</li>
                  <li>Tự động cập nhật đường dẫn Exec/Icon trong launcher <code>.desktop</code>.</li>
                  <li>Tự động cập nhật liên kết biểu tượng symlink trong <code>~/.local/bin</code> (nếu có).</li>
                  <li>Cập nhật cơ sở dữ liệu menu hệ thống (Desktop database).</li>
                </ul>
              </div>
            </div>
            <div className="modal-footer">
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => setRelocateModalOpen(false)}
                disabled={isRelocating}
              >
                Hủy
              </button>
              <button
                type="button"
                className="btn btn-primary"
                onClick={handleConfirmRelocate}
                disabled={isRelocating}
              >
                {isRelocating ? "Đang di chuyển..." : "⚡ Xác nhận Di chuyển"}
              </button>
            </div>
          </div>
        </div>
      )}
    </aside>
  );
}
