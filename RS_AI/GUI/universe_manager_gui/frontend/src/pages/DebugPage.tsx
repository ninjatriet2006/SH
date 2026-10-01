import React, { useState, useMemo, useRef, useEffect } from "react";
import {
  Bug,
  Copy,
  Trash2,
  Download,
  Search,
  Check,
  Play,
  Square,
  AlertTriangle,
  AlertCircle,
  CheckCircle2,
  Info,
  Sparkles,
  ArrowDownCircle,
  ArrowUpDown,
} from "lucide-react";
import {
  useNotificationStore,
  type LogItem,
} from "../store/useNotificationStore";
import { useTranslation } from "../utils/i18n";

export const DebugPage: React.FC = () => {
  const { logs, clearLogs, exportLogs, notify, removeLog } = useNotificationStore();
  const { t } = useTranslation();

  const [selectedLevel, setSelectedLevel] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [copied, setCopied] = useState<boolean>(false);
  const [autoScroll, setAutoScroll] = useState<boolean>(true);
  const [sortOrder, setSortOrder] = useState<"asc" | "desc">("asc");

  const logEndRef = useRef<HTMLDivElement>(null);
  const logTopRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!autoScroll) return;
    if (sortOrder === "asc" && logEndRef.current) {
      logEndRef.current.scrollIntoView({ behavior: "smooth" });
    } else if (sortOrder === "desc" && logTopRef.current) {
      logTopRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, autoScroll, sortOrder]);

  const filteredLogs = useMemo(() => {
    const matched = logs.filter((log) => {
      const matchLevel =
        selectedLevel === "all" || log.level === selectedLevel;
      const matchSearch =
        searchQuery.trim() === "" ||
        log.message.toLowerCase().includes(searchQuery.toLowerCase()) ||
        (log.tag && log.tag.toLowerCase().includes(searchQuery.toLowerCase()));
      return matchLevel && matchSearch;
    });

    return sortOrder === "asc" ? matched : [...matched].reverse();
  }, [logs, selectedLevel, searchQuery, sortOrder]);

  const levelCounts = useMemo(() => {
    const counts: Record<string, number> = {
      all: logs.length,
      info: 0,
      start: 0,
      stop: 0,
      warning: 0,
      error: 0,
      success: 0,
      debug: 0,
    };
    logs.forEach((l) => {
      if (counts[l.level] !== undefined) {
        counts[l.level]++;
      }
    });
    return counts;
  }, [logs]);

  const handleCopy = async () => {
    const content = filteredLogs
      .map(
        (l) =>
          `[${l.timestamp.toLocaleTimeString()}][${l.level.toUpperCase()}][${l.tag ?? "APP"}] ${l.message}`
      )
      .join("\n");
    await navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleTestLogs = () => {
    notify("info", "Kiểm tra dòng thông báo thông thường (INFO)", {
      tag: "TEST",
    });
    notify("start", "Bắt đầu khởi chạy dịch vụ kiểm thử (START)", {
      tag: "TEST",
    });
    notify("stop", "Đã dừng dịch vụ kiểm thử (STOP)", { tag: "TEST" });
    notify("warning", "Cảnh báo tài nguyên hệ thống (WARNING)", {
      tag: "TEST",
    });
    notify("error", "Lỗi kiểm thử kết nối ngoại vi (ERROR)", { tag: "TEST" });
    notify("success", "Tất cả bài kiểm tra đã hoàn thành mỹ mãn! (SUCCESS)", {
      tag: "TEST",
    });
    notify("debug", "Trace chi tiết luồng xử lý IPC #429 (DEBUG)", {
      tag: "TEST",
    });
  };

  return (
    <div className="debug-page">
      <div className="page-header">
        <div className="title-row">
          <Bug size={24} className="page-icon" />
          <div>
            <h1>{t("debug.title") || "Nhật ký hệ thống & Gỡ lỗi"}</h1>
            <p className="subtitle">
              {t("debug.description") ||
                "Theo dõi mọi hành động, trạng thái và chẩn đoán chi tiết theo thời gian thực."}
            </p>
          </div>
        </div>

        <div className="header-actions">
          <button
            type="button"
            className="btn btn-secondary"
            onClick={handleTestLogs}
            title="Tạo dòng thông báo mẫu mọi cấp độ để kiểm tra"
          >
            <Sparkles size={16} /> {t("debug.test_logs") || "Thử nghiệm log"}
          </button>
          <button
            type="button"
            className="btn btn-secondary"
            onClick={handleCopy}
            title="Sao chép toàn bộ dòng log hiện tại vào bộ nhớ tạm"
          >
            {copied ? (
              <>
                <Check size={16} style={{ color: "var(--badge-success)" }} />{" "}
                {t("debug.copied") || "Đã chép!"}
              </>
            ) : (
              <>
                <Copy size={16} /> {t("debug.copy_all") || "Sao chép"}
              </>
            )}
          </button>
          <button
            type="button"
            className="btn btn-secondary"
            onClick={exportLogs}
            title="Tải toàn bộ lịch sử log về máy định dạng .txt"
          >
            <Download size={16} /> {t("debug.export") || "Tải về (.txt)"}
          </button>
          <button
            type="button"
            className="btn btn-danger"
            onClick={clearLogs}
            title="Xóa toàn bộ dòng log trong phiên hiện tại"
          >
            <Trash2 size={16} /> {t("debug.clear") || "Xóa nhật ký"}
          </button>
        </div>
      </div>

      <div className="debug-toolbar">
        <div className="search-box">
          <Search size={16} className="search-icon" />
          <input
            type="text"
            placeholder={
              t("debug.search_placeholder") || "Tìm kiếm dòng nhật ký…"
            }
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
          {searchQuery && (
            <button
              type="button"
              className="clear-search-btn"
              onClick={() => setSearchQuery("")}
            >
              ×
            </button>
          )}
        </div>

        <div className="filter-chips">
          {(
            [
              "all",
              "info",
              "start",
              "stop",
              "warning",
              "error",
              "success",
              "debug",
            ] as const
          ).map((lvl) => (
            <button
              key={lvl}
              type="button"
              className={`chip ${selectedLevel === lvl ? "active" : ""}`}
              onClick={() => setSelectedLevel(lvl)}
            >
              {lvl.toUpperCase()} ({levelCounts[lvl] ?? 0})
            </button>
          ))}
        </div>

        <label className="auto-scroll-toggle">
          <input
            type="checkbox"
            checked={autoScroll}
            onChange={(e) => setAutoScroll(e.target.checked)}
          />
          <ArrowDownCircle size={15} />
          <span>{t("debug.auto_scroll") || "Tự cuộn"}</span>
        </label>
      </div>

      <div className="debug-console-card">
        <div className="console-header">
          <div className="console-header-left">
            <span>CONSOLE LOGS ({filteredLogs.length} dòng)</span>
            <span className="console-status-dot" />
          </div>
          <div className="console-header-actions">
            <button
              type="button"
              className="btn btn-xs btn-ghost console-order-btn"
              onClick={() => setSortOrder((prev) => (prev === "asc" ? "desc" : "asc"))}
              title="Đảo chiều thứ tự hiển thị logs"
            >
              <ArrowUpDown size={12} />
              <span>{sortOrder === "asc" ? "Thứ tự: Cũ ➔ Mới (Terminal)" : "Thứ tự: Mới ➔ Cũ (Feed)"}</span>
            </button>
          </div>
        </div>

        <div className="console-body">
          <div ref={logTopRef} />
          {filteredLogs.length === 0 ? (
            <div className="empty-console">
              <Bug size={32} className="empty-icon" />
              <p>{t("debug.no_logs") || "Chưa có dòng nhật ký nào phù hợp."}</p>
            </div>
          ) : (
            filteredLogs.map((log: LogItem) => {
              const time = log.timestamp.toLocaleTimeString();
              let icon = <Info size={13} />;

              if (log.level === "start") {
                icon = <Play size={13} />;
              } else if (log.level === "stop") {
                icon = <Square size={13} />;
              } else if (log.level === "warning") {
                icon = <AlertTriangle size={13} />;
              } else if (log.level === "error") {
                icon = <AlertCircle size={13} />;
              } else if (log.level === "success") {
                icon = <CheckCircle2 size={13} />;
              } else if (log.level === "debug") {
                icon = <Bug size={13} />;
              }

              return (
                <div key={log.id} className={`log-row log-${log.level}`}>
                  <span className="log-time">[{time}]</span>
                  <span className={`log-badge badge-${log.level}`}>
                    {icon}
                    <span>{log.level.toUpperCase()}</span>
                  </span>
                  {log.tag && <span className="log-tag">[{log.tag}]</span>}
                  <span className="log-msg">{log.message}</span>
                  <button
                    type="button"
                    className="log-dismiss-btn"
                    onClick={() => removeLog(log.id)}
                    title={t("debug.dismiss_row") || "Đóng dòng nhật ký này"}
                  >
                    ×
                  </button>
                </div>
              );
            })
          )}
          <div ref={logEndRef} />
        </div>
      </div>
    </div>
  );
};
