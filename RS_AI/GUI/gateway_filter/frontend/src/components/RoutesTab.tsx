import { useState } from "react";
import { Plus, Trash2, KeyRound, RotateCw, ChevronDown, ChevronUp, Sparkles, ExternalLink, Key, Fingerprint } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { GatewayConfig, RouteRule } from "../types";

interface RoutesTabProps {
  config: GatewayConfig;
  onToggleRoute: (routeId: string, enabled: boolean) => Promise<void>;
  onEditRoute: (route: RouteRule) => void;
  onSaveRouteDirect?: (route: RouteRule) => Promise<void>;
  onDeleteRoute: (id: string) => Promise<void>;
  onAdvanceKey: (routeId: string) => Promise<void>;
  onCreateRoute: () => void;
}

/// Ô nhập đường dẫn key file: giữ text cục bộ, chỉ commit (save IPC + ghi đĩa)
/// khi blur hoặc Enter (L3) — trước đây mỗi ký tự gõ là 1 lần save full config.
function KeyPathInput({
  value,
  placeholder,
  onCommit,
}: {
  value: string;
  placeholder: string;
  onCommit: (v: string) => Promise<void>;
}) {
  const [text, setText] = useState(value);
  const [saving, setSaving] = useState(false);

  const commit = async () => {
    if (text === value || saving) return;
    setSaving(true);
    try {
      await onCommit(text);
    } catch (e) {
      console.error("Save key path error:", e);
    } finally {
      setSaving(false);
    }
  };

  return (
    <input
      type="text"
      placeholder={placeholder}
      value={text}
      disabled={saving}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          (e.target as HTMLInputElement).blur();
        }
      }}
      className="w-full bg-slate-900 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200 disabled:opacity-50"
    />
  );
}

