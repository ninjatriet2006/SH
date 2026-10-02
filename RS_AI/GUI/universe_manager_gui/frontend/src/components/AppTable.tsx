import { useState, useMemo, useEffect, useRef, useCallback } from "react";
import { ArrowUpDown, ArrowUp, ArrowDown } from "lucide-react";
import { useTranslation } from "../utils/i18n";
import type { AppEntry } from "../utils/contract";

interface AppTableProps {
  apps: AppEntry[];
  actions?: boolean;
  disabled?: boolean;
  selectedAppId?: string;
  onSelectApp?: (app: AppEntry) => void;
  checkedAppIds?: Set<string>;
  onToggleCheck?: (appId: string) => void;
  onToggleCheckAll?: () => void;
  onStart?: (appId: string) => void;
  onStop?: (appId: string) => void;
  onRestart?: (appId: string) => void;
}

type SortField = "name" | "category" | "source" | "status";
type SortDirection = "asc" | "desc";

interface ColumnWidths {
  name: number;
  category: number;
  source: number;
  status: number;
}

const DEFAULT_WIDTHS: ColumnWidths = {
  name: 240,
  category: 130,
  source: 90,
  status: 110,
};

const STORAGE_KEY = "um_table_col_widths_v2";

function loadSavedWidths(): ColumnWidths {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const parsed = JSON.parse(saved);
      if (parsed && typeof parsed.name === "number") {
        return {
          name: Math.max(160, parsed.name),
          category: Math.max(90, parsed.category ?? DEFAULT_WIDTHS.category),
          source: Math.max(75, parsed.source ?? DEFAULT_WIDTHS.source),
          status: Math.max(90, parsed.status ?? DEFAULT_WIDTHS.status),
        };
      }
    }
  } catch {
    // fallback
  }
  return DEFAULT_WIDTHS;
}

