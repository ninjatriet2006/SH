import { Shield, Activity, Route, Network, Fingerprint } from "lucide-react";
import { GatewayConfig, RawTrafficLog } from "../types";

interface SidebarProps {
  activeTab: "traffic" | "routes" | "tunnels" | "fingerprint";
  setActiveTab: (tab: "traffic" | "routes" | "tunnels" | "fingerprint") => void;
  config: GatewayConfig | null;
  trafficLogs: RawTrafficLog[];
}

export function Sidebar({
  activeTab,
  setActiveTab,
  config,
  trafficLogs,
}: SidebarProps) {
  return (
    <aside className="w-64 border-r border-slate-800 bg-[#0c1220] flex flex-col justify-between select-none">
      <div>
        {/* Logo */}
        <div className="p-4 border-b border-slate-800 flex items-center gap-3">
          <div className="w-9 h-9 rounded-xl bg-gradient-to-tr from-cyan-500 via-indigo-600 to-purple-600 flex items-center justify-center shadow-lg shadow-cyan-500/20">
            <Shield className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="text-sm font-bold tracking-wide text-white">AI Mesh Gateway</h1>
            <p className="text-[11px] text-slate-400">100% Raw Wire Inspector</p>
          </div>
        </div>

        {/* Quick Metrics */}
        <div className="p-3 m-3 rounded-xl bg-slate-900/90 border border-slate-800 space-y-1.5 text-xs">
          <div className="flex items-center justify-between">
            <span className="text-slate-400">Captured Packets:</span>
            <span className="font-mono text-cyan-400 font-bold">{trafficLogs.length}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-slate-400">Active Endpoints:</span>
            <span className="font-mono text-indigo-400 font-bold">{config?.routes.length || 0}</span>
          </div>
        </div>

        {/* Navigation */}
        <nav className="px-3 space-y-1">
          <button
            onClick={() => setActiveTab("traffic")}
            className={`w-full flex items-center justify-between px-3 py-2.5 rounded-lg text-xs font-medium transition-all ${
              activeTab === "traffic"
                ? "bg-cyan-500/15 text-cyan-400 border border-cyan-500/30"
                : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/50"
            }`}
          >
            <div className="flex items-center gap-2.5">
              <Activity className="w-4 h-4" />
              <span>Raw Wire Traffic</span>
            </div>
            <span className="px-1.5 py-0.5 rounded-full text-[10px] bg-slate-800 text-slate-300 font-mono">
              {trafficLogs.length}
            </span>
          </button>

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
              <span>Endpoints & Keys</span>
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
              <span>Fingerprint Cloaking</span>
            </div>
          </button>
        </nav>
      </div>

      <div className="p-3 border-t border-slate-800 text-[11px] text-slate-500 font-mono">
        Memory Ring-Buffer (Auto Overwrite)
      </div>
    </aside>
  );
}
