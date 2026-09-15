import { useState } from "react";
import { GatewayConfig } from "../types";
import { HardDrive, Palette, Globe, ShieldCheck, RefreshCw } from "lucide-react";

interface SettingsTabProps {
  config: GatewayConfig;
  onSaveConfig: (cfg: GatewayConfig) => Promise<void>;
}

export function SettingsTab({ config, onSaveConfig }: SettingsTabProps) {
  const [maxDiskLogs, setMaxDiskLogs] = useState(config.max_disk_log_entries || 5000);
  const [theme, setTheme] = useState("dark");
  const [lang, setLang] = useState("vi");
  const [saved, setSaved] = useState(false);

  const handleSave = async () => {
    await onSaveConfig({
      ...config,
      max_disk_log_entries: maxDiskLogs,
    });
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div className="p-6 overflow-y-auto max-w-4xl space-y-6">
      <div>
        <h2 className="text-base font-bold text-slate-100">System & Portable Settings</h2>
        <p className="text-xs text-slate-400 mt-1">
          Configure portable disk logs rotation, system UI preferences, and process checks.
        </p>
      </div>

      <div className="space-y-4">
        {/* Disk Logging Rotation */}
        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800 space-y-4">
          <div className="flex items-center gap-2.5 text-cyan-400">
            <HardDrive className="w-5 h-5" />
            <h3 className="text-sm font-bold text-slate-100">Disk Log Storage & Rotation</h3>
          </div>
          <p className="text-xs text-slate-400">
            Requests are automatically written to <code className="text-slate-300 font-mono bg-slate-950 px-1.5 py-0.5 rounded">./logs/traffic_log.jsonl</code> next to the portable executable. When the log file exceeds this limit, older records are rotated.
          </p>

          <div className="flex items-center gap-3">
            <label className="text-xs text-slate-300">Max Disk Log Entries:</label>
            <select
              value={maxDiskLogs}
              onChange={(e) => setMaxDiskLogs(Number(e.target.value))}
              className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs font-mono text-slate-200 focus:outline-none focus:border-cyan-500"
            >
              <option value={1000}>1,000 entries (~2 MB)</option>
              <option value={5000}>5,000 entries (~10 MB)</option>
              <option value={10000}>10,000 entries (~20 MB)</option>
              <option value={50000}>50,000 entries (~100 MB)</option>
            </select>
          </div>
        </div>

        {/* UI Appearance */}
        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800 space-y-4">
          <div className="flex items-center gap-2.5 text-indigo-400">
            <Palette className="w-5 h-5" />
            <h3 className="text-sm font-bold text-slate-100">Appearance & Theme</h3>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-xs text-slate-400 block mb-1">Color Theme</label>
              <select
                value={theme}
                onChange={(e) => setTheme(e.target.value)}
                className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none"
              >
                <option value="dark">Cyberpunk Dark (Default)</option>
                <option value="slate">Slate Minimal</option>
                <option value="matrix">Matrix Glow</option>
              </select>
            </div>

            <div>
              <label className="text-xs text-slate-400 block mb-1">Interface Language</label>
              <select
                value={lang}
                onChange={(e) => setLang(e.target.value)}
                className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none"
              >
                <option value="vi">Tiếng Việt (Vietnamese)</option>
                <option value="en">English</option>
              </select>
            </div>
          </div>
        </div>

        {/* Core Protection Status */}
        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800 space-y-3">
          <div className="flex items-center gap-2.5 text-emerald-400">
            <ShieldCheck className="w-5 h-5" />
            <h3 className="text-sm font-bold text-slate-100">Port & Dependency Security Check</h3>
          </div>
          <p className="text-xs text-slate-400">
            All outbound tunnels automatically perform a pre-flight TCP handshake check before traffic forwarding. Offline nodes will be blocked with status code 503 instead of blind forward 502 errors.
          </p>
        </div>

        <div className="flex items-center justify-end gap-3 pt-2">
          {saved && <span className="text-xs text-emerald-400 font-semibold">Settings saved!</span>}
          <button
            onClick={handleSave}
            className="px-4 py-2 rounded-xl bg-cyan-600 hover:bg-cyan-500 text-xs font-bold text-white transition shadow-lg shadow-cyan-600/20"
          >
            Save Settings
          </button>
        </div>
      </div>
    </div>
  );
}