export function AppTable({
  apps,
  actions = false,
  disabled = false,
  selectedAppId,
  onSelectApp,
  checkedAppIds,
  onToggleCheck,
  onToggleCheckAll,
  onStart,
  onStop,
  onRestart,
}: AppTableProps) {
  const { t } = useTranslation();

  // Column width state with persistence
  const [colWidths, setColWidths] = useState<ColumnWidths>(loadSavedWidths);
  const [resizingCol, setResizingCol] = useState<keyof ColumnWidths | null>(null);

  // Sorting state
  const [sortField, setSortField] = useState<SortField | null>("name");
  const [sortDir, setSortDir] = useState<SortDirection>("asc");

  // Virtualization state
  const containerRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [containerHeight, setContainerHeight] = useState(540);
  const ROW_HEIGHT = 52;
  const OVERSCAN = 6;

  // Dynamic container height tracking for optimal virtualization
  useEffect(() => {
    if (!containerRef.current) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (entry.contentRect.height > 0) {
          setContainerHeight(entry.contentRect.height);
        }
      }
    });
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, []);

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      if (sortDir === "asc") {
        setSortDir("desc");
      } else {
        setSortField(null);
        setSortDir("asc");
      }
    } else {
      setSortField(field);
      setSortDir("asc");
    }
  };

  const sortedApps = useMemo(() => {
    if (!sortField) return apps;

    return [...apps].sort((a, b) => {
      let res = 0;
      if (sortField === "name") {
        res = a.name.localeCompare(b.name, "vi", { sensitivity: "base", numeric: true });
      } else if (sortField === "category") {
        const catA = a.category?.trim() || "Other";
        const catB = b.category?.trim() || "Other";
        res = catA.localeCompare(catB, "vi", { sensitivity: "base" });
      } else if (sortField === "source") {
        const srcA = a.package_type?.trim() || "System";
        const srcB = b.package_type?.trim() || "System";
        res = srcA.localeCompare(srcB);
      } else if (sortField === "status") {
        const isRunA = a.status === "Running" ? 1 : 0;
        const isRunB = b.status === "Running" ? 1 : 0;
        res = isRunB - isRunA; // Running first by default
      }
      return sortDir === "asc" ? res : -res;
    });
  }, [apps, sortField, sortDir]);

  // Resizing logic
  const handleStartResize = useCallback(
    (col: keyof ColumnWidths, e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const startX = e.clientX;
      const startWidth = colWidths[col];

      const minWidths: Record<keyof ColumnWidths, number> = {
        name: 160,
        category: 90,
        source: 75,
        status: 90,
      };

      setResizingCol(col);

      const onMouseMove = (moveEvent: MouseEvent) => {
        const delta = moveEvent.clientX - startX;
        const nextWidth = Math.max(minWidths[col], startWidth + delta);
        setColWidths((prev) => {
          const updated = { ...prev, [col]: nextWidth };
          try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
          } catch {
            // ignore
          }
          return updated;
        });
      };

      const onMouseUp = () => {
        setResizingCol(null);
        window.removeEventListener("mousemove", onMouseMove);
        window.removeEventListener("mouseup", onMouseUp);
      };

      window.addEventListener("mousemove", onMouseMove);
      window.addEventListener("mouseup", onMouseUp);
    },
    [colWidths]
  );

  const handleResetWidths = (e: React.MouseEvent) => {
    e.stopPropagation();
    setColWidths(DEFAULT_WIDTHS);
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      // ignore
    }
  };

  if (!apps.length) {
    return <p className="empty">{t("status.no_apps") || "Không tìm thấy ứng dụng nào."}</p>;
  }

  const hasCheckbox = Boolean(checkedAppIds);
  const allChecked = hasCheckbox && apps.length > 0 && apps.every((a) => checkedAppIds!.has(a.id));

  // Virtual slice calculations
  const totalCount = sortedApps.length;
  const totalHeight = totalCount * ROW_HEIGHT;
  const startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const endIndex = Math.min(
    totalCount,
    Math.ceil((scrollTop + containerHeight) / ROW_HEIGHT) + OVERSCAN
  );
  const visibleApps = sortedApps.slice(startIndex, endIndex);
  const offsetY = startIndex * ROW_HEIGHT;

  const renderSourceBadge = (app: AppEntry) => {
    const ptype = app.package_type?.toLowerCase() ?? "";
    if (ptype === "flatpak" || app.id.endsWith("-flatpak")) {
      return <span className="source-badge flatpak">Flatpak</span>;
    }
    if (ptype === "snap" || app.id.endsWith("-snap")) {
      return <span className="source-badge snap">Snap</span>;
    }
    if (ptype === "local" || ptype === "portable") {
      return <span className="source-badge portable">Local</span>;
    }
    if (ptype === "apt") {
      return <span className="source-badge apt">APT</span>;
    }
    if (ptype === "cli" || app.id.startsWith("cli-")) {
      return <span className="source-badge cli">CLI</span>;
    }
    return <span className="source-badge system">{app.package_type || "System"}</span>;
  };

  const renderStatusBadge = (status?: string | null) => {
    const isRunning = status === "Running";
    return (
      <span className={`status-badge ${isRunning ? "running" : "stopped"}`}>
        <span className="dot" />
        {isRunning ? "RUNNING" : "STOPPED"}
      </span>
    );
  };

  const renderSortIcon = (field: SortField) => {
    if (sortField !== field) {
      return <ArrowUpDown size={11} className="sort-hint" />;
    }
    return sortDir === "asc" ? (
      <span className="sort-indicator"><ArrowUp size={12} /></span>
    ) : (
      <span className="sort-indicator"><ArrowDown size={12} /></span>
    );
  };

  const rowLayoutClass = `${hasCheckbox ? "has-checkbox" : ""} ${actions ? "with-actions" : ""}`.trim();

  const cssVariables = {
    "--col-name": `${colWidths.name}px`,
    "--col-category": `${colWidths.category}px`,
    "--col-source": `${colWidths.source}px`,
    "--col-status": `${colWidths.status}px`,
  } as React.CSSProperties;

  return (
    <div className="data-table" style={cssVariables}>
      {/* Header Row with Sort & Resize Handles */}
      <div className={`table-row heading ${rowLayoutClass}`}>
        {hasCheckbox && (
          <span className="col-check">
            <input
              type="checkbox"
              checked={Boolean(allChecked)}
              onChange={onToggleCheckAll}
              title="Chọn tất cả ứng dụng"
            />
          </span>
        )}

        {/* Column: Tên */}
        <span
          className="table-header-cell col-name"
          onClick={() => handleSort("name")}
          title="Nhấn để sắp xếp theo Tên ứng dụng"
        >
          <span>{t("table.name") || "Tên ứng dụng"}</span>
          {renderSortIcon("name")}
          <div
            className={`col-resizer ${resizingCol === "name" ? "is-resizing" : ""}`}
            onMouseDown={(e) => handleStartResize("name", e)}
            onDoubleClick={handleResetWidths}
            title="Kéo để chỉnh độ rộng cột (Nhấp đúp để đặt lại)"
          />
        </span>

        {/* Column: Chuyên mục */}
        <span
          className="table-header-cell col-category"
          onClick={() => handleSort("category")}
          title="Nhấn để sắp xếp theo Chuyên mục"
        >
          <span>{t("table.category") || "Chuyên mục"}</span>
          {renderSortIcon("category")}
          <div
            className={`col-resizer ${resizingCol === "category" ? "is-resizing" : ""}`}
            onMouseDown={(e) => handleStartResize("category", e)}
            onDoubleClick={handleResetWidths}
            title="Kéo để chỉnh độ rộng cột (Nhấp đúp để đặt lại)"
          />
        </span>

        {/* Column: Nguồn */}
        <span
          className="table-header-cell col-source"
          onClick={() => handleSort("source")}
          title="Nhấn để sắp xếp theo Nguồn gói"
        >
          <span>{t("table.source") || "Nguồn"}</span>
          {renderSortIcon("source")}
          <div
            className={`col-resizer ${resizingCol === "source" ? "is-resizing" : ""}`}
            onMouseDown={(e) => handleStartResize("source", e)}
            onDoubleClick={handleResetWidths}
            title="Kéo để chỉnh độ rộng cột (Nhấp đúp để đặt lại)"
          />
        </span>

        {/* Column: Trạng thái */}
        <span
          className="table-header-cell col-status"
          onClick={() => handleSort("status")}
          title="Nhấn để sắp xếp theo Trạng thái (Đang chạy / Dừng)"
        >
          <span>{t("table.status") || "Trạng thái"}</span>
          {renderSortIcon("status")}
          <div
            className={`col-resizer ${resizingCol === "status" ? "is-resizing" : ""}`}
            onMouseDown={(e) => handleStartResize("status", e)}
            onDoubleClick={handleResetWidths}
            title="Kéo để chỉnh độ rộng cột (Nhấp đúp để đặt lại)"
          />
        </span>

        {actions && (
          <span className="table-header-cell col-actions" style={{ cursor: "default" }}>
            <span>{t("table.actions") || "Thao tác"}</span>
          </span>
        )}
      </div>

      {/* Virtual Scroll Body (Lazy Rendering for optimal 60fps performance) */}
      <div
        ref={containerRef}
        className="table-body-scroll"
        onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
      >
        <div style={{ height: `${totalHeight}px`, width: "100%", position: "relative" }}>
          <div
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              right: 0,
              transform: `translateY(${offsetY}px)`,
            }}
          >
            {visibleApps.map((app) => {
              const isRunning = app.status === "Running";
              const isSelected = selectedAppId === app.id;
              const isChecked = checkedAppIds ? checkedAppIds.has(app.id) : false;

              return (
                <div
                  className={`table-row ${isSelected ? "selected-row" : ""} ${rowLayoutClass}`}
                  key={app.id}
                  onClick={() => onSelectApp?.(app)}
                  role="button"
                  tabIndex={0}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      onSelectApp?.(app);
                    }
                  }}
                >
                  {hasCheckbox && (
                    <span
                      className="col-check"
                      onClick={(e) => {
                        e.stopPropagation();
                      }}
                    >
                      <input
                        type="checkbox"
                        checked={isChecked}
                        onChange={() => onToggleCheck?.(app.id)}
                      />
                    </span>
                  )}

                  <span
                    className="col-name"
                    title={app.exec_path ? `${app.name}\n${app.exec_path}` : app.name}
                    style={{ minWidth: 0 }}
                  >
                    <span className="app-name-text">{app.name}</span>
                    {app.symlink_file ? (
                      <span
                        style={{
                          fontSize: "0.72rem",
                          opacity: 0.65,
                          display: "block",
                          fontFamily: "monospace",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                      >
                        {app.symlink_file}
                      </span>
                    ) : app.package_type === "CLI" && app.exec_path ? (
                      <span
                        style={{
                          fontSize: "0.72rem",
                          opacity: 0.65,
                          display: "block",
                          fontFamily: "monospace",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                      >
                        {app.exec_path}
                      </span>
                    ) : null}
                  </span>

                  <span
                    className="col-category"
                    title={app.category ?? "Utility"}
                    style={{ minWidth: 0 }}
                  >
                    <span className="category-text">{app.category ?? "Other"}</span>
                  </span>

                  <span className="col-source">{renderSourceBadge(app)}</span>

                  <span className="col-status">{renderStatusBadge(app.status)}</span>

                  {actions && (
                    <span
                      className="col-actions action-buttons"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {!isRunning ? (
                        <button
                          type="button"
                          className="btn btn-sm btn-primary"
                          disabled={disabled}
                          onClick={() => onStart?.(app.id)}
                          title={t("manager.start") || "Khởi động"}
                        >
                          ▶
                        </button>
                      ) : (
                        <>
                          <button
                            type="button"
                            className="btn btn-sm btn-danger"
                            disabled={disabled}
                            onClick={() => onStop?.(app.id)}
                            title={t("manager.stop") || "Dừng"}
                          >
                            ⏹
                          </button>
                          {onRestart && (
                            <button
                              type="button"
                              className="btn btn-sm btn-ghost"
                              disabled={disabled}
                              onClick={() => onRestart?.(app.id)}
                              title="Khởi động lại"
                            >
                              ⟳
                            </button>
                          )}
                        </>
                      )}
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
