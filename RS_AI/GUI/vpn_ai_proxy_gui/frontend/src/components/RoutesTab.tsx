import { Plus, Trash2, KeyRound, RotateCw } from "lucide-react";
import { GatewayConfig, RouteRule } from "../types";

interface RoutesTabProps {
  config: GatewayConfig;
  onEditRoute: (route: RouteRule) => void;
  onDeleteRoute: (id: string) => Promise<void>;
  onAdvanceKey: (routeId: string) => Promise<void>;
  onCreateRoute: () => void;
}

export function RoutesTab({
  config,
  onEditRoute,
  onDeleteRoute,
  onAdvanceKey,
  onCreateRoute,
}: RoutesTabProps) {
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
                  <span className="text-sm font-bold text-white">{route.name}</span>
                  <span className="px-2 py-0.5 rounded bg-cyan-500/20 text-cyan-400 font-mono text-[11px] font-bold">
                    Port {route.port}
                  </span>
                  <span className="px-2 py-0.5 rounded bg-slate-800 text-slate-300 font-mono text-[11px]">
                    {route.path_prefix}
                  </span>
                  <span
                    className={`text-[10px] px-1.5 py-0.5 rounded font-mono ${
                      route.enabled ? "bg-emerald-500/20 text-emerald-400" : "bg-slate-800 text-slate-500"
                    }`}
                  >
                    {route.enabled ? "Active" : "Disabled"}
                  </span>
                </div>

                <div className="flex items-center gap-2">
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
