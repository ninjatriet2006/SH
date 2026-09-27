import { useState } from "react";
import { OutboundTunnel, RouteRule } from "./types";
import { Sidebar } from "./components/Sidebar";
import { TrafficTab } from "./components/TrafficTab";
import { RoutesTab } from "./components/RoutesTab";
import { TunnelsTab } from "./components/TunnelsTab";
import { FingerprintTab } from "./components/FingerprintTab";
import { SettingsTab } from "./components/SettingsTab";
import { Modals } from "./components/Modals";
import { PortConflictModal } from "./components/PortConflictModal";
import { useConfig } from "./hooks/useConfig";
import { useTraffic } from "./hooks/useTraffic";
import { useTunnels } from "./hooks/useTunnels";

export function App() {
  const [activeTab, setActiveTab] = useState<"traffic" | "routes" | "tunnels" | "fingerprint" | "settings">("traffic");

  const {
    config,
    bindError,
    setBindError,
    fetchConfig,
    saveConfig,
    addOrUpdateRoute,
    deleteRoute,
    toggleRoute,
    advanceKey,
  } = useConfig();

  const {
    trafficLogs,
    selectedLog,
    setSelectedLog,
    clearLogs,
  } = useTraffic(activeTab === "traffic");

  const {
    activeCliTunnelId,
    tunnelEvents,
    loginUrls,
    testTunnel,
    toggleTunnel,
    forceStopTunnel,
    addOrUpdateTunnel,
    deleteTunnel,
    loginTunnel,
    cancelLogin,
    openLoginUrl,
  } = useTunnels(fetchConfig);

  const [editingTunnel, setEditingTunnel] = useState<OutboundTunnel | null>(null);
  const [editingRoute, setEditingRoute] = useState<RouteRule | null>(null);

  const handleSaveRoute = async () => {
    if (!editingRoute) return;
    try {
      await addOrUpdateRoute(editingRoute);
      setEditingRoute(null);
    } catch (e) {
      console.error(e);
    }
  };

  const handleSaveTunnel = async () => {
    if (!editingTunnel) return;
    try {
      await addOrUpdateTunnel(editingTunnel);
      setEditingTunnel(null);
    } catch (e) {
      console.error(e);
    }
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
            onClearLogs={clearLogs}
            maxLogEntries={config?.max_log_entries || 500}
            onUpdateMaxLogEntries={async (count) => {
              if (config) {
                await saveConfig({ ...config, max_log_entries: count });
              }
            }}
          />
        )}

        {activeTab === "routes" && config && (
          <RoutesTab
            config={config}
            onToggleRoute={toggleRoute}
            onEditRoute={(r) => setEditingRoute({ ...r })}
            onSaveRouteDirect={addOrUpdateRoute}
            onDeleteRoute={deleteRoute}
            onAdvanceKey={advanceKey}
            onCreateRoute={() =>
              setEditingRoute({
                id: `route_${Date.now()}`,
                name: "Custom Endpoint",
                port: 3000,
                path_prefix: "/v1",
                target_base_url: "https://abc.xyz/v1",
                tunnel_id: config.tunnels[0]?.id || "direct_bypass",
                enabled: true,
                fingerprint_index: null,
                protocol_adapter: "none",
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
            tunnelEvents={tunnelEvents}
            loginUrls={loginUrls}
            onTestTunnel={testTunnel}
            onToggleTunnel={toggleTunnel}
            onForceStop={forceStopTunnel}
            onLoginTunnel={loginTunnel}
            onCancelLogin={cancelLogin}
            onOpenLoginUrl={openLoginUrl}
            onEditTunnel={(t) => setEditingTunnel({ ...t })}
            onDeleteTunnel={deleteTunnel}
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
            onSaveConfig={saveConfig}
          />
        )}

        {activeTab === "settings" && config && (
          <SettingsTab
            config={config}
            onSaveConfig={saveConfig}
          />
        )}
      </main>

      <Modals
        editingRoute={editingRoute}
        setEditingRoute={setEditingRoute}
        onSaveRoute={handleSaveRoute}
        editingTunnel={editingTunnel}
        setEditingTunnel={setEditingTunnel}
        onSaveTunnel={handleSaveTunnel}
        config={config}
      />

      <PortConflictModal
        bindError={bindError}
        onClose={() => setBindError(null)}
      />
    </div>
  );
}
