import { AlertTriangle, X } from "lucide-react";

interface PortConflictModalProps {
  bindError: { port: number; error: string } | null;
  onClose: () => void;
}

export function PortConflictModal({ bindError, onClose }: PortConflictModalProps) {
  if (!bindError) return null;

  return (
    <div className="fixed inset-0 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4 z-50 animate-in fade-in duration-200">
      <div className="bg-slate-900 border-2 border-rose-500 rounded-2xl w-full max-w-md p-6 space-y-4 shadow-2xl shadow-rose-950/50">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div className="p-2.5 bg-rose-500/20 border border-rose-500/40 rounded-xl text-rose-400">
              <AlertTriangle className="w-6 h-6" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-rose-400 tracking-wide uppercase">
                ⚠️ Lỗi Chiếm Dụng Cổng Endpoint
              </h3>
              <p className="text-xs text-slate-300 font-mono mt-0.5">
                Port: <span className="text-rose-300 font-bold">{bindError.port}</span>
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="space-y-2.5 text-xs text-slate-300 bg-slate-950 p-4 rounded-xl border border-slate-800/80">
          <p>
            Không thể khởi động Proxy Server tại cổng{" "}
            <strong className="text-rose-400 font-mono">{bindError.port}</strong>!
          </p>
          <p className="text-[11px] font-mono text-rose-300/80 bg-rose-950/30 p-2 rounded border border-rose-900/50 break-all">
            {bindError.error}
          </p>
          <div className="pt-1 text-[11px] text-slate-400 space-y-1">
            <p>
              <strong className="text-slate-200">Nguyên nhân:</strong> Có thể bạn đang mở nhiều cửa sổ App cùng lúc, hoặc phần mềm khác đang chiếm cổng này.
            </p>
            <p>
              <strong className="text-slate-200">Cách xử lý:</strong> Tắt các app/cửa sổ trùng lặp hoặc đổi sang Port khác trong tab Endpoints.
            </p>
          </div>
        </div>

        <div className="flex items-center justify-end gap-2 pt-2">
          <button
            onClick={onClose}
            className="px-4 py-2 rounded-xl bg-rose-600 hover:bg-rose-500 text-xs font-bold text-white transition shadow-lg shadow-rose-900/40"
          >
            Đã hiểu & Đóng
          </button>
        </div>
      </div>
    </div>
  );
}
