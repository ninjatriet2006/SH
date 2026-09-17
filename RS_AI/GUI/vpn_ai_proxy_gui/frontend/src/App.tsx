import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AlertTriangle, X } from "lucide-react";
import {
  GatewayConfig,
  OutboundTunnel,
  RouteRule,
  RawTrafficLog,
} from "./types";
import { Sidebar } from "./components/Sidebar";
import { TrafficTab } from "./components/TrafficTab";
import { RoutesTab } from "./components/RoutesTab";
import { TunnelsTab } from "./components/TunnelsTab";
import { FingerprintTab } from "./components/FingerprintTab";
import { SettingsTab } from "./components/SettingsTab";
import { Modals } from "./components/Modals";

export function App() {
  const [activeTab, setActiveTab] = useState<"traffic" | "routes" | "tunnels" | "fingerprint" | "settings">("traffic");
  const [config, setConfig] = useState<GatewayConfig | null>(null);
  const [trafficLogs, setTrafficLogs] = useState<RawTrafficLog[]>([]);
  const [selectedLog, setSelectedLog] = useState<RawTrafficLog | null>(null);

  const [editingTunnel, setEditingTunnel] = useState<OutboundTunnel | null>(null);
  const [editingRoute, setEditingRoute] = useState<RouteRule | null>(null);
  const [activeCliTunnelId, setActiveCliTunnelId] = useState<string | null>(null);
  const [bindError, setBindError] = useState<{ port: number; error: string } | null>(null);

  const fetchConfig = useCallback(async () => {
    try {
      const cfg = await invoke<GatewayConfig>("get_config");
      setConfig(cfg);
    } catch (e) {
      console.error(e);
    }
  }, []);

  const fetchActiveCliTunnel = useCallback(async () => {
    try {
      const id = await invoke<string | null>("get_active_cli_tunnel");
      setActiveCliTunnelId(id);
    } catch (e) {
      console.error(e);
    }
  }, []);

  const fetchTraffic = useCallback(async () => {
    try {
      const list = await invoke<RawTrafficLog[]>("get_raw_traffic");
      setTrafficLogs(list.reverse());
    } catch (e) {
      console.error(e);
    }
  }, []);

  useEffect(() => {
    fetchConfig();
    fetchTraffic();
    fetchActiveCliTunnel();
    const interval = setInterval(() => {
      // Chỉ poll traffic khi user đang mở tab traffic để tiết kiệm IPC serialization
      if (activeTab === "traffic") {
        fetchTraffic();
      }
      fetchActiveCliTunnel();
    }, 1500);

    const unlistenPromise = listen<{ port: number; error: string }>(
      "endpoint-bind-error",
      async (event) => {
        const { port, error } = event.payload;
        setBindError({ port, error });
        fetchConfig(); // Reload config to get the updated status and last_error
      }
    );

    // Lắng nghe sự kiện cập nhật Key Real-time khi key bị xóa do 401/403
    const unlistenKeysPromise = listen("route-keys-updated", () => {
      fetchConfig();
    });

    // Lắng nghe sự kiện định kỳ kiểm tra sức khỏe Tunnel (Proactive VPN check)
    const unlistenTunnelPromise = listen("tunnel-status-changed", () => {
      fetchConfig();
    });

    return () => {
      clearInterval(interval);
      unlistenPromise.then((unlisten) => unlisten());
      unlistenKeysPromise.then((unlisten) => unlisten());
      unlistenTunnelPromise.then((unlisten) => unlisten());
    };
  }, [fetchConfig, fetchTraffic, fetchActiveCliTunnel, activeTab]);

  const handleSaveConfig = async (newConfig: GatewayConfig) => {
    try {
      await invoke("save_config", { newConfig });
      setConfig(newConfig);
    } catch (e) {
      console.error(e);
    }
  };

  const handleAdvanceKey = async (routeId: string) => {
    try {
      await invoke("advance_endpoint_key", { routeId });
      await fetchConfig();
    } catch (e) {
      console.error(e);
    }
  };

  // Kết quả test đọc trực tiếp từ tunnel.last_exit_ip/last_error (backend đã persist vào config),
  // không giữ bản sao testResults riêng (tránh Trùng lặp State React vs Rust).
  const handleTestTunnel = async (tunnel: OutboundTunnel) => {
    try {
      await invoke("test_single_tunnel", { tunnel });
      await fetchConfig();
    } catch (err) {
      console.error(err);
    }
  };

  const handleAddOrUpdateRoute = async () => {
    if (!editingRoute) return;
    try {
      await invoke("add_or_update_route", { route: editingRoute });
      setEditingRoute(null);
      await fetchConfig();
    } catch (e) {
      console.error(e);
    }
  };

  const handleDeleteRoute = async (id: string) => {
    try {
      await invoke("delete_route", { routeId: id });
      await fetchConfig();
    } catch (e) {
      console.error(e);
    }
  };

  // Backend toggle_tunnel đã ôm toàn bộ vòng đời process (start/test/stop) —
  // Frontend chỉ gọi đúng 1 IPC duy nhất, không gọi rời rạc start/stop nữa.
  const handleToggleTunnel = async (tunnelId: string, enabled: boolean) => {
    try {
      await invoke("toggle_tunnel", { tunnelId, enabled });
      await fetchConfig();
      await fetchActiveCliTunnel();
    } catch (e) {
      console.error("Toggle tunnel error:", e);
      await fetchConfig();
      await fetchActiveCliTunnel();
      window.alert(String(e));
    }
  };

  const handleToggleRoute = async (routeId: string, enabled: boolean) => {
    try {
      setBindError(null);
      await invoke("toggle_route", { routeId, enabled });
      await fetchConfig();
    } catch (e) {
      console.error("Toggle route error:", e);
      window.alert(String(e));
    }
  };

  const handleForceStopTunnel = async (tunnelId: string) => {
    try {
      await invoke("force_stop_tunnel", { tunnelId });
      await fetchConfig();
      await fetchActiveCliTunnel();
    } catch (e) {
      console.error("Force stop tunnel error:", e);
      window.alert(String(e));
    }
  };

  const handleAddOrUpdateTunnel = async () => {
    if (!editingTunnel) return;
    try {
      await invoke("add_or_update_tunnel", { tunnel: editingTunnel });
      setEditingTunnel(null);
      await fetchConfig();
    } catch (e) {
      console.error(e);
    }
  };

  const handleDeleteTunnel = async (id: string) => {
    try {
      await invoke("delete_tunnel", { tunnelId: id });
      await fetchConfig();
    } catch (e) {
      console.error(e);
    }
  };

  const handleClearLogs = async () => {
    await invoke("clear_logs");
    setTrafficLogs([]);
    setSelectedLog(null);
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[#090d16] text-slate-100 font-sans">
      <Sidebar
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        config={config}
        trafficLogs={trafficLogs}
      />

      <main className="flex-1 flex flex-col overflow-hidden">
        {activeTab === "traffic" && (
          <TrafficTab
            trafficLogs={trafficLogs}
            selectedLog={selectedLog}
            setSelectedLog={setSelectedLog}
            onClearLogs={handleClearLogs}
            maxLogEntries={config?.max_log_entries || 500}
            onUpdateMaxLogEntries={async (count) => {
              if (config) {
                await handleSaveConfig({ ...config, max_log_entries: count });
              }
            }}
          />
        )}

        {activeTab === "routes" && config && (
          <RoutesTab
            config={config}
            onToggleRoute={handleToggleRoute}
            onEditRoute={(r) => setEditingRoute({ ...r })}
            onSaveRouteDirect={async (route) => {
              try {
                await invoke("add_or_update_route", { route });
                await fetchConfig();
              } catch (e) {
                console.error(e);
              }
            }}
            onDeleteRoute={handleDeleteRoute}
            onAdvanceKey={handleAdvanceKey}
            onCreateRoute={() =>
              setEditingRoute({
                id: `route_${Date.now()}`,
                name: "Custom Endpoint",
                port: 3000,
                path_prefix: "/v1",
                target_base_url: "https://abc.xyz/v1",
                tunnel_id: config.tunnels[0]?.id || "direct_bypass",
                enabled: true,
                key_manager: {
                  key_file_path: "",
                  current_key_index: 0,
                  total_keys: 0,
                },
              })
            }
          />
        )}

        {activeTab === "tunnels" && config && (
          <TunnelsTab
            config={config}
            activeCliTunnelId={activeCliTunnelId}
            onTestTunnel={handleTestTunnel}
            onToggleTunnel={handleToggleTunnel}
            onForceStop={handleForceStopTunnel}
            onEditTunnel={(t) => setEditingTunnel({ ...t })}
            onDeleteTunnel={handleDeleteTunnel}
            onCreateTunnel={() =>
              setEditingTunnel({
                id: `tunnel_${Date.now()}`,
                name: "Custom VPN Tunnel",
                protocol: "socks5",
                endpoint: "127.0.0.1:1081",
                enabled: true,
                tags: ["custom"],
              })
            }
          />
        )}

        {activeTab === "fingerprint" && config && (
          <FingerprintTab
            config={config}
            onSaveConfig={handleSaveConfig}
          />
        )}

        {activeTab === "settings" && config && (
          <SettingsTab
            config={config}
            onSaveConfig={handleSaveConfig}
          />
        )}
      </main>

      <Modals
        editingRoute={editingRoute}
        setEditingRoute={setEditingRoute}
        onSaveRoute={handleAddOrUpdateRoute}
        editingTunnel={editingTunnel}
        setEditingTunnel={setEditingTunnel}
        onSaveTunnel={handleAddOrUpdateTunnel}
        config={config}
      />

      {/* MODAL CẢNH BÁO ĐỤNG CỔNG ENDPOINT */}
      {bindError && (
        <div className="fixed inset-0 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4 z-50 animate-in fade-in duration-200">
          <div className="bg-slate-900 border-2 border-rose-500 rounded-2xl w-full max-w-md p-6 space-y-4 shadow-2xl shadow-rose-950/50">
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2.5 bg-rose-500/20 border border-rose-500/40 rounded-xl text-rose-400">
                  <AlertTriangle className="w-6 h-6" />
                </div>
                <div>
                  <h3 className="text-sm font-bold text-rose-400 tracking-wide uppercase">
                    ⚠️ Lỗi Chiếm Dụng Cổng Endpoint
                  </h3>
                  <p className="text-xs text-slate-300 font-mono mt-0.5">
                    Port: <span className="text-rose-300 font-bold">{bindError.port}</span>
                  </p>
                </div>
              </div>
              <button
                onClick={() => setBindError(null)}
                className="p-1 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <div className="space-y-2.5 text-xs text-slate-300 bg-slate-950 p-4 rounded-xl border border-slate-800/80">
              <p>
                Không thể khởi động Proxy Server tại cổng{" "}
                <strong className="text-rose-400 font-mono">{bindError.port}</strong>!
              </p>
              <p className="text-[11px] font-mono text-rose-300/80 bg-rose-950/30 p-2 rounded border border-rose-900/50 break-all">
                {bindError.error}
              </p>
              <div className="pt-1 text-[11px] text-slate-400 space-y-1">
                <p>
                  <strong className="text-slate-200">Nguyên nhân:</strong> Có thể bạn đang mở nhiều cửa sổ App cùng lúc, hoặc phần mềm khác đang chiếm cổng này.
                </p>
                <p>
                  <strong className="text-slate-200">Cách xử lý:</strong> Tắt các app/cửa sổ trùng lặp hoặc đổi sang Port khác trong tab Endpoints.
                </p>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2">
              <button
                onClick={() => setBindError(null)}
                className="px-4 py-2 rounded-xl bg-rose-600 hover:bg-rose-500 text-xs font-bold text-white transition shadow-lg shadow-rose-900/40"
              >
                Đã hiểu & Đóng
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
