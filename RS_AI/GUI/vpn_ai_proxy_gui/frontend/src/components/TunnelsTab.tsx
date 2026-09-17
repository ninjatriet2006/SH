import { useState } from "react";
import { Plus, Zap, Trash2, Check } from "lucide-react";
import { GatewayConfig, OutboundTunnel } from "../types";

interface TunnelsTabProps {
  config: GatewayConfig;
  activeCliTunnelId: string | null;
  onTestTunnel: (tunnel: OutboundTunnel) => Promise<void>;
  onToggleTunnel: (tunnelId: string, enabled: boolean) => Promise<void>;
  onForceStop?: (tunnelId: string) => Promise<void>;
  onEditTunnel: (tunnel: OutboundTunnel) => void;
  onDeleteTunnel: (id: string) => Promise<void>;
  onCreateTunnel: () => void;
}

export function TunnelsTab({
  config,
  activeCliTunnelId,
  onTestTunnel,
  onToggleTunnel,
  onForceStop,
  onEditTunnel,
  onDeleteTunnel,
  onCreateTunnel,
}: TunnelsTabProps) {
  // Busy flag cục bộ cho nút Test (kết quả đọc từ tunnel.last_exit_ip, không duplicate state)
  const [testingId, setTestingId] = useState<string | null>(null);
  // Loading state cho Toggle: Backend block vài giây chạy CLI + test, hiển thị CONNECTING... ngay khi bấm
  const [togglingId, setTogglingId] = useState<string | null>(null);
  return (
    <div className="p-6 overflow-y-auto max-w-5xl space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-bold text-slate-100">Outbound VPN & Proxy Pool</h2>
          <p className="text-xs text-slate-400 mt-1">
            Manage multiple local VPN tunnels (AdGuard, WireGuard, HTTP proxies).
          </p>
        </div>
        <button
          onClick={onCreateTunnel}
          className="px-3 py-1.5 rounded-lg bg-cyan-600 hover:bg-cyan-500 text-xs font-semibold text-white flex items-center gap-1.5 transition"
        >
          <Plus className="w-4 h-4" />
          Add Tunnel Node
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {config.tunnels.map((tunnel) => {
          const isTesting = testingId === tunnel.id;
          const isToggling = togglingId === tunnel.id;

          return (
            <div
              key={tunnel.id}
              className="p-4 rounded-xl bg-slate-900/80 border border-slate-800 space-y-3 flex flex-col justify-between"
            >
              <div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      disabled={isToggling}
                      onClick={async () => {
                        setTogglingId(tunnel.id);
                        try {
                          await onToggleTunnel(tunnel.id, !tunnel.enabled);
                        } finally {
                          setTogglingId(null);
                        }
                      }}
                      className={`relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none disabled:cursor-wait ${
                        tunnel.enabled ? "bg-emerald-500" : "bg-slate-700"
                      } ${isToggling ? "opacity-50 animate-pulse" : ""}`}
                      title={
                        isToggling
                          ? "Connecting... please wait"
                          : tunnel.enabled
                            ? "Tunnel is Enabled (Click to turn Off)"
                            : "Tunnel is Disabled (Click to turn On)"
                      }
                    >
                      <span
                        className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                          tunnel.enabled ? "translate-x-4" : "translate-x-0"
                        }`}
                      />
                    </button>
                    <span className="text-xs font-bold text-slate-200">{tunnel.name}</span>
                    {isToggling && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded font-mono font-bold bg-amber-500/20 text-amber-300 animate-pulse">
                        CONNECTING...
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-1.5">
                    {activeCliTunnelId === tunnel.id && (
                      <span className="uppercase text-[10px] px-2 py-0.5 rounded font-mono font-bold bg-emerald-500 text-slate-950 animate-pulse">
                        ▶ RUNNING
                      </span>
                    )}
                    <span className={`uppercase text-[10px] px-2 py-0.5 rounded font-mono font-bold ${
                      tunnel.status === "online"
                        ? "bg-emerald-500/20 text-emerald-400"
                        : tunnel.status === "offline"
                        ? "bg-rose-500/20 text-rose-400"
                        : "bg-slate-700/40 text-slate-400"
                    }`}>
                      {tunnel.status || "unknown"}
                    </span>
                    <span className="uppercase text-[10px] px-2 py-0.5 rounded font-mono font-bold bg-indigo-500/20 text-indigo-400">
                      {tunnel.protocol}
                    </span>
                  </div>
                </div>
                <div className="text-xs font-mono text-slate-400 mt-1">
                  Endpoint: {tunnel.endpoint || "(Direct/Bypass)"}
                  <span className="ml-2 text-indigo-400/80">
                    Streams: {tunnel.max_concurrent_streams ? `${tunnel.max_concurrent_streams} max` : "unlimited"}
                  </span>
                  {!tunnel.enabled && (
                    <span className="ml-2 text-rose-400 font-semibold">(Disabled)</span>
                  )}
                  {tunnel.last_checked_at && (
                    <div className="text-[10px] text-slate-500 mt-0.5">Checked: {tunnel.last_checked_at}</div>
                  )}
                </div>

                {(tunnel.last_exit_ip || tunnel.last_error) && (
                  <div className="mt-3 p-2 rounded bg-slate-950 border border-slate-800 text-[11px] font-mono space-y-0.5">
                    {tunnel.last_exit_ip ? (
                      <>
                        <div className="text-emerald-400 flex items-center gap-1 font-semibold">
                          <Check className="w-3.5 h-3.5" />
                          Exit IP: {tunnel.last_exit_ip}
                        </div>
                        {tunnel.last_latency_ms !== undefined && (
                          <div className="text-slate-500">Latency: {tunnel.last_latency_ms}ms</div>
                        )}
                      </>
                    ) : (
                      <div className="text-rose-400">Error: {tunnel.last_error}</div>
                    )}
                  </div>
                )}
              </div>

              <div className="pt-2 border-t border-slate-800/80 flex items-center justify-between">
                <div className="flex items-center gap-1.5">
                  <button
                    disabled={isTesting}
                    onClick={async () => {
                      setTestingId(tunnel.id);
                      try {
                        await onTestTunnel(tunnel);
                      } finally {
                        setTestingId(null);
                      }
                    }}
                    className="px-2.5 py-1.5 rounded bg-cyan-600/20 hover:bg-cyan-600/30 text-cyan-300 text-xs font-semibold flex items-center gap-1 transition disabled:opacity-50"
                  >
                    <Zap className="w-3.5 h-3.5" />
                    {isTesting ? "Testing..." : "Test Exit IP"}
                  </button>

                  {(tunnel.start_command || tunnel.stop_command) && onForceStop && (
                    <button
                      onClick={async () => {
                        if (window.confirm(`Force kill VPN process of "${tunnel.name}"? This bypasses the safety lock.`)) {
                          await onForceStop(tunnel.id);
                        }
                      }}
                      className="px-2 py-1.5 rounded bg-rose-600/20 hover:bg-rose-600/40 text-rose-400 text-xs font-semibold flex items-center gap-1 transition"
                      title="Force Stop / Kill: fire stop_command + killall to free a stuck port"
                    >
                      Force Kill
                    </button>
                  )}
                </div>

                <div className="flex items-center gap-2">
                  <button
                    onClick={() => onEditTunnel(tunnel)}
                    className="px-2.5 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-300 transition"
                  >
                    Edit
                  </button>
                  {tunnel.id !== "adguard_default" && tunnel.id !== "direct_bypass" && (
                    <button
                      onClick={() => onDeleteTunnel(tunnel.id)}
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
  );
}
