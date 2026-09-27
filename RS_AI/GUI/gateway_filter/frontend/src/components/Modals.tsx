import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GatewayConfig, RouteRule, OutboundTunnel, AdguardLocationItem, ProtocolAdapter } from "../types";

function portFromEndpoint(ep: string): string {
  // Bóc userinfo trước để user:1234@host:1080 không match nhầm port 1234
  const noAuth = ep.includes("@") ? ep.slice(ep.lastIndexOf("@") + 1) : ep;
  const match = noAuth.replace(/^(socks5h?|http|https):\/\//, "").match(/:(\d+)/);
  return match ? match[1] : "";
}

function adguardTemplate(port: string, location?: string) {
  const p = port || "1080";
  let connectFlag = "";
  if (location === "fastest") {
    connectFlag = " -f";
  } else if (location && location !== "random") {
    connectFlag = ` -l "${location}"`;
  }
  return {
    start_command: `adguardvpn-cli config set-socks-port ${p} && adguardvpn-cli config set-mode socks && adguardvpn-cli connect${connectFlag}`,
    stop_command: "adguardvpn-cli disconnect",
  };
}

function warpTemplate(port: string) {
  const p = port || "1081";
  return {
    start_command: `warp-cli mode proxy && warp-cli proxy port ${p} && warp-cli connect`,
    stop_command: "warp-cli disconnect",
  };
}

type CliPreset = "adguard" | "warp" | "custom";
type TunnelMode = "app" | "proxy";
type VpnApp = "adguard" | "warp" | "direct";

/// Đoán preset từ commands hiện tại (so với template theo port của endpoint).
/// Không khớp preset nào → "custom" (cho sửa tay).
function detectPreset(t: OutboundTunnel): CliPreset {
  const port = portFromEndpoint(t.endpoint || "");
  const p = port || "1080";
  if (
    t.start_command?.startsWith(`adguardvpn-cli config set-socks-port ${p}`) &&
    t.stop_command === "adguardvpn-cli disconnect"
  ) {
    return "adguard";
  }
  const w = warpTemplate(port);
  if (t.start_command === w.start_command && t.stop_command === w.stop_command) {
    return "warp";
  }
  return "custom";
}

/// Chế độ tunnel suy từ dữ liệu (không lưu backend, khỏi migration):
/// preset AdGuard/WARP hoặc direct-trần (không lệnh) → "app", còn lại "proxy".
function detectMode(t: OutboundTunnel): TunnelMode {
  if (detectPreset(t) !== "custom") return "app";
  if (t.protocol === "direct" && !t.start_command && !t.stop_command) return "app";
  return "proxy";
}

function detectApp(t: OutboundTunnel): VpnApp {
  const p = detectPreset(t);
  if (p === "adguard" || p === "warp") return p;
  return "direct";
}

/// Áp app được chọn: endpoint/protocol/lệnh/auth tự điền cố định.
function applyApp(t: OutboundTunnel, app: VpnApp): OutboundTunnel {
  const port = portFromEndpoint(t.endpoint || "");
  if (app === "adguard") {
    const loc = t.adguard_location || "random";
    return {
      ...t,
      protocol: "socks5",
      endpoint: `127.0.0.1:${port || "1080"}`,
      auth_user: "",
      auth_pass: "",
      adguard_location: loc,
      ...adguardTemplate(port, loc),
    };
  }
  if (app === "warp") {
    return {
      ...t,
      protocol: "socks5",
      endpoint: `127.0.0.1:${port || "1081"}`,
      auth_user: "",
      auth_pass: "",
      ...warpTemplate(port),
    };
  }
  return {
    ...t,
    protocol: "direct",
    endpoint: "",
    auth_user: "",
    auth_pass: "",
    start_command: "",
    stop_command: "",
  };
}

interface ModalsProps {
  editingRoute: RouteRule | null;
  setEditingRoute: (route: RouteRule | null) => void;
  onSaveRoute: () => Promise<void>;
  editingTunnel: OutboundTunnel | null;
  setEditingTunnel: (tunnel: OutboundTunnel | null) => void;
  onSaveTunnel: () => Promise<void>;
  config: GatewayConfig | null;
}

export function Modals({
  editingRoute,
  setEditingRoute,
  onSaveRoute,
  editingTunnel,
  setEditingTunnel,
  onSaveTunnel,
  config,
}: ModalsProps) {
  // Mode/app của tunnel đang mở (reset mỗi khi đổi tunnel — pattern adjust-during-render)
  const [prevTunnelId, setPrevTunnelId] = useState<string | null>(null);
  const [tunnelMode, setTunnelMode] = useState<TunnelMode>("app");
  const [tunnelApp, setTunnelApp] = useState<VpnApp>("adguard");
  const [adguardLocations, setAdguardLocations] = useState<AdguardLocationItem[]>([]);
  const [loadingLocations, setLoadingLocations] = useState<boolean>(false);

  const loadAdguardLocations = async () => {
    setLoadingLocations(true);
    try {
      const list = await invoke<AdguardLocationItem[]>("get_adguard_locations");
      setAdguardLocations(list || []);
    } catch (err) {
      console.error("Failed to load adguard locations:", err);
    } finally {
      setLoadingLocations(false);
    }
  };

  if ((editingTunnel?.id ?? null) !== prevTunnelId) {
    setPrevTunnelId(editingTunnel?.id ?? null);
    if (editingTunnel) {
      setTunnelMode(detectMode(editingTunnel));
      setTunnelApp(detectApp(editingTunnel));
      if (detectApp(editingTunnel) === "adguard" && adguardLocations.length === 0) {
        loadAdguardLocations();
      }
    }
  }
  return (
    <>
      {/* MODAL: EDIT ROUTE */}
      {editingRoute && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 z-50">
          <div className="bg-slate-900 border border-slate-800 rounded-2xl w-full max-w-lg p-5 space-y-4 shadow-2xl">
            <h3 className="text-sm font-bold text-white">Configure Endpoint & Dedicated Key File</h3>
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
                <label className="text-slate-400 block mb-1">Min Request Interval ms (0 = off, riêng endpoint này)</label>
                <input
                  type="number"
                  min="0"
                  step="50"
                  value={editingRoute.min_request_interval_ms ?? 0}
                  onChange={(e) =>
                    setEditingRoute({
                      ...editingRoute,
                      min_request_interval_ms: Math.max(0, parseInt(e.target.value) || 0),
                    })
                  }
                  className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                />
              </div>

              <div>
                <label className="text-slate-400 block mb-1">Assigned Outbound Tunnel</label>
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

              <div>
                <label className="text-slate-400 block mb-1">Fingerprint Profile (per-endpoint)</label>
                <select
                  value={editingRoute.fingerprint_index ?? "global"}
                  onChange={(e) =>
                    setEditingRoute({
                      ...editingRoute,
                      fingerprint_index:
                        e.target.value === "global" ? null : parseInt(e.target.value),
                    })
                  }
                  className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200"
                >
                  <option value="global">Global default (follows pool active ★)</option>
                  {(config?.fingerprint_pool ?? []).map((p, i) => (
                    <option key={i} value={i}>
                      #{i + 1} — {p.mode}
                      {i === (config?.active_fingerprint_index ?? 0) ? " ★" : ""}
                    </option>
                  ))}
                </select>
                <p className="text-[10px] text-slate-500 mt-1">
                  Pinned endpoints rotate only their own profile on 401/403; Global default follows the pool active.
                </p>
              </div>

              <div>
                <label className="text-slate-400 block mb-1">Protocol Adapter</label>
                <select
                  value={editingRoute.protocol_adapter ?? "none"}
                  onChange={(e) =>
                    setEditingRoute({
                      ...editingRoute,
                      protocol_adapter: e.target.value as ProtocolAdapter,
                    })
                  }
                  className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200"
                >
                  <option value="none">None (Direct Passthrough)</option>
                  <option value="openai_to_1min">OpenAI Completion ↔ 1min.AI</option>
                  <option value="openai_to_anthropic">OpenAI Completion ↔ Anthropic Messages</option>
                </select>
                <p className="text-[10px] text-slate-500 mt-1">
                  Chuyển đổi giao thức: Cho phép client gọi OpenAI Chat Completion nhưng Gateway tự dịch sang Provider đích (bao gồm cả SSE streaming và Models list).
                </p>
              </div>

              <div className="flex items-center gap-4 pt-2">
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
                onClick={onSaveRoute}
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

            {/* ON / OFF config toggle (prominent, top of modal) */}
            <div
              className="p-3 rounded-lg bg-slate-950 border border-slate-800 flex items-center justify-between cursor-pointer"
              title="ON/OFF chỉ mang ý nghĩa cho phép Tunnel tiếp nhận & chuyển tiếp Traffic. Nó KHÔNG đồng nghĩa tiến trình phần mềm VPN bên dưới đang thực sự chạy (trạng thái Active)."
            >
              <div>
                <div className="text-xs font-bold text-slate-200">Tunnel Status</div>
                <div className="text-[10px] text-slate-500">
                  {editingTunnel.enabled
                    ? "ON — Allow this tunnel to route traffic"
                    : "OFF — Pause traffic routing through this tunnel"}
                </div>
              </div>
              <button
                type="button"
                onClick={() => setEditingTunnel({ ...editingTunnel, enabled: !editingTunnel.enabled })}
                className={`relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
                  editingTunnel.enabled ? "bg-emerald-500" : "bg-slate-700"
                }`}
              >
                <span
                  className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                    editingTunnel.enabled ? "translate-x-4" : "translate-x-0"
                  }`}
                />
              </button>
            </div>

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

              <div>
                <label className="text-slate-400 block mb-1">Chế độ tunnel</label>
                <select
                  value={tunnelMode}
                  onChange={(e) => {
                    const mode = e.target.value as TunnelMode;
                    setTunnelMode(mode);
                    // Vào nhánh app thì áp app hiện tại ngay để form nhất quán
                    if (mode === "app" && editingTunnel) {
                      setEditingTunnel(applyApp(editingTunnel, tunnelApp));
                    }
                  }}
                  className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200"
                >
                  <option value="app">VPN App (chọn app, còn lại tự động)</option>
                  <option value="proxy">Proxy thủ công (nhập thông số)</option>
                </select>
              </div>

              {tunnelMode === "app" ? (
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="text-slate-400 block mb-1">Ứng dụng VPN</label>
                    <select
                      value={tunnelApp}
                      onChange={(e) => {
                        const app = e.target.value as VpnApp;
                        setTunnelApp(app);
                        setEditingTunnel(applyApp(editingTunnel, app));
                      }}
                      className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200"
                    >
                      <option value="adguard">AdGuard</option>
                      <option value="warp">WARP</option>
                      <option value="direct">Direct (không app)</option>
                    </select>
                  </div>
                  <div>
                    <label className="text-slate-400 block mb-1">Cổng SOCKS (Host:Port)</label>
                    <input
                      type="text"
                      disabled={tunnelApp === "direct"}
                      value={editingTunnel.endpoint}
                      onChange={(e) => {
                        const endpoint = e.target.value;
                        const base = { ...editingTunnel, endpoint };
                        // App adguard/warp: dựng lại lệnh theo port mới để không lệch cổng
                        if (tunnelApp === "adguard") {
                          setEditingTunnel({
                            ...base,
                            ...adguardTemplate(portFromEndpoint(endpoint), editingTunnel.adguard_location),
                          });
                        } else if (tunnelApp === "warp") {
                          setEditingTunnel({ ...base, ...warpTemplate(portFromEndpoint(endpoint)) });
                        } else {
                          setEditingTunnel(base);
                        }
                      }}
                      placeholder="127.0.0.1:1080"
                      className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono disabled:opacity-40"
                    />
                  </div>

                  {tunnelApp === "adguard" && (
                    <div className="col-span-2 mt-1">
                      <div className="flex items-center justify-between mb-1">
                        <label className="text-slate-400">Vị Trí AdGuard VPN (Location)</label>
                        <button
                          type="button"
                          onClick={loadAdguardLocations}
                          disabled={loadingLocations}
                          className="text-[11px] text-sky-400 hover:text-sky-300 flex items-center gap-1 disabled:opacity-50"
                        >
                          {loadingLocations ? "Đang tải vị trí..." : "🔄 Cập nhật danh sách từ CLI"}
                        </button>
                      </div>
                      <select
                        value={editingTunnel.adguard_location || "random"}
                        onFocus={() => {
                          if (adguardLocations.length === 0 && !loadingLocations) {
                            loadAdguardLocations();
                          }
                        }}
                        onChange={(e) => {
                          const loc = e.target.value;
                          const port = portFromEndpoint(editingTunnel.endpoint || "");
                          setEditingTunnel({
                            ...editingTunnel,
                            adguard_location: loc,
                            ...adguardTemplate(port, loc),
                          });
                        }}
                        className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200"
                      >
                        <option value="random">🎲 Random Location (Mặc định - Đổi IP liên tục)</option>
                        <option value="fastest">⚡ Fastest Location (Nhanh nhất - Tối ưu Ping)</option>
                        {adguardLocations.length > 0 && (
                          <optgroup label="Danh Sách Vị Trí Khả Dụng (Từ adguardvpn-cli)">
                            {adguardLocations.map((item) => (
                              <option key={item.id} value={item.id}>
                                📍 {item.name} {item.ping_ms ? `(${item.ping_ms} ms)` : ""}
                              </option>
                            ))}
                          </optgroup>
                        )}
                      </select>
                      <p className="text-[10px] text-slate-500 mt-1">
                        Chế độ "Random" tự động lấy vị trí khả dụng từ AdGuard mỗi lần kết nối để chống nhận diện và cố định IP.
                      </p>
                    </div>
                  )}
                </div>
              ) : (
                <>
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
                        <option value="socks5">SOCKS5</option>
                        <option value="socks5h">SOCKS5H</option>
                        <option value="http">HTTP</option>
                        <option value="https">HTTPS</option>
                        <option value="direct">Direct</option>
                      </select>
                    </div>
                    <div>
                      <label className="text-slate-400 block mb-1">Endpoint (Host:Port)</label>
                      <input
                        type="text"
                        disabled={editingTunnel.protocol === "direct"}
                        value={editingTunnel.endpoint}
                        onChange={(e) => setEditingTunnel({ ...editingTunnel, endpoint: e.target.value })}
                        placeholder="127.0.0.1:1080 hoặc socks5h://user:pass@host:port"
                        className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono disabled:opacity-40"
                      />
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="text-slate-400 block mb-1">Proxy User (nếu có)</label>
                      <input
                        type="text"
                        value={editingTunnel.auth_user || ""}
                        onChange={(e) => setEditingTunnel({ ...editingTunnel, auth_user: e.target.value })}
                        placeholder="username"
                        autoComplete="off"
                        className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                      />
                    </div>
                    <div>
                      <label className="text-slate-400 block mb-1">Proxy Pass (nếu có)</label>
                      <input
                        type="password"
                        value={editingTunnel.auth_pass || ""}
                        onChange={(e) => setEditingTunnel({ ...editingTunnel, auth_pass: e.target.value })}
                        placeholder="••••••"
                        autoComplete="new-password"
                        className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                      />
                    </div>
                  </div>
                </>
              )}

              <div>
                <label className="text-slate-400 block mb-1">Max Concurrent Streams (0 = Unlimited)</label>
                <input
                  type="number"
                  min="0"
                  value={editingTunnel.max_concurrent_streams ?? 0}
                  onChange={(e) =>
                    setEditingTunnel({
                      ...editingTunnel,
                      max_concurrent_streams: Math.max(0, parseInt(e.target.value) || 0),
                    })
                  }
                  className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                />
              </div>

              <div>
                <label className="text-slate-400 block mb-1">Min Request Interval ms (0 = Off, nhịp chung cả tunnel)</label>
                <input
                  type="number"
                  min="0"
                  step="50"
                  value={editingTunnel.min_request_interval_ms ?? 0}
                  onChange={(e) =>
                    setEditingTunnel({
                      ...editingTunnel,
                      min_request_interval_ms: Math.max(0, parseInt(e.target.value) || 0),
                    })
                  }
                  className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                />
              </div>

              {/* Lệnh CLI: nhánh app tự động hoàn toàn, nhánh proxy cho nhập tay */}
              {tunnelMode === "app" ? (
                <div className="p-2.5 rounded-lg bg-slate-950 border border-slate-800 text-[11px] text-slate-400 font-mono space-y-1">
                  <div>
                    Start: <span className="text-slate-200">{editingTunnel.start_command || "(Direct — không cần lệnh)"}</span>
                  </div>
                  <div>
                    Stop: <span className="text-slate-200">{editingTunnel.stop_command || "(Direct — không cần lệnh)"}</span>
                  </div>
                  <div className="text-slate-500">
                    Lệnh cố định theo app + cổng, login bằng nút Login CLI trên card tunnel.
                  </div>
                </div>
              ) : (
                <div className="space-y-2 pt-1 border-t border-slate-800/80">
                  <div className="text-slate-400 font-medium">Custom Start/Stop Commands (Proxy thủ công)</div>
                  <div>
                    <label className="text-slate-400 block mb-1">Start Command (CLI)</label>
                    <input
                      type="text"
                      value={editingTunnel.start_command || ""}
                      onChange={(e) => setEditingTunnel({ ...editingTunnel, start_command: e.target.value })}
                      placeholder="e.g. ssh -D 1080 user@host -N"
                      className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                    />
                  </div>

                  <div>
                    <label className="text-slate-400 block mb-1">Stop Command (CLI)</label>
                    <input
                      type="text"
                      value={editingTunnel.stop_command || ""}
                      onChange={(e) => setEditingTunnel({ ...editingTunnel, stop_command: e.target.value })}
                      placeholder="e.g. pkill -f 'ssh -D 1080'"
                      className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono"
                    />
                  </div>
                </div>
              )}

            </div>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-800">
              <button
                onClick={() => setEditingTunnel(null)}
                className="px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-300"
              >
                Cancel
              </button>
              <button
                onClick={onSaveTunnel}
                className="px-3 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-xs font-semibold text-white"
              >
                Save Tunnel
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
