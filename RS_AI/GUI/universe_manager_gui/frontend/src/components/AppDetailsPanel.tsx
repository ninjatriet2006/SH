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
} from "lucide-react";
import type { AppEntry } from "../utils/contract";
import {
  getAppPaths,
  detectFramework,
  getInstallTypeLabel,
  getSystemLinkageStatus,
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
            <span className="meta-badge-autostart disabled">
              <Power size={12} /> ĐÃ TẮT (DISABLED)
            </span>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">Kiểu cài đặt:</span>
            <span className="meta-value">{installType}</span>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">Công nghệ/Framework:</span>
            <span className="meta-badge-framework">
              <Cpu size={12} /> {framework}
            </span>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">File chạy chính:</span>
            <code className="meta-code break-all" title={app.exec_path}>
              {app.exec_path || "—"}
            </code>
          </div>

          <div className="detail-meta-row">
            <span className="meta-label">Thư mục lưu:</span>
            <code className="meta-code break-all" title={app.install_path}>
              {app.install_path || "—"}
            </code>
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

        {/* System Linkage Integrity */}
        <div className="details-section">
          <h4 className="section-title">
            <Folder size={14} /> TÌNH TRẠNG LIÊN KẾT HỆ THỐNG:
          </h4>
          <div className={`linkage-banner ${linkage.ok ? "link-ok" : linkage.isWarning ? "link-warn" : "link-bad"}`}>
            {linkage.ok ? (
              <CheckCircle2 size={15} />
            ) : (
              <AlertTriangle size={15} />
            )}
            <span>{linkage.statusText}</span>
          </div>
        </div>

        {app.desktop_file && (
          <div className="details-footer-path">
            <FolderOpen size={13} />
            <span>Launcher: {app.desktop_file}</span>
          </div>
        )}
      </div>
    </aside>
  );
};
