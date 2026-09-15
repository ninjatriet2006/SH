import { GatewayConfig } from "../types";

interface FingerprintTabProps {
  config: GatewayConfig;
  onSaveConfig: (cfg: GatewayConfig) => Promise<void>;
}

export function FingerprintTab({ config, onSaveConfig }: FingerprintTabProps) {
  return (
    <div className="p-6 overflow-y-auto max-w-4xl space-y-6">
      <div>
        <h2 className="text-base font-bold text-slate-100">Fingerprint Cloaking & Sanitizer</h2>
        <p className="text-xs text-slate-400 mt-1">
          Disguise outbound User-Agent and strip client telemetry.
        </p>
      </div>

      <div className="p-4 rounded-xl bg-slate-900 border border-slate-800 space-y-4">
        <div>
          <label className="text-xs font-semibold text-slate-300 block mb-1">
            Outbound User-Agent Spoofing
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
              onSaveConfig(updated);
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
                onSaveConfig(updated);
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
                onSaveConfig(updated);
              }}
              className="accent-cyan-500"
            />
          </label>
        </div>
      </div>
    </div>
  );
}
