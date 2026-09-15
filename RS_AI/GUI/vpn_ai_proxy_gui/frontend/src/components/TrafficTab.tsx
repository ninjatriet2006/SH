import { useState } from "react";
import { Search, Trash2, Radio, Copy } from "lucide-react";
import { RawTrafficLog } from "../types";

interface TrafficTabProps {
  trafficLogs: RawTrafficLog[];
  selectedLog: RawTrafficLog | null;
  setSelectedLog: (log: RawTrafficLog | null) => void;
  onClearLogs: () => Promise<void>;
  maxLogEntries?: number;
  onUpdateMaxLogEntries?: (count: number) => Promise<void>;
}

export function TrafficTab({
  trafficLogs,
  selectedLog,
  setSelectedLog,
  onClearLogs,
  maxLogEntries = 500,
  onUpdateMaxLogEntries,
}: TrafficTabProps) {
  const [searchTerm, setSearchTerm] = useState("");

  const filteredLogs = trafficLogs.filter((l) => {
    if (!searchTerm) return true;
    const s = searchTerm.toLowerCase();
    return (
      l.target_url.toLowerCase().includes(s) ||
      l.path.toLowerCase().includes(s) ||
      l.status_code.toString().includes(s) ||
      l.raw_request_body.toLowerCase().includes(s) ||
      l.raw_response_body.toLowerCase().includes(s)
    );
  });

  return (
    <div className="flex-1 flex overflow-hidden">
      {/* Packet Stream */}
      <div className="w-5/12 border-r border-slate-800 flex flex-col">
        <div className="h-12 border-b border-slate-800 px-3 flex items-center justify-between bg-slate-900/40 gap-2">
          <div className="relative flex-1">
            <Search className="w-3.5 h-3.5 absolute left-2.5 top-2.5 text-slate-500" />
            <input
              type="text"
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              placeholder="Search raw traffic (body, url, code)..."
              className="w-full bg-slate-950 border border-slate-800 rounded-lg pl-8 pr-2 py-1 text-xs font-mono text-slate-200 focus:outline-none focus:border-cyan-500"
            />
          </div>

          {onUpdateMaxLogEntries && (
            <div className="flex items-center gap-1 text-[11px] text-slate-400 font-mono">
              <span>Max:</span>
              <select
                value={maxLogEntries}
                onChange={(e) => onUpdateMaxLogEntries(Number(e.target.value))}
                className="bg-slate-950 border border-slate-800 rounded px-1.5 py-0.5 text-slate-300 text-xs focus:outline-none"
              >
                <option value={50}>50</option>
                <option value={100}>100</option>
                <option value={200}>200</option>
                <option value={500}>500</option>
                <option value={1000}>1000</option>
              </select>
            </div>
          )}

          <button
            onClick={onClearLogs}
            className="p-1.5 rounded hover:bg-slate-800 text-slate-400 hover:text-rose-400 transition"
            title="Clear Traffic Buffer"
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto divide-y divide-slate-800/60">
          {filteredLogs.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-slate-500 text-xs p-6 text-center space-y-2">
              <Radio className="w-8 h-8 stroke-1 text-slate-600 animate-pulse" />
              <div>No raw traffic recorded yet</div>
            </div>
          ) : (
            filteredLogs.map((log) => {
              const isSelected = selectedLog?.id === log.id;
              const isErr = log.status_code >= 400;
              return (
                <div
                  key={log.id}
                  onClick={() => setSelectedLog(log)}
                  className={`p-3 text-xs cursor-pointer transition flex items-center justify-between ${
                    isSelected ? "bg-cyan-500/10 border-l-2 border-cyan-400" : "hover:bg-slate-800/30"
                  }`}
                >
                  <div className="space-y-1 flex-1 pr-2 truncate">
                    <div className="flex items-center gap-1.5">
                      <span
                        className={`px-1.5 py-0.5 rounded text-[10px] font-bold font-mono ${
                          isErr ? "bg-rose-500/20 text-rose-400" : "bg-emerald-500/20 text-emerald-400"
                        }`}
                      >
                        {log.status_code}
                      </span>
                      <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-blue-500/20 text-blue-400">
                        {log.method}
                      </span>
                      <span className="font-mono text-slate-300 truncate">
                        :{log.port}
                        {log.path}
                      </span>
                    </div>
                    <div className="text-[11px] text-slate-500 truncate font-mono">↳ {log.target_url}</div>
                  </div>

                  <div className="flex flex-col items-end gap-1">
                    <span className="text-[10px] text-slate-400 font-mono">{log.duration_ms}ms</span>
                    <span className="text-[10px] text-slate-500 font-mono">{log.timestamp}</span>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </div>

      {/* Packet Inspector (100% Raw Data View) */}
      <div className="w-7/12 flex flex-col bg-slate-950/70 overflow-y-auto p-4 space-y-4">
        {selectedLog ? (
          <>
            <div className="flex items-center justify-between pb-3 border-b border-slate-800">
              <div>
                <h3 className="text-sm font-bold text-slate-200">Raw Packet Inspector</h3>
                <p className="text-xs text-slate-500 font-mono">ID: {selectedLog.id} · Route: {selectedLog.route_id}</p>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-xs px-2 py-1 rounded font-mono font-bold bg-slate-800 text-slate-300">
                  {selectedLog.duration_ms}ms
                </span>
                <span
                  className={`text-xs px-2 py-1 rounded font-mono font-bold ${
                    selectedLog.status_code >= 400
                      ? "bg-rose-500/20 text-rose-400"
                      : "bg-emerald-500/20 text-emerald-400"
                  }`}
                >
                  HTTP {selectedLog.status_code}
                </span>
              </div>
            </div>

            {/* Raw Request Headers */}
            <div>
              <div className="flex items-center justify-between mb-1">
                <h4 className="text-xs font-semibold uppercase text-slate-400">Raw Request Headers (From Client)</h4>
                <div className="flex items-center gap-2">
                  <span className="text-[10px] text-slate-500 font-mono">
                    {selectedLog.raw_request_headers.length} headers
                  </span>
                  <button
                    onClick={() => {
                      const text = selectedLog.raw_request_headers.map(([k, v]) => `${k}: ${v}`).join("\n");
                      navigator.clipboard.writeText(text);
                    }}
                    className="text-[10px] text-cyan-400 hover:underline flex items-center gap-1"
                  >
                    <Copy className="w-3 h-3" /> Copy
                  </button>
                </div>
              </div>
              <div className="p-2.5 rounded bg-slate-900 border border-slate-800 font-mono text-[10px] space-y-1 max-h-36 overflow-y-auto select-text">
                {selectedLog.raw_request_headers.map(([k, v], i) => (
                  <div key={i} className="truncate">
                    <span className="text-slate-500">{k}:</span> <span className="text-slate-300">{v}</span>
                  </div>
                ))}
              </div>
            </div>

            {/* Raw Forwarded Headers (Actual Wire Sent to Upstream) */}
            <div>
              <div className="flex items-center justify-between mb-1">
                <h4 className="text-xs font-semibold uppercase text-indigo-400">Raw Forwarded Headers (Sent to Upstream)</h4>
                <div className="flex items-center gap-2">
                  <span className="text-[10px] text-slate-500 font-mono">
                    {selectedLog.raw_forwarded_headers.length} headers
                  </span>
                  <button
                    onClick={() => {
                      const text = selectedLog.raw_forwarded_headers.map(([k, v]) => `${k}: ${v}`).join("\n");
                      navigator.clipboard.writeText(text);
                    }}
                    className="text-[10px] text-indigo-400 hover:underline flex items-center gap-1"
                  >
                    <Copy className="w-3 h-3" /> Copy
                  </button>
                </div>
              </div>
              <div className="p-2.5 rounded bg-slate-900 border border-indigo-950/80 font-mono text-[10px] space-y-1 max-h-36 overflow-y-auto select-text">
                {selectedLog.raw_forwarded_headers.map(([k, v], i) => (
                  <div key={i} className="truncate">
                    <span className="text-indigo-400/80">{k}:</span> <span className="text-slate-300">{v}</span>
                  </div>
                ))}
              </div>
            </div>

            {/* Raw Request Body */}
            <div>
              <div className="flex items-center justify-between mb-1">
                <h4 className="text-xs font-semibold uppercase text-slate-400">Raw Inbound Request Payload</h4>
                <button
                  onClick={() => navigator.clipboard.writeText(selectedLog.raw_request_body)}
                  className="text-[10px] text-cyan-400 hover:underline flex items-center gap-1"
                >
                  <Copy className="w-3 h-3" /> Copy
                </button>
              </div>
              <div className="p-3 rounded bg-slate-900 border border-slate-800 font-mono text-[11px] text-slate-200 whitespace-pre-wrap max-h-56 overflow-y-auto select-text">
                {selectedLog.raw_request_body || "<empty body>"}
              </div>
            </div>

            {/* Raw Response Headers */}
            <div>
              <div className="flex items-center justify-between mb-1">
                <h4 className="text-xs font-semibold uppercase text-slate-400">Raw Upstream Response Headers</h4>
                <div className="flex items-center gap-2">
                  <span className="text-[10px] text-slate-500 font-mono">
                    {selectedLog.raw_response_headers.length} headers
                  </span>
                  <button
                    onClick={() => {
                      const text = selectedLog.raw_response_headers.map(([k, v]) => `${k}: ${v}`).join("\n");
                      navigator.clipboard.writeText(text);
                    }}
                    className="text-[10px] text-cyan-400 hover:underline flex items-center gap-1"
                  >
                    <Copy className="w-3 h-3" /> Copy
                  </button>
                </div>
              </div>
              <div className="p-2.5 rounded bg-slate-900 border border-slate-800 font-mono text-[10px] space-y-1 max-h-36 overflow-y-auto select-text">
                {selectedLog.raw_response_headers.map(([k, v], i) => (
                  <div key={i} className="truncate">
                    <span className="text-slate-500">{k}:</span> <span className="text-slate-300">{v}</span>
                  </div>
                ))}
              </div>
            </div>

            {/* Raw Response Body */}
            <div>
              <div className="flex items-center justify-between mb-1">
                <h4
                  className={`text-xs font-semibold uppercase ${
                    selectedLog.status_code >= 400 ? "text-rose-400" : "text-emerald-400"
                  }`}
                >
                  Raw Upstream Response Body ({selectedLog.status_code})
                </h4>
                <button
                  onClick={() => navigator.clipboard.writeText(selectedLog.raw_response_body)}
                  className="text-[10px] text-cyan-400 hover:underline flex items-center gap-1"
                >
                  <Copy className="w-3 h-3" /> Copy
                </button>
              </div>
              <div
                className={`p-3 rounded bg-slate-900 border font-mono text-[11px] whitespace-pre-wrap max-h-72 overflow-y-auto select-text ${
                  selectedLog.status_code >= 400
                    ? "border-rose-900/60 text-rose-300"
                    : "border-slate-800 text-slate-200"
                }`}
              >
                {selectedLog.raw_response_body || "<empty response>"}
              </div>
            </div>
          </>
        ) : (
          <div className="h-full flex items-center justify-center text-slate-500 text-xs">
            Select a traffic packet from the left list to inspect full raw wire data
          </div>
        )}
      </div>
    </div>
  );
}
