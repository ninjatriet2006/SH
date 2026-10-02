import { useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  PackagePlus,
  FolderOpen,
  Search,
  ArrowRight,
  Terminal,
  LayoutGrid,
  CheckCircle2,
  FolderSync,
  HelpCircle,
} from "lucide-react";
import { useAppStore } from "../store/useAppStore";
import { DetectionReport } from "../components/DetectionReport";

export function AddAppPage() {
  const navigate = useNavigate();
  const detection = useAppStore((s) => s.detection);
  const clearDetection = useAppStore((s) => s.clearDetection);
  const pickAndDetect = useAppStore((s) => s.pickAndDetect);
  const detectApp = useAppStore((s) => s.detectApp);
  const busy = useAppStore((s) => s.busy);

  const [inputPath, setInputPath] = useState("");

  const handleManualInspect = () => {
    const trimmed = inputPath.trim();
    if (trimmed) {
      void detectApp(trimmed);
    }
  };

  return (
    <div className="add-app-page-layout" style={{ display: "flex", flexDirection: "column", gap: "1.25rem", maxWidth: "980px", margin: "0 auto" }}>
      {/* Header Banner */}
      <header className="glass-panel" style={{ padding: "1.25rem 1.5rem" }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", flexWrap: "wrap", gap: "0.75rem" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "0.85rem" }}>
            <div
              style={{
                width: "44px",
                height: "44px",
                borderRadius: "12px",
                background: "linear-gradient(135deg, var(--accent), #a855f7)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "#ffffff",
                boxShadow: "0 4px 14px rgba(99, 102, 241, 0.35)",
              }}
            >
              <PackagePlus size={24} />
            </div>
            <div>
              <h1 style={{ fontSize: "1.35rem", fontWeight: "700", margin: 0, letterSpacing: "-0.01em" }}>
                Thêm & Tích hợp Ứng dụng Mới
              </h1>
              <p style={{ margin: "0.2rem 0 0", fontSize: "0.88rem", color: "var(--panel-foreground)" }}>
                Tích hợp phần mềm giải nén từ Downloads, Documents, ổ cứng ngoài, AppImage hoặc binary độc lập vào hệ thống.
              </p>
            </div>
          </div>

          {detection && (
            <button
              type="button"
              className="btn btn-ghost"
              onClick={clearDetection}
              style={{ fontSize: "0.85rem" }}
            >
              ✕ Hủy / Chọn tệp khác
            </button>
          )}
        </div>
      </header>

      {/* Main Content Area */}
      {detection ? (
        <section>
          <div style={{ marginBottom: "0.75rem", display: "flex", alignItems: "center", gap: "0.5rem", color: "var(--accent)" }}>
            <CheckCircle2 size={18} />
            <span style={{ fontSize: "0.92rem", fontWeight: 600 }}>
              Đã phân tích cấu trúc ứng dụng. Vui lòng kiểm tra và tùy chỉnh thông số tích hợp bên dưới:
            </span>
          </div>
          <DetectionReport
            report={detection}
            onComplete={(_entry) => {
              navigate("/manager");
            }}
          />
        </section>
      ) : (
        <>
          {/* Source Selection Panel */}
          <section className="glass-panel" style={{ padding: "1.75rem 1.5rem", display: "flex", flexDirection: "column", gap: "1.5rem" }}>
            <div style={{ textAlign: "center", maxWidth: "600px", margin: "0 auto", display: "flex", flexDirection: "column", gap: "0.5rem" }}>
              <h2 style={{ fontSize: "1.15rem", fontWeight: 600, margin: 0 }}>
                Bước 1: Chọn thư mục hoặc tệp phần mềm nguồn
              </h2>
              <p style={{ fontSize: "0.88rem", color: "var(--panel-foreground)", margin: 0 }}>
                Bạn có thể mở hộp thoại chọn thư mục phần mềm vừa giải nén, hoặc dán trực tiếp đường dẫn tuyệt đối.
              </p>
            </div>

            <div style={{ display: "flex", justifyContent: "center" }}>
              <button
                type="button"
                className="btn btn-primary"
                disabled={busy}
                onClick={() => void pickAndDetect()}
                style={{
                  padding: "0.85rem 1.75rem",
                  fontSize: "1rem",
                  fontWeight: 600,
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "0.6rem",
                  boxShadow: "0 6px 20px rgba(99, 102, 241, 0.35)",
                }}
              >
                <FolderOpen size={20} />
                <span>{busy ? "Đang phân tích gói…" : "Chọn thư mục giải nén…"}</span>
              </button>
            </div>

            <div style={{ display: "flex", alignItems: "center", gap: "1rem", margin: "0.5rem 0" }}>
              <div style={{ flex: 1, height: "1px", background: "var(--border)" }} />
              <span style={{ fontSize: "0.8rem", color: "var(--panel-foreground)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                Hoặc nhập đường dẫn trực tiếp
              </span>
              <div style={{ flex: 1, height: "1px", background: "var(--border)" }} />
            </div>

            <div style={{ display: "flex", gap: "0.75rem", maxWidth: "720px", width: "100%", margin: "0 auto" }}>
              <div style={{ position: "relative", flex: 1 }}>
                <Search
                  size={16}
                  style={{
                    position: "absolute",
                    left: "0.85rem",
                    top: "50%",
                    transform: "translateY(-50%)",
                    color: "var(--panel-foreground)",
                  }}
                />
                <input
                  type="text"
                  placeholder="/home/bimatkeo/Downloads/my-app hoặc /opt/matlab/bin/matlab..."
                  value={inputPath}
                  onChange={(e) => setInputPath(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleManualInspect();
                  }}
                  style={{
                    width: "100%",
                    padding: "0.65rem 0.85rem 0.65rem 2.4rem",
                    borderRadius: "8px",
                    border: "1px solid var(--border)",
                    background: "var(--canvas)",
                    color: "var(--foreground)",
                    fontSize: "0.9rem",
                  }}
                />
              </div>
              <button
                type="button"
                className="btn btn-ghost"
                disabled={!inputPath.trim() || busy}
                onClick={handleManualInspect}
                style={{ whiteSpace: "nowrap", display: "inline-flex", alignItems: "center", gap: "0.4rem" }}
              >
                <span>Kiểm tra gói</span>
                <ArrowRight size={15} />
              </button>
            </div>
          </section>

          {/* Integration Capabilities Guide */}
          <section style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "1rem" }}>
            <article className="glass-panel" style={{ padding: "1.25rem", display: "flex", flexDirection: "column", gap: "0.65rem" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", color: "var(--accent)" }}>
                <FolderSync size={20} />
                <h3 style={{ fontSize: "1rem", fontWeight: 600, margin: 0 }}>1. Di chuyển tập trung</h3>
              </div>
              <p style={{ fontSize: "0.85rem", color: "var(--panel-foreground)", margin: 0, lineHeight: 1.5 }}>
                Tự động di dời ứng dụng từ các thư mục tạm (Downloads, Desktop, Temp) về <code>~/Applications</code>. Đảm bảo toàn vẹn dữ liệu, tự động cấp quyền thực thi (<code>chmod +x</code>) và không sợ xóa nhầm.
              </p>
            </article>

            <article className="glass-panel" style={{ padding: "1.25rem", display: "flex", flexDirection: "column", gap: "0.65rem" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", color: "#a855f7" }}>
                <LayoutGrid size={20} />
                <h3 style={{ fontSize: "1rem", fontWeight: 600, margin: 0 }}>2. Tạo Launcher (.desktop)</h3>
              </div>
              <p style={{ fontSize: "0.85rem", color: "var(--panel-foreground)", margin: 0, lineHeight: 1.5 }}>
                Khởi tạo tệp <code>.desktop</code> chuẩn FreeDesktop trong <code>~/.local/share/applications</code>. Xuất hiện ngay trong Application Menu của hệ thống (GNOME/KDE/XFCE), ghim vào Dock và Taskbar với icon sắc nét.
              </p>
            </article>

            <article className="glass-panel" style={{ padding: "1.25rem", display: "flex", flexDirection: "column", gap: "0.65rem" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "0.6rem", color: "#10b981" }}>
                <Terminal size={20} />
                <h3 style={{ fontSize: "1rem", fontWeight: 600, margin: 0 }}>3. Lệnh Terminal ($PATH)</h3>
              </div>
              <p style={{ fontSize: "0.85rem", color: "var(--panel-foreground)", margin: 0, lineHeight: 1.5 }}>
                Tự động tạo liên kết symbolic link trong <code>~/.local/bin/&lt;tên_lệnh&gt;</code>. Cho phép gọi phần mềm trực tiếp từ bất kỳ cửa sổ terminal nào (như <code>grok</code>, <code>claude</code>, <code>antigravity</code>).
              </p>
            </article>
          </section>

          {/* Helpful Tips Panel */}
          <footer className="glass-panel" style={{ padding: "1rem 1.25rem", display: "flex", alignItems: "center", gap: "0.75rem", fontSize: "0.85rem", color: "var(--panel-foreground)" }}>
            <HelpCircle size={18} style={{ flexShrink: 0, color: "var(--accent)" }} />
            <span>
              <strong>Mẹo:</strong> Đối với các gói như MATLAB, JetBrains IDEs hoặc AppImage, Universe Manager sẽ tự động quét sâu tìm tệp thực thi chính (main binary) và tệp biểu tượng (PNG/SVG) để bạn không phải tự cấu hình thủ công.
            </span>
          </footer>
        </>
      )}
    </div>
  );
}
