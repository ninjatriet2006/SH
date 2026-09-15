import { GatewayConfig, RouteRule, OutboundTunnel } from "../types";

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
                <label className="text-slate-400 block mb-1">Dedicated Key File Path (.txt or .json)</label>
                <input
                  type="text"
                  placeholder="/path/to/endpoint_keys.txt"
                  value={editingRoute.key_manager.key_file_path || ""}
                  onChange={(e) =>
                    setEditingRoute({
                      ...editingRoute,
                      key_manager: {
                        ...editingRoute.key_manager,
                        key_file_path: e.target.value,
                      },
                    })
                  }
                  className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200"
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

              <div className="flex items-center gap-4 pt-2">
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={editingRoute.strip_prefix}
                    onChange={(e) => setEditingRoute({ ...editingRoute, strip_prefix: e.target.checked })}
                    className="accent-cyan-500"
                  />
                  <span>Strip path prefix</span>
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
