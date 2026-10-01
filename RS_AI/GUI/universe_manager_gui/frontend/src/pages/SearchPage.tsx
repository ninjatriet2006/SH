import { useState, type FormEvent } from "react";
import { useAppStore } from "../store/useAppStore";
import { useTranslation } from "../utils/i18n";

export function SearchPage() {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const busy = useAppStore(s => s.busy);
  const search = useAppStore(s => s.search);
  const searchApps = useAppStore(s => s.searchApps);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    const trimmed = query.trim();
    if (trimmed) void searchApps(trimmed);
  };

  return (
    <section className="glass-panel">
      <form className="row" onSubmit={handleSubmit}>
        <input
          className="input"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          required
          placeholder={t("search.placeholder")}
        />
        <button className="btn btn-primary" disabled={busy}>
          {t("search.search_btn")}
        </button>
      </form>

      {search && (
        search.results.length === 0
          ? <p className="empty">{t("status.no_results")}</p>
          : (
            <div className="data-table">
              <div className="table-row heading">
                <span>{t("table.name")}</span>
                <span>{t("table.id")}</span>
                <span>{t("table.version")}</span>
                <span>{t("table.source")}</span>
              </div>
              {search.results.map((item, i) => (
                <div className="table-row" key={`${item.id}-${i}`}>
                  <span>{item.name}</span>
                  <span>{item.id}</span>
                  <span>{item.version}</span>
                  <span>{item.source}</span>
                </div>
              ))}
            </div>
          )
      )}
    </section>
  );
}
