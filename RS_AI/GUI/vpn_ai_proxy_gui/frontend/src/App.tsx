import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Shield,
  Activity,
  Route,
  Fingerprint,
  Trash2,
  CheckCircle2,
  AlertTriangle,
  Radio,
  Plus,
  Network,
  Zap,
  Check,
} from "lucide-react";
import { GatewayConfig, OutboundTunnel, RouteRule, RequestLog, TunnelTestResult } from "./types";

export function App() {
  const [activeTab, setActiveTab] = useState<"logs" | "routes" | "tunnels" | "fingerprint">("routes");
  const [config, setConfig] = useState<GatewayConfig | null>(null);
  const [logs, setLogs] = useState<RequestLog[]>([]);
  const [selectedLog, setSelectedLog] = useState<RequestLog | null>(null);
  const [testingTunnelId, setTestingTunnelId] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, TunnelTestResult>>({});

  // Tunnel modal/edit state
  const [editingTunnel, setEditingTunnel] = useState<OutboundTunnel | null>(null);
  // Route modal/edit state
  const [editingRoute, setEditingRoute] = useState<RouteRule | null>(null);

  const fetchConfig = async () => {
    try {
      const cfg = await invoke<GatewayConfig>("get_config");
      setConfig(cfg);
    } catch (e) {
      console.error(e);
    }
  };

  const fetchLogs = async () => {
    try {
      const logList = await invoke<RequestLog[]>("get_logs");
      setLogs(logList.reverse());
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    fetchConfig();
    fetchLogs();
    const interval = setInterval(() => {
      fetchLogs();
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleSaveConfig = async (newConfig: GatewayConfig) => {
    try {
      await invoke("save_config", { newConfig });
      setConfig(newConfig);
    } catch (e) {
      console.error(e);
    }
  };

  const handleTestTunnel = async (tunnel: OutboundTunnel) => {
    setTestingTunnelId(tunnel.id);
    try {
      const res = await invoke<TunnelTestResult>("test_single_tunnel", { tunnel });
      setTestResults((prev) => ({ ...prev, [tunnel.id]: res }));
    } catch (err) {
      console.error(err);
    } finally {
      setTestingTunnelId(null);
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

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[#090d16] text-slate-100 font-sans">
      {/* SIDEBAR */}
      <aside className="w-64 border-r border-slate-800 bg-[#0c1220] flex flex-col justify-between select-none">
        <div>
          {/* Logo & Header */}
          <div className="p-4 border-b border-slate-800 flex items-center gap-3">
            <div className="w-9 h-9 rounded-xl bg-gradient-to-tr from-cyan-500 via-indigo-600 to-purple-600 flex items-center justify-center shadow-lg shadow-cyan-500/20">
              <Shield className="w-5 h-5 text-white" />
            </div>
            <div>
              <h1 className="text-sm font-bold tracking-wide text-white">Mesh AI Gateway</h1>
              <p className="text-[11px] text-slate-400">Multi-VPN & Custom Targets</p>
            </div>
          </div>

          {/* Quick Metrics */}
          <div className="p-3 m-3 rounded-xl bg-slate-900/90 border border-slate-800 space-y-1.5 text-xs">
            <div className="flex items-center justify-between">
              <span className="text-slate-400">Active Tunnels:</span>
              <span className="font-mono text-cyan-400 font-bold">
                {config?.tunnels.filter((t) => t.enabled).length || 0} / {config?.tunnels.length || 0}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-slate-400">Port Endpoints:</span>
              <span className="font-mono text-indigo-400 font-bold">
                {new Set(config?.routes.filter((r) => r.enabled).map((r) => r.port)).size} ports
              </span>
            </div>
          </div>

          {/* Navigation */}
          <nav className="px-3 space-y-1">
            <button
              onClick={() => setActiveTab("routes")}
              className={`w-full flex items-center justify-between px-3 py-2.5 rounded-lg text-xs font-medium transition-all ${
                activeTab === "routes"
                  ? "bg-cyan-500/15 text-cyan-400 border border-cyan-500/30"
                  : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/50"
              }`}
            >
              <div className="flex items-center gap-2.5">
                <Route className="w-4 h-4" />
                <span>Ports & Routes</span>
              </div>
              <span className="px-1.5 py-0.5 rounded-full text-[10px] bg-slate-800 text-slate-300 font-mono">
                {config?.routes.length || 0}
              </span>
            </button>

            <button
              onClick={() => setActiveTab("tunnels")}
              className={`w-full flex items-center justify-between px-3 py-2.5 rounded-lg text-xs font-medium transition-all ${
                activeTab === "tunnels"
                  ? "bg-cyan-500/15 text-cyan-400 border border-cyan-500/30"
                  : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/50"
              }`}
            >
              <div className="flex items-center gap-2.5">
                <Network className="w-4 h-4" />
                <span>VPN & Tunnels Pool</span>
              </div>
              <span className="px-1.5 py-0.5 rounded-full text-[10px] bg-slate-800 text-slate-300 font-mono">
                {config?.tunnels.length || 0}
              </span>
            </button>

            <button
              onClick={() => setActiveTab("fingerprint")}
              className={`w-full flex items-center justify-between px-3 py-2.5 rounded-lg text-xs font-medium transition-all ${
                activeTab === "fingerprint"
                  ? "bg-cyan-500/15 text-cyan-400 border border-cyan-500/30"
                  : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/50"
              }`}
            >
              <div className="flex items-center gap-2.5">
                <Fingerprint className="w-4 h-4" />
                <span>Cloaking & Sanitizer</span>
              </div>
              <span className="text-[10px] text-purple-400 uppercase font-mono font-bold">Stealth</span>
            </button>

            <button
              onClick={() => setActiveTab("logs")}
              className={`w-full flex items-center justify-between px-3 py-2.5 rounded-lg text-xs font-medium transition-all ${
                activeTab === "logs"
                  ? "bg-cyan-500/15 text-cyan-400 border border-cyan-500/30"
                  : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/50"
              }`}
            >
              <div className="flex items-center gap-2.5">
                <Activity className="w-4 h-4" />
                <span>Leak Inspector & Logs</span>
              </div>
              <span className="px-1.5 py-0.5 rounded-full text-[10px] bg-slate-800 text-slate-300 font-mono">
                {logs.length}
              </span>
            </button>
          </nav>
        </div>

        {/* Footer info */}
        <div className="p-3 border-t border-slate-800 text-[11px] text-slate-500 font-mono">
          Memory Ring-Buffer: {config?.max_log_entries || 500} entries max
        </div>
      </aside>

      {/* MAIN CONTENT AREA */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* ROUTES & PORTS TAB */}
        {activeTab === "routes" && config && (
          <div className="p-6 overflow-y-auto max-w-5xl space-y-6">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-base font-bold text-slate-100">Gateway Port & Route Bindings</h2>
                <p className="text-xs text-slate-400 mt-1">
                  Bind specific ports and paths to custom targets (e.g. OpenAI, Anthropic, or custom mirrors like https://abc.xyz/v1) routed via designated VPN tunnels.
                </p>
              </div>
              <button
                onClick={() =>
                  setEditingRoute({
                    id: `route_${Date.now()}`,
                    name: "New Route",
                    port: 3000,
                    path_prefix: "/v1",
                    target_base_url: "https://api.openai.com/v1",
                    tunnel_id: config.tunnels[0]?.id || "direct_bypass",
                    enabled: true,
                    strip_prefix: true,
                  })
                }
                className="px-3 py-1.5 rounded-lg bg-cyan-600 hover:bg-cyan-500 text-xs font-semibold text-white flex items-center gap-1.5 transition"
              >
                <Plus className="w-4 h-4" />
                Add Port / Route
              </button>
            </div>

            {/* List Routes */}
            <div className="space-y-3">
              {config.routes.map((route) => {
                const assignedTunnel = config.tunnels.find((t) => t.id === route.tunnel_id);
                return (
                  <div
                    key={route.id}
                    className="p-4 rounded-xl bg-slate-900/70 border border-slate-800 flex flex-col md:flex-row items-start md:items-center justify-between gap-4"
                  >
                    <div className="space-y-1 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-bold text-white">{route.name}</span>
                        <span className="px-2 py-0.5 rounded bg-cyan-500/20 text-cyan-400 font-mono text-[11px] font-bold">
                          Port {route.port}
                        </span>
                        <span className="px-2 py-0.5 rounded bg-slate-800 text-slate-300 font-mono text-[11px]">
                          {route.path_prefix}
                        </span>
                        <span
                          className={`text-[10px] px-1.5 py-0.5 rounded font-mono ${
                            route.enabled
                              ? "bg-emerald-500/20 text-emerald-400"
                              : "bg-slate-800 text-slate-500"
                          }`}
                        >
                          {route.enabled ? "Active" : "Disabled"}
                        </span>
                      </div>
                      <div className="text-xs text-slate-400 flex items-center gap-2">
                        <span>Target:</span>
                        <span className="font-mono text-slate-300 bg-slate-950 px-2 py-0.5 rounded border border-slate-800 truncate max-w-sm">
                          {route.target_base_url}
                        </span>
                      </div>
                      <div className="text-[11px] text-slate-500 flex items-center gap-2">
                        <span>Tunnel Outbound:</span>
                        <span className="text-indigo-400 font-medium">
                          {assignedTunnel ? assignedTunnel.name : route.tunnel_id}
                        </span>
                      </div>
                    </div>

                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => setEditingRoute({ ...route })}
                        className="px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 transition"
                      >
                        Edit
                      </button>
                      <button
                        onClick={() => handleDeleteRoute(route.id)}
                        className="p-1.5 rounded hover:bg-rose-500/20 text-slate-400 hover:text-rose-400 transition"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* VPN TUNNELS POOL TAB */}
        {activeTab === "tunnels" && config && (
          <div className="p-6 overflow-y-auto max-w-5xl space-y-6">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-base font-bold text-slate-100">Outbound VPN & Proxy Pool</h2>
                <p className="text-xs text-slate-400 mt-1">
                  Register multiple local VPN tunnels (AdGuard SOCKS5, WireGuard SOCKS, HTTP proxies, or direct). Assign them individually to routes.
                </p>
              </div>
              <button
                onClick={() =>
                  setEditingTunnel({
                    id: `tunnel_${Date.now()}`,
                    name: "New Tunnel",
                    protocol: "socks5",
                    endpoint: "127.0.0.1:1081",
                    enabled: true,
                    tags: ["custom"],
                  })
                }
                className="px-3 py-1.5 rounded-lg bg-cyan-600 hover:bg-cyan-500 text-xs font-semibold text-white flex items-center gap-1.5 transition"
              >
                <Plus className="w-4 h-4" />
                Add Tunnel Node
              </button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {config.tunnels.map((tunnel) => {
                const testRes = testResults[tunnel.id];
                const isTesting = testingTunnelId === tunnel.id;

                return (
                  <div
                    key={tunnel.id}
                    className="p-4 rounded-xl bg-slate-900/80 border border-slate-800 space-y-3 flex flex-col justify-between"
                  >
                    <div>
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-bold text-slate-200">{tunnel.name}</span>
                        <span className="uppercase text-[10px] px-2 py-0.5 rounded font-mono font-bold bg-indigo-500/20 text-indigo-400">
                          {tunnel.protocol}
                        </span>
                      </div>
                      <div className="text-xs font-mono text-slate-400 mt-1">
                        Endpoint: {tunnel.endpoint || "(Direct/No proxy)"}
                      </div>

                      {/* Test Output */}
                      {testRes && (
                        <div className="mt-3 p-2 rounded bg-slate-950 border border-slate-800 text-[11px] font-mono space-y-0.5">
                          {testRes.success ? (
                            <>
                              <div className="text-emerald-400 flex items-center gap-1 font-semibold">
                                <Check className="w-3.5 h-3.5" />
                                Exit IP: {testRes.exit_ip}
                              </div>
                              <div className="text-slate-500">Latency: {testRes.latency_ms}ms</div>
                            </>
                          ) : (
                            <div className="text-rose-400">Error: {testRes.error}</div>
                          )}
                        </div>
                      )}
                    </div>

                    <div className="pt-2 border-t border-slate-800/80 flex items-center justify-between">
                      <button
                        disabled={isTesting}
                        onClick={() => handleTestTunnel(tunnel)}
                        className="px-2.5 py-1.5 rounded bg-cyan-600/20 hover:bg-cyan-600/30 text-cyan-300 text-xs font-semibold flex items-center gap-1 transition disabled:opacity-50"
                      >
                        <Zap className="w-3.5 h-3.5" />
                        {isTesting ? "Testing..." : "Test Exit IP"}
                      </button>

                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => setEditingTunnel({ ...tunnel })}
                          className="px-2.5 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-300 transition"
                        >
                          Edit
                        </button>
                        {tunnel.id !== "adguard_default" && tunnel.id !== "direct_bypass" && (
                          <button
                            onClick={() => handleDeleteTunnel(tunnel.id)}
                            className="p-1.5 rounded hover:bg-rose-500/20 text-slate-400 hover:text-rose-400 transition"
                          >
                            <Trash2 className="w-4 h-4" />
                          </button>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* FINGERPRINT CLOAKING TAB */}
        {activeTab === "fingerprint" && config && (
          <div className="p-6 overflow-y-auto max-w-4xl space-y-6">
            <div>
              <h2 className="text-base font-bold text-slate-100">Fingerprint Cloaking & Sanitizer</h2>
              <p className="text-xs text-slate-400 mt-1">
                Sanitize and disguise client metadata before outbound transmission via VPN.
              </p>
            </div>

            <div className="p-4 rounded-xl bg-slate-900 border border-slate-800 space-y-4">
              <div>
                <label className="text-xs font-semibold text-slate-300 block mb-1">
                  Rust Outbound User-Agent Spoofing
                </label>
                <input
                  type="text"
                  value={config.fingerprint_profile.custom_user_agent || ""}
                  onChange={(e) => {
                    const updated = {
                      ...config,
                      fingerprint_profile: {
                        ...config.fingerprint_profile,
                        custom_user_agent: e.target.value,
                      },
                    };
                    handleSaveConfig(updated);
                  }}
                  className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-2 font-mono text-xs text-slate-200"
                />
              </div>

              <div className="grid grid-cols-2 gap-3 pt-2">
                <label className="p-3 rounded-lg bg-slate-950 border border-slate-800 flex items-center justify-between text-xs cursor-pointer">
                  <div>
                    <div className="font-semibold text-slate-300">Strip SDK Headers</div>
                    <div className="text-[11px] text-slate-500">x-stainless-*, anthropic-client-*</div>
                  </div>
                  <input
                    type="checkbox"
                    checked={config.fingerprint_profile.strip_sdk_headers}
                    onChange={(e) => {
                      const updated = {
                        ...config,
                        fingerprint_profile: {
                          ...config.fingerprint_profile,
                          strip_sdk_headers: e.target.checked,
                        },
                      };
                      handleSaveConfig(updated);
                    }}
                    className="accent-cyan-500"
                  />
                </label>

                <label className="p-3 rounded-lg bg-slate-950 border border-slate-800 flex items-center justify-between text-xs cursor-pointer">
                  <div>
                    <div className="font-semibold text-slate-300">Strip IDE & Machine IDs</div>
                    <div className="text-[11px] text-slate-500">cursor, vscode, machine-id headers</div>
                  </div>
                  <input
                    type="checkbox"
                    checked={config.fingerprint_profile.strip_ide_headers}
                    onChange={(e) => {
                      const updated = {
                        ...config,
                        fingerprint_profile: {
                          ...config.fingerprint_profile,
                          strip_ide_headers: e.target.checked,
                        },
                      };
                      handleSaveConfig(updated);
                    }}
                    className="accent-cyan-500"
                  />
                </label>

                <label className="p-3 rounded-lg bg-slate-950 border border-slate-800 flex items-center justify-between text-xs cursor-pointer">
                  <div>
                    <div className="font-semibold text-slate-300">Strip Client Hints</div>
                    <div className="text-[11px] text-slate-500">sec-ch-ua platform & engine hints</div>
                  </div>
                  <input
                    type="checkbox"
                    checked={config.fingerprint_profile.strip_sec_ch_ua}
                    onChange={(e) => {
                      const updated = {
                        ...config,
                        fingerprint_profile: {
                          ...config.fingerprint_profile,
                          strip_sec_ch_ua: e.target.checked,
                        },
                      };
                      handleSaveConfig(updated);
                    }}
                    className="accent-cyan-500"
                  />
                </label>

                <label className="p-3 rounded-lg bg-slate-950 border border-slate-800 flex items-center justify-between text-xs cursor-pointer">
                  <div>
                    <div className="font-semibold text-slate-300">Mask Local Home Paths</div>
                    <div className="text-[11px] text-slate-500">Detect /home/username in payload prompt</div>
                  </div>
                  <input
                    type="checkbox"
                    checked={config.fingerprint_profile.mask_local_paths_in_body}
                    onChange={(e) => {
                      const updated = {
                        ...config,
                        fingerprint_profile: {
                          ...config.fingerprint_profile,
                          mask_local_paths_in_body: e.target.checked,
                        },
                      };
                      handleSaveConfig(updated);
                    }}
                    className="accent-cyan-500"
                  />
                </label>
              </div>
            </div>
          </div>
        )}

        {/* LOGS TAB */}
        {activeTab === "logs" && (
          <div className="flex-1 flex overflow-hidden">
            {/* List */}
            <div className="w-1/2 border-r border-slate-800 flex flex-col">
              <div className="h-12 border-b border-slate-800 px-4 flex items-center justify-between bg-slate-900/30">
                <div className="flex items-center gap-2 text-xs font-semibold uppercase text-slate-300">
                  <Activity className="w-4 h-4 text-cyan-400" />
                  Request Stream (Ring-Buffer)
                </div>
                <button
                  onClick={async () => {
                    await invoke("clear_logs");
                    setLogs([]);
                    setSelectedLog(null);
                  }}
                  className="p-1.5 rounded hover:bg-slate-800 text-slate-400 hover:text-rose-400 transition"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>

              <div className="flex-1 overflow-y-auto divide-y divide-slate-800/60">
                {logs.length === 0 ? (
                  <div className="h-full flex flex-col items-center justify-center text-slate-500 text-xs p-6 text-center space-y-2">
                    <Radio className="w-8 h-8 stroke-1 text-slate-600 animate-pulse" />
                    <div>No requests in buffer yet</div>
                  </div>
                ) : (
                  logs.map((log) => {
                    const isSelected = selectedLog?.id === log.id;
                    const hasLeak = log.leaked_findings.length > 0;
                    return (
                      <div
                        key={log.id}
                        onClick={() => setSelectedLog(log)}
                        className={`p-3 text-xs cursor-pointer transition flex items-center justify-between ${
                          isSelected ? "bg-cyan-500/10 border-l-2 border-cyan-400" : "hover:bg-slate-800/30"
                        }`}
                      >
                        <div className="space-y-1 flex-1 pr-2 truncate">
                          <div className="flex items-center gap-2">
                            <span className="px-1.5 py-0.5 rounded text-[10px] font-bold font-mono bg-blue-500/20 text-blue-400">
                              {log.method}
                            </span>
                            <span className="font-mono text-slate-300 truncate">{log.path}</span>
                            {log.is_streaming && (
                              <span className="px-1 text-[9px] bg-purple-500/20 text-purple-300 rounded font-mono">
                                SSE
                              </span>
                            )}
                          </div>
                          <div className="text-[11px] text-slate-500 truncate font-mono">↳ {log.target_url}</div>
                        </div>

                        <div className="flex flex-col items-end gap-1">
                          <div className="flex items-center gap-1.5">
                            {hasLeak && (
                              <span className="flex items-center gap-0.5 text-[10px] text-amber-400 bg-amber-500/10 px-1 rounded">
                                <AlertTriangle className="w-3 h-3" />
                                {log.leaked_findings.length}
                              </span>
                            )}
                            <span
                              className={`font-mono text-[11px] font-semibold ${
                                log.status_code >= 200 && log.status_code < 300
                                  ? "text-emerald-400"
                                  : "text-rose-400"
                              }`}
                            >
                              {log.status_code}
                            </span>
                          </div>
                          <span className="text-[10px] text-slate-500 font-mono">
                            {log.duration_ms}ms · {log.timestamp}
                          </span>
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </div>

            {/* Inspector */}
            <div className="w-1/2 flex flex-col bg-slate-950/60 overflow-y-auto">
              {selectedLog ? (
                <div className="p-4 space-y-4">
                  <div className="flex items-center justify-between pb-3 border-b border-slate-800">
                    <div>
                      <h3 className="text-sm font-semibold text-slate-200">Request Inspection</h3>
                      <p className="text-xs text-slate-500 font-mono">ID: {selectedLog.id}</p>
                    </div>
                    <span
                      className={`text-xs px-2 py-1 rounded font-mono font-bold ${
                        selectedLog.status_code === 200
                          ? "bg-emerald-500/20 text-emerald-400"
                          : "bg-rose-500/20 text-rose-400"
                      }`}
                    >
                      HTTP {selectedLog.status_code}
                    </span>
                  </div>

                  {/* Leaks */}
                  <div>
                    <h4 className="text-xs font-semibold uppercase text-slate-400 mb-2 flex items-center gap-1.5">
                      <Shield className="w-3.5 h-3.5 text-amber-400" />
                      Inbound Leak Findings ({selectedLog.leaked_findings.length})
                    </h4>
                    {selectedLog.leaked_findings.length === 0 ? (
                      <div className="p-2.5 rounded-lg bg-emerald-950/20 border border-emerald-800/40 text-xs text-emerald-400 flex items-center gap-2">
                        <CheckCircle2 className="w-4 h-4" />
                        No fingerprint or path leaks detected in this request.
                      </div>
                    ) : (
                      <div className="space-y-1.5">
                        {selectedLog.leaked_findings.map((f, idx) => (
                          <div
                            key={idx}
                            className="p-2 rounded bg-amber-950/20 border border-amber-800/40 text-xs space-y-0.5"
                          >
                            <div className="flex items-center justify-between font-semibold">
                              <span className="text-amber-300">{f.field}</span>
                              <span className="uppercase text-[9px] px-1 rounded bg-amber-500/30 text-amber-300">
                                {f.severity}
                              </span>
                            </div>
                            <div className="text-[11px] text-slate-400">{f.description}</div>
                            <div className="text-[10px] font-mono text-slate-500 bg-slate-900/80 p-1 rounded break-all">
                              {f.value}
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Headers */}
                  <div>
                    <h4 className="text-xs font-semibold uppercase text-slate-400 mb-2">Header Comparison</h4>
                    <div className="grid grid-cols-2 gap-2 text-[11px]">
                      <div className="p-2.5 rounded bg-slate-900 border border-slate-800 space-y-1">
                        <div className="font-semibold text-slate-400 border-b border-slate-800 pb-1">Raw Inbound</div>
                        <div className="space-y-1 max-h-40 overflow-y-auto font-mono text-[10px]">
                          {selectedLog.client_headers.map(([k, v], i) => (
                            <div key={i} className="truncate">
                              <span className="text-slate-500">{k}:</span>{" "}
                              <span className="text-slate-300">{v}</span>
                            </div>
                          ))}
                        </div>
                      </div>

                      <div className="p-2.5 rounded bg-slate-900 border border-slate-800 space-y-1">
                        <div className="font-semibold text-cyan-400 border-b border-slate-800 pb-1">
                          Forwarded Outbound
                        </div>
                        <div className="space-y-1 max-h-40 overflow-y-auto font-mono text-[10px]">
                          {selectedLog.forwarded_headers.map(([k, v], i) => (
                            <div key={i} className="truncate">
                              <span className="text-cyan-600">{k}:</span>{" "}
                              <span className="text-slate-300">{v}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Body Preview */}
                  <div>
                    <h4 className="text-xs font-semibold uppercase text-slate-400 mb-2">Payload Preview</h4>
                    <div className="p-2.5 rounded bg-slate-900 border border-slate-800 font-mono text-[11px] text-slate-300 whitespace-pre-wrap max-h-36 overflow-y-auto">
                      {selectedLog.prompt_preview || "<empty>"}
                    </div>
                  </div>
                </div>
              ) : (
                <div className="h-full flex items-center justify-center text-slate-500 text-xs">
                  Select a request to inspect details
                </div>
              )}
            </div>
          </div>
        )}

        {/* MODAL: EDIT ROUTE */}
        {editingRoute && (
          <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 z-50">
            <div className="bg-slate-900 border border-slate-800 rounded-2xl w-full max-w-lg p-5 space-y-4 shadow-2xl">
              <h3 className="text-sm font-bold text-white">Configure Route & Port</h3>
              <div className="space-y-3 text-xs">
                <div>
                  <label className="text-slate-400 block mb-1">Route Name</label>
                  <input
                    type="text"
                    value={editingRoute.name}
                    onChange={(e) => setEditingRoute({ ...editingRoute, name: e.target.value })}
                    className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                  />
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="text-slate-400 block mb-1">Listening Port</label>
                    <input
                      type="number"
                      value={editingRoute.port}
                      onChange={(e) => setEditingRoute({ ...editingRoute, port: parseInt(e.target.value) || 3000 })}
                      className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                    />
                  </div>
                  <div>
                    <label className="text-slate-400 block mb-1">Path Prefix (e.g. /v1 or /)</label>
                    <input
                      type="text"
                      value={editingRoute.path_prefix}
                      onChange={(e) => setEditingRoute({ ...editingRoute, path_prefix: e.target.value })}
                      className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                    />
                  </div>
                </div>

                <div>
                  <label className="text-slate-400 block mb-1">Target Base URL (e.g. https://abc.xyz/v1)</label>
                  <input
                    type="text"
                    value={editingRoute.target_base_url}
                    onChange={(e) => setEditingRoute({ ...editingRoute, target_base_url: e.target.value })}
                    className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                  />
                </div>

                <div>
                  <label className="text-slate-400 block mb-1">Assigned VPN Tunnel</label>
                  <select
                    value={editingRoute.tunnel_id}
                    onChange={(e) => setEditingRoute({ ...editingRoute, tunnel_id: e.target.value })}
                    className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200"
                  >
                    {config?.tunnels.map((t) => (
                      <option key={t.id} value={t.id}>
                        {t.name} ({t.protocol}: {t.endpoint || "Direct"})
                      </option>
                    ))}
                  </select>
                </div>

                <div className="flex items-center gap-4 pt-2">
                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={editingRoute.strip_prefix}
                      onChange={(e) => setEditingRoute({ ...editingRoute, strip_prefix: e.target.checked })}
                      className="accent-cyan-500"
                    />
                    <span>Strip path prefix before forward</span>
                  </label>
                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={editingRoute.enabled}
                      onChange={(e) => setEditingRoute({ ...editingRoute, enabled: e.target.checked })}
                      className="accent-cyan-500"
                    />
                    <span>Enabled</span>
                  </label>
                </div>
              </div>

              <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-800">
                <button
                  onClick={() => setEditingRoute(null)}
                  className="px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-300"
                >
                  Cancel
                </button>
                <button
                  onClick={handleAddOrUpdateRoute}
                  className="px-3 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-xs font-semibold text-white"
                >
                  Save Route
                </button>
              </div>
            </div>
          </div>
        )}

        {/* MODAL: EDIT TUNNEL */}
        {editingTunnel && (
          <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 z-50">
            <div className="bg-slate-900 border border-slate-800 rounded-2xl w-full max-w-lg p-5 space-y-4 shadow-2xl">
              <h3 className="text-sm font-bold text-white">Configure Outbound VPN Tunnel</h3>
              <div className="space-y-3 text-xs">
                <div>
                  <label className="text-slate-400 block mb-1">Tunnel Name</label>
                  <input
                    type="text"
                    value={editingTunnel.name}
                    onChange={(e) => setEditingTunnel({ ...editingTunnel, name: e.target.value })}
                    className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                  />
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="text-slate-400 block mb-1">Protocol</label>
                    <select
                      value={editingTunnel.protocol}
                      onChange={(e) =>
                        setEditingTunnel({ ...editingTunnel, protocol: e.target.value as any })
                      }
                      className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200"
                    >
                      <option value="socks5">SOCKS5 (AdGuard / Clash / SSH)</option>
                      <option value="http">HTTP Proxy</option>
                      <option value="direct">Direct (Bypass VPN)</option>
                    </select>
                  </div>
                  <div>
                    <label className="text-slate-400 block mb-1">Endpoint (Host:Port)</label>
                    <input
                      type="text"
                      disabled={editingTunnel.protocol === "direct"}
                      value={editingTunnel.endpoint}
                      onChange={(e) => setEditingTunnel({ ...editingTunnel, endpoint: e.target.value })}
                      placeholder="127.0.0.1:1080"
                      className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono disabled:opacity-40"
                    />
                  </div>
                </div>

                <label className="flex items-center gap-2 cursor-pointer pt-2">
                  <input
                    type="checkbox"
                    checked={editingTunnel.enabled}
                    onChange={(e) => setEditingTunnel({ ...editingTunnel, enabled: e.target.checked })}
                    className="accent-cyan-500"
                  />
                  <span>Enabled</span>
                </label>
              </div>

              <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-800">
                <button
                  onClick={() => setEditingTunnel(null)}
                  className="px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-300"
                >
                  Cancel
                </button>
                <button
                  onClick={handleAddOrUpdateTunnel}
                  className="px-3 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-xs font-semibold text-white"
                >
                  Save Tunnel
                </button>
              </div>
            </div>
          </div>
        )}
      </main>
    </div>
  );
}
