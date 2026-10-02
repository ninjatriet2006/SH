import { Link } from "react-router-dom";
import { useAppStore } from "../store/useAppStore";
import { useTranslation } from "../utils/i18n";

export function DashboardPage() {
  const { t } = useTranslation();
  const config = useAppStore(s => s.config);
  const apps = config?.apps ?? [];
  const managedDir = config?.settings.managed_dir?.trim() ?? "";

  const runningCount = apps.filter(a => a.status === "Running").length;
  const systemCount = apps.filter(a => a.package_type !== "Local").length;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      <section className="cards">
        <article className="glass-panel card">
          <strong>{apps.length}</strong>
          <span>{t("dashboard.managed_apps") ?? "Tổng số ứng dụng"}</span>
        </article>
        <article className="glass-panel card">
          <strong style={{ color: "var(--badge-success)" }}>{runningCount}</strong>
          <span>🟢 Đang chạy</span>
        </article>
        <article className="glass-panel card">
          <strong style={{ color: "var(--accent)" }}>{systemCount}</strong>
          <span>🌐 Flatpak / Snap / Hệ thống</span>
        </article>
        <article className="glass-panel card">
          <strong className="break-word" style={{ fontSize: "0.85rem" }}>{managedDir || "Chưa chọn (Tùy chọn)"}</strong>
          <span>💼 Thư mục Portable</span>
        </article>
      </section>

      {apps.length === 0 ? (
        <section className="glass-panel alert-card info">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
            <h3 style={{ fontSize: "1.05rem" }}>⚡ Đang quét ngầm ứng dụng hệ thống…</h3>
            <p style={{ fontSize: "0.9rem", color: "var(--panel-foreground)" }}>
              Hệ thống đang tự động tìm kiếm các phần mềm Flatpak, Snap và Desktop applications.
            </p>
          </div>
          <Link to="/scan" className="btn btn-primary" style={{ textDecoration: "none", alignSelf: "flex-start" }}>
            {t("nav.scan")} →
          </Link>
        </section>
      ) : (
        <section className="glass-panel" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <h3 style={{ fontSize: "1.05rem", marginBottom: "0.25rem" }}>
              🚀 {apps.length} ứng dụng sẵn sàng quản lý ({runningCount} đang chạy)
            </h3>
            <p style={{ fontSize: "0.88rem", color: "var(--panel-foreground)" }}>
              {t("dashboard.apps_ready") ?? "Tất cả ứng dụng đã sẵn sàng quản lý, kiểm tra trạng thái và khởi chạy."}
            </p>
          </div>
          <Link to="/manager" className="btn btn-primary" style={{ textDecoration: "none" }}>
            {t("nav.manager")} →
          </Link>
        </section>
      )}

      <section className="glass-panel" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "1rem", borderLeft: "4px solid var(--accent)" }}>
        <div>
          <h3 style={{ fontSize: "1.05rem", marginBottom: "0.25rem" }}>
            📦 Tích hợp ứng dụng mới từ thư mục ngoài (Downloads, Documents…)
          </h3>
          <p style={{ fontSize: "0.88rem", color: "var(--panel-foreground)", margin: 0 }}>
            Tự động chuyển vào ~/Applications, tạo launcher menu (.desktop) và liên kết lệnh terminal ($PATH).
          </p>
        </div>
        <Link to="/add" className="btn btn-primary" style={{ textDecoration: "none", whiteSpace: "nowrap" }}>
          + Thêm ứng dụng mới →
        </Link>
      </section>
    </div>
  );
}