export function RoutesTab({
  config,
  onToggleRoute,
  onEditRoute,
  onSaveRouteDirect,
  onDeleteRoute,
  onAdvanceKey,
  onCreateRoute,
}: RoutesTabProps) {
  const [expandedRouteId, setExpandedRouteId] = useState<string | null>(null);
  return (
    <div className="p-6 overflow-y-auto max-w-5xl space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-bold text-slate-100">Endpoints & Dedicated Key Files</h2>
          <p className="text-xs text-slate-400 mt-1">
            Each endpoint can link to its own separate key file.
          </p>
        </div>
        <button
          onClick={onCreateRoute}
          className="px-3 py-1.5 rounded-lg bg-cyan-600 hover:bg-cyan-500 text-xs font-semibold text-white flex items-center gap-1.5 transition"
        >
          <Plus className="w-4 h-4" />
          Add Endpoint Route
        </button>
      </div>

      <div className="space-y-4">
        {config.routes.map((route) => {
          const assignedTunnel = config.tunnels.find((t) => t.id === route.tunnel_id);
          const km = route.key_manager;

          return (
            <div
              key={route.id}
              className="p-4 rounded-xl bg-slate-900/80 border border-slate-800 space-y-3"
            >
              <div className="flex flex-col md:flex-row md:items-center justify-between gap-2 pb-2 border-b border-slate-800/60">
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => onToggleRoute(route.id, !route.enabled)}
                    className={`relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
                      route.enabled ? "bg-emerald-500" : "bg-slate-700"
                    }`}
                    title={route.enabled ? "Route is Active (Click to Pause)" : "Route is Disabled (Click to Activate)"}
                  >
                    <span
                      className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                        route.enabled ? "translate-x-4" : "translate-x-0"
                      }`}
                    />
                  </button>
                  <span className="text-sm font-bold text-white">{route.name}</span>
                  <span className="px-2 py-0.5 rounded bg-cyan-500/20 text-cyan-400 font-mono text-[11px] font-bold">
                    Port {route.port}
                  </span>
                  <span className="px-2 py-0.5 rounded bg-slate-800 text-slate-300 font-mono text-[11px]">
                    {route.path_prefix}
                  </span>
                  <span
                    className={`px-2 py-0.5 rounded font-mono text-[11px] flex items-center gap-1 ${
                      route.fingerprint_index == null
                        ? "bg-slate-800 text-slate-400"
                        : "bg-violet-500/20 text-violet-300"
                    }`}
                    title={
                      route.fingerprint_index == null
                        ? "Uses global default fingerprint (pool active)"
                        : `Pinned to fingerprint profile #${route.fingerprint_index + 1} — rotates independently on 401/403`
                    }
                  >
                    <Fingerprint className="w-3 h-3" />
                    {route.fingerprint_index == null
                      ? "FP: Global"
                      : `FP #${route.fingerprint_index + 1}`}
                  </span>
                  {!route.enabled ? (
                    <span className="text-[10px] px-1.5 py-0.5 rounded font-mono bg-slate-800 text-slate-500">
                      Inactive
                    </span>
                  ) : route.status === "Active" ? (
                    <span className="text-[10px] px-1.5 py-0.5 rounded font-mono bg-emerald-500/20 text-emerald-400">
                      Active
                    </span>
                  ) : route.status === "Inactive" ? (
                    <span
                      className="text-[10px] px-1.5 py-0.5 rounded font-mono bg-rose-500/20 text-rose-400 cursor-help"
                      title={route.last_error || "Failed to bind port"}
                    >
                      Inactive
                    </span>
                  ) : (
                    <span
                      className="text-[10px] px-1.5 py-0.5 rounded font-mono bg-amber-500/20 text-amber-300 animate-pulse"
                      title="Waiting for listener bind / health check"
                    >
                      Unknown
                    </span>
                  )}
                </div>

                <div className="flex items-center gap-2">
                  <select
                    value={route.fingerprint_index ?? "global"}
                    onChange={async (e) => {
                      const v = e.target.value;
                      const updated: RouteRule = {
                        ...route,
                        fingerprint_index: v === "global" ? null : parseInt(v),
                      };
                      if (onSaveRouteDirect) await onSaveRouteDirect(updated);
                    }}
                    title="Quick-select fingerprint profile for this endpoint"
                    className="px-2 py-1 rounded bg-slate-800 hover:bg-slate-700 text-[11px] font-mono text-slate-300 border border-slate-700/60 max-w-32"
                  >
                    <option value="global">FP: Global</option>
                    {(config.fingerprint_pool ?? []).map((p, i) => (
                      <option key={i} value={i}>
                        FP #{i + 1} — {p.mode}
                      </option>
                    ))}
                  </select>
                  <button
                    onClick={() => setExpandedRouteId(expandedRouteId === route.id ? null : route.id)}
                    className={`px-2.5 py-1 rounded text-xs font-semibold flex items-center gap-1 transition ${
                      expandedRouteId === route.id
                        ? "bg-amber-500/20 text-amber-300 border border-amber-500/40"
                        : "bg-slate-800 hover:bg-slate-700 text-slate-300"
                    }`}
                  >
                    <Key className="w-3.5 h-3.5" />
                    Manage Keys
                    {expandedRouteId === route.id ? (
                      <ChevronUp className="w-3 h-3" />
                    ) : (
                      <ChevronDown className="w-3 h-3" />
                    )}
                  </button>
                  <button
                    onClick={() => onEditRoute(route)}
                    className="px-3 py-1 rounded bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 transition"
                  >
                    Edit
                  </button>
                  <button
                    onClick={() => onDeleteRoute(route.id)}
                    className="p-1 rounded hover:bg-rose-500/20 text-slate-400 hover:text-rose-400 transition"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
                <div>
                  <span className="text-slate-500">Target Base URL:</span>{" "}
                  <span className="font-mono text-slate-300 bg-slate-950 px-2 py-0.5 rounded border border-slate-800 truncate inline-block max-w-xs">
                    {route.target_base_url}
                  </span>
                </div>
                <div>
                  <span className="text-slate-500">Outbound Tunnel:</span>{" "}
                  <span className="text-indigo-400 font-mono">
                    {assignedTunnel ? `${assignedTunnel.name} (${assignedTunnel.protocol})` : route.tunnel_id}
                  </span>
                </div>
              </div>

              {/* Expandable Key Management Panel (Accordion) */}
              {expandedRouteId === route.id && (
                <div className="p-4 rounded-xl bg-slate-950 border border-amber-500/30 space-y-4 shadow-inner">
                  <div className="flex items-center justify-between border-b border-slate-800 pb-2">
                    <span className="text-xs font-bold text-amber-400 flex items-center gap-1.5">
                      <Key className="w-4 h-4" />
                      Dedicated Key Storage for {route.name}
                    </span>
                    <span className="text-[11px] text-slate-500 font-mono">
                      Changes auto-sync to backend
                    </span>
                  </div>

                  <div className="space-y-3 text-xs">
                    {/* Dedicated Key File Path */}
                    <div>
                      <div className="flex items-center justify-between mb-1">
                        <label className="text-slate-400">Dedicated Key File Path (.txt)</label>
                        <div className="flex items-center gap-2">
                          <button
                            type="button"
                            onClick={async () => {
                              try {
                                const path = await invoke<string>("generate_key_file", {
                                  endpointName: route.name || "endpoint",
                                });
                                const updated = {
                                  ...route,
                                  key_manager: {
                                    ...route.key_manager,
                                    key_file_path: path,
                                  },
                                };
                                if (onSaveRouteDirect) await onSaveRouteDirect(updated);
                              } catch (err) {
                                console.error("Auto generate key file error:", err);
                              }
                            }}
                            className="text-[11px] text-cyan-400 hover:text-cyan-300 flex items-center gap-1 bg-cyan-950/50 hover:bg-cyan-900/50 border border-cyan-800/60 px-2 py-0.5 rounded transition"
                            title="Auto-create a new key file on disk"
                          >
                            <Sparkles className="w-3 h-3" />
                            Auto Generate
                          </button>

                          {km.key_file_path && (
                            <button
                              type="button"
                              onClick={async () => {
                                try {
                                  await invoke("open_key_file", {
                                    filePath: km.key_file_path,
                                  });
                                } catch (err) {
                                  console.error("Open key file error:", err);
                                }
                              }}
                              className="text-[11px] text-indigo-400 hover:text-indigo-300 flex items-center gap-1 bg-indigo-950/50 hover:bg-indigo-900/50 border border-indigo-800/60 px-2 py-0.5 rounded transition"
                              title="Open file in OS default editor"
                            >
                              <ExternalLink className="w-3 h-3" />
                              Open File
                            </button>
                          )}
                        </div>
                      </div>
                      <KeyPathInput
                        value={km.key_file_path || ""}
                        placeholder="/path/to/endpoint_keys.txt"
                        onCommit={async (v) => {
                          const updated = {
                            ...route,
                            key_manager: {
                              ...route.key_manager,
                              key_file_path: v,
                            },
                          };
                          if (onSaveRouteDirect) await onSaveRouteDirect(updated);
                        }}
                      />
                    </div>

                    {/* Failed Key File Path */}
                    <div>
                      <div className="flex items-center justify-between mb-1">
                        <label className="text-slate-400">Failed Key File Path (403 Quota Error)</label>
                        <div className="flex items-center gap-2">
                          <button
                            type="button"
                            onClick={async () => {
                              try {
                                const path = await invoke<string>("generate_key_file", {
                                  endpointName: `${route.name || "endpoint"}_failed_403`,
                                });
                                const updated = {
                                  ...route,
                                  key_manager: {
                                    ...route.key_manager,
                                    failed_key_file_path: path,
                                  },
                                };
                                if (onSaveRouteDirect) await onSaveRouteDirect(updated);
                              } catch (err) {
                                console.error("Auto generate failed key file error:", err);
                              }
                            }}
                            className="text-[11px] text-rose-400 hover:text-rose-300 flex items-center gap-1 bg-rose-950/50 hover:bg-rose-900/50 border border-rose-800/60 px-2 py-0.5 rounded transition"
                            title="Auto-create a failed keys file on disk"
                          >
                            <Sparkles className="w-3 h-3" />
                            Auto Generate
                          </button>

                          {km.failed_key_file_path && (
                            <button
                              type="button"
                              onClick={async () => {
                                try {
                                  await invoke("open_key_file", {
                                    filePath: km.failed_key_file_path,
                                  });
                                } catch (err) {
                                  console.error("Open failed key file error:", err);
                                }
                              }}
                              className="text-[11px] text-slate-400 hover:text-slate-300 flex items-center gap-1 bg-slate-800/50 hover:bg-slate-750/50 border border-slate-700 px-2 py-0.5 rounded transition"
                              title="Open failed key file in OS editor"
                            >
                              <ExternalLink className="w-3 h-3" />
                              Open File
                            </button>
                          )}
                        </div>
                      </div>
                      <KeyPathInput
                        value={km.failed_key_file_path || ""}
                        placeholder="/path/to/endpoint_failed_403.txt"
                        onCommit={async (v) => {
                          const updated = {
                            ...route,
                            key_manager: {
                              ...route.key_manager,
                              failed_key_file_path: v,
                            },
                          };
                          if (onSaveRouteDirect) await onSaveRouteDirect(updated);
                        }}
                      />
                    </div>
                  </div>
                </div>
              )}

              <div className="p-3 rounded-lg bg-slate-950/80 border border-slate-800/80 flex flex-col md:flex-row items-start md:items-center justify-between gap-3">
                <div className="space-y-1 flex-1">
                  <div className="flex items-center gap-2 text-xs font-semibold text-slate-300">
                    <KeyRound className="w-3.5 h-3.5 text-amber-400" />
                    <span>Linked Key File:</span>
                    <span className="font-mono text-[11px] text-amber-300 truncate max-w-sm">
                      {km.key_file_path || "(No key file assigned)"}
                    </span>
                  </div>
                  <div className="flex items-center gap-3 text-[11px] text-slate-400 font-mono">
                    <span>
                      Active Key:{" "}
                      <strong className="text-emerald-400">
                        {km.current_key_preview || "(Empty / None)"}
                      </strong>
                    </span>
                    <span>
                      Slot: {km.current_key_index + 1} / {km.total_keys || 0}
                    </span>
                  </div>
                </div>

                {km.key_file_path && km.total_keys > 1 && (
                  <button
                    onClick={() => onAdvanceKey(route.id)}
                    className="px-2.5 py-1.5 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 text-xs font-semibold flex items-center gap-1.5 transition"
                  >
                    <RotateCw className="w-3.5 h-3.5" />
                    Next Key
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
