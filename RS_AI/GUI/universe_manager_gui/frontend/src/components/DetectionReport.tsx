import { useState } from "react";
import type { AppEntry, DetectionReport as DetectionReportType } from "../utils/contract";
import { useAppStore } from "../store/useAppStore";

interface DetectionReportProps {
  report: DetectionReportType;
}

export function DetectionReport({ report }: DetectionReportProps) {
  const integrateApp = useAppStore(s => s.integrateApp);
  const clearDetection = useAppStore(s => s.clearDetection);
  const busy = useAppStore(s => s.busy);

  const [name, setName] = useState(report.suggested_name);
  const [selectedExec, setSelectedExec] = useState(report.executables[0]?.path ?? "");
  const [selectedIcon, setSelectedIcon] = useState(report.icons[0]?.path ?? "");
  const [installType, setInstallType] = useState<"InPlace" | "Moved">("InPlace");

  const handleIntegrate = async () => {
    if (!selectedExec) return;
    const cleanName = name.trim() || report.suggested_name;
    const slug = cleanName
      .toLowerCase()
      .replace(/[^a-z0-9_-]/g, "-")
      .replace(/-+/g, "-")
      .replace(/^-|-$/g, "");
    const id = slug || `app-${Date.now()}`;
    const parentDir = selectedExec.includes("/")
      ? selectedExec.substring(0, selectedExec.lastIndexOf("/"))
      : "";

    const entry: AppEntry = {
      id,
      name: cleanName,
      install_type: installType,
      source_path: installType === "Moved" ? parentDir : null,
      install_path: parentDir,
      exec_path: selectedExec,
      icon_path: selectedIcon ? selectedIcon : null,
      desktop_file: report.desktop_templates[0]?.path ?? "",
      symlink_file: null,
      added_at: new Date().toISOString(),
      is_custom: true,
      start_cmd: null,
      stop_cmd: null,
      category: "Utility",
      package_type: report.is_appimage ? "AppImage" : "Local",
      inventory_sources: [report.is_appimage ? "AppImage" : "Portable"],
      registry_key: null,
      product_code: null,
      about_url: null,
      publisher: null,
      version: null,
      uninstall_cmd: null,
      status: "Ok",
    };

    await integrateApp(entry);
  };

  return (
    <article className="glass-panel report" style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <h3 style={{ margin: 0, display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <span>📦</span> {report.suggested_name}
            {report.is_appimage && (
              <span className="source-badge appimage" style={{ fontSize: "0.75rem" }}>
                AppImage
              </span>
            )}
          </h3>
          <p style={{ margin: "0.25rem 0 0", fontSize: "0.85rem", opacity: 0.8 }}>
            Đã phân tích gói ứng dụng thành công. Xác nhận thông tin để tích hợp vào hệ thống.
          </p>
        </div>
        <button className="btn btn-ghost btn-sm" onClick={clearDetection} title="Đóng">
          ✕
        </button>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr", gap: "0.75rem", background: "var(--input-bg)", border: "1px solid var(--border)", padding: "1rem", borderRadius: "8px" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
          <label style={{ fontSize: "0.85rem", fontWeight: 600 }}>Tên hiển thị ứng dụng:</label>
          <input
            type="text"
            className="input"
            value={name}
            onChange={e => setName(e.target.value)}
            placeholder="Tên ứng dụng"
            style={{ padding: "0.4rem 0.6rem", borderRadius: "6px" }}
          />
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
          <label style={{ fontSize: "0.85rem", fontWeight: 600 }}>Tệp thực thi chính (Executable):</label>
          {report.executables.length > 1 ? (
            <select
              className="input"
              value={selectedExec}
              onChange={e => setSelectedExec(e.target.value)}
              style={{ padding: "0.4rem 0.6rem", borderRadius: "6px" }}
            >
              {report.executables.map(e => (
                <option key={e.path} value={e.path}>
                  {e.path.split("/").pop()} — {e.path}
                </option>
              ))}
            </select>
          ) : (
            <code style={{ fontSize: "0.8rem", wordBreak: "break-all", padding: "0.45rem 0.65rem", background: "var(--input-bg)", border: "1px solid var(--border)", borderRadius: "6px", color: "var(--foreground)" }}>
              {selectedExec || "Không tìm thấy tệp thực thi"}
            </code>
          )}
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
          <label style={{ fontSize: "0.85rem", fontWeight: 600 }}>Biểu tượng (Icon):</label>
          {report.icons.length > 1 ? (
            <select
              className="input"
              value={selectedIcon}
              onChange={e => setSelectedIcon(e.target.value)}
              style={{ padding: "0.4rem 0.6rem", borderRadius: "6px" }}
            >
              <option value="">(Không dùng biểu tượng)</option>
              {report.icons.map(i => (
                <option key={i.path} value={i.path}>
                  {i.path.split("/").pop()} — {i.path}
                </option>
              ))}
            </select>
          ) : (
            <span style={{ fontSize: "0.85rem" }}>
              {selectedIcon ? (
                <code style={{ fontSize: "0.8rem", wordBreak: "break-all", padding: "0.3rem 0.5rem", background: "var(--input-bg)", border: "1px solid var(--border)", borderRadius: "4px", color: "var(--foreground)" }}>{selectedIcon}</code>
              ) : (
                <span style={{ opacity: 0.7 }}>Chưa có biểu tượng (sẽ dùng icon mặc định của hệ thống)</span>
              )}
            </span>
          )}
        </div>

        {report.desktop_templates.length > 0 && (
          <div style={{ fontSize: "0.85rem" }}>
            <strong>Tệp mẫu launcher (.desktop):</strong>{" "}
            <code style={{ fontSize: "0.8rem", wordBreak: "break-all", padding: "0.3rem 0.5rem", background: "var(--input-bg)", border: "1px solid var(--border)", borderRadius: "4px", color: "var(--foreground)" }}>{report.desktop_templates[0].path}</code>
          </div>
        )}

        <div style={{ display: "flex", flexDirection: "column", gap: "0.35rem", paddingTop: "0.5rem", borderTop: "1px solid var(--border)" }}>
          <label style={{ fontSize: "0.85rem", fontWeight: 600 }}>Chế độ cài đặt & Lưu trữ (Relocation):</label>
          <div style={{ display: "flex", gap: "1.25rem", flexWrap: "wrap" }}>
            <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", cursor: "pointer", fontSize: "0.85rem" }}>
              <input
                type="radio"
                name="installType"
                value="InPlace"
                checked={installType === "InPlace"}
                onChange={() => setInstallType("InPlace")}
              />
              <span>Giữ nguyên tại chỗ (In-Place / Thư mục hiện tại)</span>
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: "0.4rem", cursor: "pointer", fontSize: "0.85rem" }}>
              <input
                type="radio"
                name="installType"
                value="Moved"
                checked={installType === "Moved"}
                onChange={() => setInstallType("Moved")}
              />
              <span>Quản lý tập trung (Relocate / Move vào ~/Applications)</span>
            </label>
          </div>
        </div>
      </div>

      <div style={{ display: "flex", gap: "0.75rem", justifyContent: "flex-end", marginTop: "0.5rem" }}>
        <button className="btn btn-ghost" onClick={clearDetection} disabled={busy}>
          ✕ Huỷ bỏ
        </button>
        <button
          className="btn btn-primary"
          onClick={handleIntegrate}
          disabled={busy || !selectedExec}
          style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}
        >
          <span>⚡</span>
          <span>{busy ? "Đang tích hợp..." : "Cài đặt & Tích hợp vào hệ thống"}</span>
        </button>
      </div>
    </article>
  );
}
