import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  GatewayConfig,
  OutboundTunnel,
  RouteRule,
  RawTrafficLog,
  TunnelTestResult,
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
  const [testingTunnelId, setTestingTunnelId] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, TunnelTestResult>>({});

  const [editingTunnel, setEditingTunnel] = useState<OutboundTunnel | null>(null);
  const [editingRoute, setEditingRoute] = useState<RouteRule | null>(null);

  const fetchConfig = useCallback(async () => {
    try {
      const cfg = await invoke<GatewayConfig>("get_config");
      setConfig(cfg);
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
    const interval = setInterval(() => {
      fetchTraffic();
    }, 1500);
    return () => clearInterval(interval);
  }, [fetchConfig, fetchTraffic]);

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

  const handleTestTunnel = async (tunnel: OutboundTunnel) => {
    setTestingTunnelId(tunnel.id);
    try {
      const res = await invoke<TunnelTestResult>("test_single_tunnel", { tunnel });
      setTestResults((prev) => ({ ...prev, [tunnel.id]: res }));
      await fetchConfig();
    } catch (err) {
      console.error(err);
    } finally {
      setTestingTunnelId(null);
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

  const handleToggleTunnel = async (tunnelId: string, enabled: boolean) => {
    try {
      await invoke("toggle_tunnel", { tunnelId, enabled });
      await fetchConfig();
    } catch (e) {
      console.error(e);
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
            onEditRoute={(r) => setEditingRoute({ ...r })}
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
            testResults={testResults}
            testingTunnelId={testingTunnelId}
            onTestTunnel={handleTestTunnel}
            onToggleTunnel={handleToggleTunnel}
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
    </div>
  );
}
