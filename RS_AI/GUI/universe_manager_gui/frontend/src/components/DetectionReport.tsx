import { useTranslation } from "../utils/i18n";
import type { DetectionReport as DetectionReportType } from "../utils/contract";

interface DetectionReportProps {
  report: DetectionReportType;
}

export function DetectionReport({ report }: DetectionReportProps) {
  const { t } = useTranslation();

  return (
    <article className="glass-panel report">
      <h3>{report.suggested_name}</h3>
      <p>
        <strong>{t("detection.appimage")}:</strong>{" "}
        {report.is_appimage ? "Yes" : "No"}
      </p>
      <PathList label={t("detection.executables")} items={report.executables} />
      <PathList label={t("detection.icons")} items={report.icons} />
      <PathList label={t("detection.desktop")} items={report.desktop_templates} />
    </article>
  );
}

function PathList({ label, items }: { label: string; items: { path: string }[] }) {
  return (
    <p>
      <strong>{label}:</strong>{" "}
      {items.length ? items.map((item) => item.path).join(", ") : "—"}
    </p>
  );
}
