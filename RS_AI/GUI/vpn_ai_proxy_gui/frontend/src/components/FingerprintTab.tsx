import { useState } from "react";
import {
  Plus,
  Trash2,
  Pencil,
  Fingerprint,
  Star,
  ShieldCheck,
  Info,
} from "lucide-react";
import { GatewayConfig, FingerprintProfile } from "../types";

interface FingerprintTabProps {
  config: GatewayConfig;
  onSaveConfig: (cfg: GatewayConfig) => Promise<void>;
}

const FLAG_LABELS: [keyof FingerprintProfile, string][] = [
  ["strip_sdk_headers", "Strip SDK"],
  ["strip_ide_headers", "Strip IDE"],
  ["strip_sec_ch_ua", "Strip Sec-CH-UA"],
  ["remove_empty_headers", "Drop Empty"],
  ["mask_local_paths_in_body", "Mask Paths"],
];

function defaultProfile(): FingerprintProfile {
  return {
    mode: "custom",
    custom_user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
    strip_sdk_headers: true,
    strip_ide_headers: true,
    strip_sec_ch_ua: true,
    remove_empty_headers: true,
    mask_local_paths_in_body: true,
    spoof_headers: {},
  };
}

export function FingerprintTab({ config, onSaveConfig }: FingerprintTabProps) {
  const pool = config.fingerprint_pool ?? [];
  const activeIndex =
    pool.length > 0 ? (config.active_fingerprint_index ?? 0) % pool.length : -1;

  const [editing, setEditing] = useState<{
    index: number | null;
    profile: FingerprintProfile;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const makePool = (
    mutate: (pool: FingerprintProfile[]) => FingerprintProfile[]
  ) => {
    const next = mutate([...pool]);
    const maxIndex = Math.max(0, next.length - 1);
    const clampedIndex = Math.min(config.active_fingerprint_index ?? 0, maxIndex);
    return onSaveConfig({
      ...config,
      fingerprint_pool: next,
      active_fingerprint_index: clampedIndex,
    });
  };

  const handleSaveProfile = async () => {
    if (!editing) return;
    const prof = editing.profile;
    if (!(prof.custom_user_agent ?? "").trim()) {
      setError("User-Agent cannot be empty.");
      return;
    }
    setError(null);
    await makePool((pool) => {
      const next = [...pool];
      if (editing.index === null) {
        next.push(prof);
      } else {
        next[editing.index] = prof;
      }
      return next;
    });
    setEditing(null);
  };

  const handleDelete = async (index: number) => {
    await makePool((pool) => pool.filter((_, i) => i !== index));
  };

  const handleSetActive = async (index: number) => {
    await onSaveConfig({ ...config, active_fingerprint_index: index });
  };

  return (
    <div className="p-6 overflow-y-auto max-w-5xl space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-bold text-slate-100">Fingerprint Pool & Session Rotation</h2>
          <p className="text-xs text-slate-400 mt-1">
            Manage multiple anonymity profiles. The active profile rotates automatically when keys advance.
          </p>
        </div>
        <button
          onClick={() => setEditing({ index: null, profile: defaultProfile() })}
          className="px-3 py-1.5 rounded-lg bg-cyan-600 hover:bg-cyan-500 text-xs font-semibold text-white flex items-center gap-1.5 transition"
        >
          <Plus className="w-4 h-4" />
          Add New Fingerprint Profile
        </button>
      </div>

      {pool.length === 0 && (
        <div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/30 flex items-start gap-3">
          <Info className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
          <div className="text-xs text-amber-200/90 space-y-1">
            <div className="font-semibold text-amber-300">No fingerprint profiles in the pool.</div>
            <div>
              Traffic is currently masked by the legacy static profile. Add profiles above to enable
              per-session rotation.
            </div>
          </div>
        </div>
      )}

      <div className="space-y-4">
        {pool.map((p, index) => {
          const isActive = index === activeIndex;
          return (
            <div
              key={index}
              className={`p-4 rounded-xl border space-y-3 transition ${
                isActive
                  ? "bg-slate-900/80 border-cyan-500/40 ring-1 ring-cyan-500/20"
                  : "bg-slate-900/80 border-slate-800"
              }`}
            >
              <div className="flex flex-col md:flex-row md:items-center justify-between gap-2 pb-2 border-b border-slate-800/60">
                <div className="flex items-center gap-2 min-w-0">
                  {isActive && (
                    <span className="px-2 py-0.5 rounded bg-cyan-500/20 text-cyan-400 text-[10px] font-bold flex items-center gap-1">
                      <Star className="w-3 h-3" />
                      Active
                    </span>
                  )}
                  <span className="px-2 py-0.5 rounded bg-slate-800 text-slate-300 font-mono text-[10px]">
                    profile #{index + 1}
                  </span>
                </div>

                <div className="flex items-center gap-2">
                  {!isActive && (
                    <button
                      onClick={() => handleSetActive(index)}
                      className="px-2.5 py-1 rounded bg-cyan-500/15 hover:bg-cyan-500/25 text-cyan-300 text-xs font-semibold flex items-center gap-1 transition"
                    >
                      <Star className="w-3.5 h-3.5" />
                      Set Active
                    </button>
                  )}
                  <button
                    onClick={() => setEditing({ index, profile: { ...p } })}
                    className="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 flex items-center gap-1 transition"
                  >
                    <Pencil className="w-3.5 h-3.5" />
                    Edit
                  </button>
                  <button
                    onClick={() => handleDelete(index)}
                    className="p-1.5 rounded hover:bg-rose-500/20 text-slate-400 hover:text-rose-400 transition"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>

              <div className="flex items-center gap-2 text-xs">
                <Fingerprint className="w-4 h-4 text-amber-400 shrink-0" />
                <span className="font-mono text-slate-200 truncate">
                  {p.custom_user_agent || "(empty)"}
                </span>
              </div>

              <div className="flex flex-wrap items-center gap-1.5">
                {FLAG_LABELS.filter(([key]) => (p as any)[key]).map(([, label]) => (
                  <span
                    key={label}
                    className="px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-300 text-[10px] font-mono border border-emerald-500/30"
                  >
                    {label}
                  </span>
                ))}
                {!FLAG_LABELS.some(([key]) => (p as any)[key]) && (
                  <span className="text-[10px] text-slate-500">No sanitization flags enabled</span>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* MODAL: EDIT / ADD FINGERPRINT PROFILE */}
      {editing && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 z-50">
          <div className="bg-slate-900 border border-slate-800 rounded-2xl w-full max-w-lg p-5 space-y-4 shadow-2xl">
            <h3 className="text-sm font-bold text-white flex items-center gap-2">
              <Fingerprint className="w-4 h-4 text-amber-400" />
              {editing.index === null ? "New" : `Edit #${editing.index + 1}`} Fingerprint Profile
            </h3>

            {error && (
              <div className="p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/30 text-xs text-rose-300">
                {error}
              </div>
            )}

            <div className="space-y-3 text-xs">
              <div>
                <label className="text-slate-400 block mb-1">Mode</label>
                <select
                  value={editing.profile.mode}
                  onChange={(e) =>
                    setEditing({
                      ...editing,
                      profile: { ...editing.profile, mode: e.target.value },
                    })
                  }
                  className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200"
                >
                  <option value="stealth">stealth</option>
                  <option value="browser_chrome">browser_chrome</option>
                  <option value="minimal_curl">minimal_curl</option>
                  <option value="raw">raw</option>
                  <option value="custom">custom</option>
                </select>
              </div>

              <div>
                <label className="text-slate-400 block mb-1">Custom User-Agent</label>
                <input
                  type="text"
                  value={editing.profile.custom_user_agent ?? ""}
                  onChange={(e) =>
                    setEditing({
                      ...editing,
                      profile: { ...editing.profile, custom_user_agent: e.target.value },
                    })
                  }
                  className="w-full bg-slate-950 border border-slate-800 rounded px-3 py-1.5 font-mono text-slate-200"
                />
              </div>

              <div className="grid grid-cols-1 gap-2 pt-1">
                {FLAG_LABELS.map(([key, label]) => (
                  <label
                    key={key}
                    className="p-2.5 rounded-lg bg-slate-950 border border-slate-800 flex items-center justify-between text-xs cursor-pointer"
                  >
                    <span className="font-semibold text-slate-300">{label}</span>
                    <input
                      type="checkbox"
                      checked={(editing.profile as any)[key]}
                      onChange={(e) =>
                        setEditing({
                          ...editing,
                          profile: { ...editing.profile, [key]: e.target.checked },
                        })
                      }
                      className="accent-cyan-500"
                    />
                  </label>
                ))}
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-800">
              <button
                onClick={() => setEditing(null)}
                className="px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-300"
              >
                Cancel
              </button>
              <button
                onClick={handleSaveProfile}
                className="px-3 py-1.5 rounded bg-cyan-600 hover:bg-cyan-500 text-xs font-semibold text-white"
              >
                {editing.index === null ? "Add Profile" : "Save Profile"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Backend sync note */}
      {pool.length > 0 && (
        <div className="flex items-center gap-2 text-[11px] text-slate-500">
          <ShieldCheck className="w-3.5 h-3.5 text-emerald-500" />
          Backend uses the active pool profile for outbound spoofing and rotates to the next profile
          on every key switch.
        </div>
      )}
    </div>
  );
}