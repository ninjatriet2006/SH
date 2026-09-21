/*
[INTEGRITY NOTES]
- Mục đích: Thư viện ĐỌC LOG thuần cho transfer + vé params đã đóng dấu.
- Trách nhiệm: Chỉ parse dòng log rclone (`--use-json-log`) → `TransferStats`
  và giữ struct `TransferTicket`; KHÔNG spawn rclone, KHÔNG ráp lệnh, KHÔNG đọc settings.
- Tương tác: `actions::{copy_op, move_op}` parse từng dòng stderr + nhận vé;
  `logic::queue` map `ChildMode` → `TransferMode` rồi gọi actions.
*/
// UNIVERSAL: tracker — nơi DUY NHẤT tính % (đọc log thô → TransferStats).
// Actions chỉ phun dòng thô, queue hỏi tracker rồi ghi vào Job.

use serde::{Deserialize, Serialize};

/// UNIVERSAL: snapshot tiến độ từ object `stats` của rclone
/// (`--use-json-log --stats 0.5s`): % + byte đã chép/tổng + tốc độ + ETA.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransferStats {
    pub percent: u8,
    pub bytes: u64,
    pub total: u64,
    pub speed_bps: f64,
    pub eta_s: Option<u64>,
}

/// UNIVERSAL: chế độ chạy tường minh trên vé (actions chỉ làm theo, không suy).
/// - `Whole`: nguyên khối src→dst một lệnh (kèm `across` + snapshot cờ).
/// - `Item`: một lệnh đơn cho file `rel` tương đối.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferMode {
    Whole,
    Item,
}

/// UNIVERSAL: vé transfer đã đóng dấu — JOB đóng 1 lần lúc dispatch, actions
/// nhận theo giá trị (không đọc engine settings, không AppHandle/State).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferTicket {
    pub src: String,
    pub dst: String,
    /// UNIVERSAL: path tương đối của món (`Item`); `Whole` để rỗng.
    #[serde(default)]
    pub rel: String,
    pub mode: TransferMode,
    /// UNIVERSAL: quyết định across do JOB tính sẵn (chỉ nghĩa khi `Whole`).
    #[serde(default)]
    pub across: bool,
    /// UNIVERSAL: snapshot cờ engine do JOB đóng dấu.
    pub engine_flags: crate::settings::engine::EngineSettings,
    /// UNIVERSAL: chính sách leo thang quyền đóng dấu lúc dispatch.
    pub policy: crate::actions::perm::Policy,
}

/// UNIVERSAL: đọc số nguyên tolerant (rclone có thể nhả float cho byte/count).
fn as_u64(v: &serde_json::Value) -> Option<u64> {
    v.as_u64().or_else(|| {
        v.as_f64()
            .filter(|f| f.is_finite() && *f >= 0.0)
            .map(|f| f as u64)
    })
}

/// UNIVERSAL: đọc số thực tolerant (int lẫn float, loại NaN/inf).
fn as_f64(v: &serde_json::Value) -> Option<f64> {
    v.as_f64()
        .filter(|f| f.is_finite())
        .or_else(|| v.as_u64().map(|u| u as f64))
}

/// UNIVERSAL: bóc object `stats` thành `TransferStats`.
/// Ưu tiên `percentage` (float 0-100), rớt về `bytes/totalBytes` khi thiếu;
/// thiếu cả hai → `None` (không đủ dữ kiện, worker bỏ qua).
/// `speed` thiếu → 0.0; `eta` null/thiếu → `None`.
fn stats_from_value(stats: &serde_json::Value) -> Option<TransferStats> {
    let bytes = stats.get("bytes").and_then(as_u64).unwrap_or(0);
    let total = stats.get("totalBytes").and_then(as_u64).unwrap_or(0);
    let percent = stats
        .get("percentage")
        .and_then(as_f64)
        .map(|p| p.clamp(0.0, 100.0).round() as u8)
        .or_else(|| {
            if total > 0 {
                Some((bytes as f64 / total as f64 * 100.0).clamp(0.0, 100.0).round() as u8)
            } else {
                None
            }
        })?;
    let speed_bps = stats.get("speed").and_then(as_f64).unwrap_or(0.0);
    let eta_s = stats
        .get("eta")
        .and_then(as_f64)
        .filter(|e| *e >= 0.0)
        .map(|e| e.round() as u64);
    Some(TransferStats { percent, bytes, total, speed_bps, eta_s })
}

/// UNIVERSAL: trích % hoàn thành từ object `stats` của rclone (`--use-json-log`).
/// Ưu tiên `percentage` (float 0-100), rớt về `bytes/totalBytes` khi thiếu.
pub fn stats_percent(stats: &serde_json::Value) -> Option<u8> {
    stats_from_value(stats).map(|s| s.percent)
}

/// UNIVERSAL: bóc một dòng log JSON của transfer thành struct đầy đủ
/// (%/byte/tốc độ/ETA) để worker stream tiến độ; dòng không phải stats
/// (log thường, lỗi text) trả `None` để worker bỏ qua.
pub fn parse_stats_line(line: &str) -> Option<TransferStats> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    stats_from_value(v.get("stats")?)
}

/// UNIVERSAL: bóc một dòng log JSON của transfer thành % để map vào `job.progress`;
/// dòng không phải stats (log thường, lỗi text) trả `None` để worker bỏ qua.
pub fn parse_progress_line(line: &str) -> Option<u8> {
    parse_stats_line(line).map(|s| s.percent)
}

/// UNIVERSAL: smart compact — chỉ emit khi % ĐỔI so với lần trước (bỏ spam
/// cùng % từ `--stats 0.5s`); `Tracker::next_stats` dùng nội bộ (queue không
/// gọi trực tiếp hàm này mà qua `Tracker`).
pub fn should_emit_percent(last: &mut Option<u8>, pct: u8) -> bool {
    if *last == Some(pct) {
        return false;
    }
    *last = Some(pct);
    true
}

/// UNIVERSAL: Tracker là nơi DUY NHẤT tính % — bọc `parse_stats_line` +
/// `should_emit_percent`: queue đưa từng dòng thô từ actions vào, nhận về
/// `TransferStats` mới nhất (đã lọc trùng %). Dòng không phải stats → `None`.
#[derive(Debug, Default)]
pub struct Tracker {
    last: Option<u8>,
}

impl Tracker {
    /// UNIVERSAL: Tracker mới — chưa thấy % nào nên dòng stats đầu luôn emit.
    pub fn new() -> Self {
        Self { last: None }
    }

    /// UNIVERSAL: đọc 1 dòng thô → % mới nhất (lọc trùng %); `None` = bỏ qua.
    pub fn next_stats(&mut self, line: &str) -> Option<TransferStats> {
        let st = parse_stats_line(line)?;
        if should_emit_percent(&mut self.last, st.percent) {
            Some(st)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_line_prefers_percentage_and_reads_speed_eta() {
        // UNIVERSAL: có percentage thì ưu tiên, kèm speed/eta đúng kiểu rclone.
        let line = r#"{"level":"info","msg":"","stats":{"bytes":50.0,"totalBytes":200.0,"percentage":33.6,"speed":1024.5,"eta":12.4}}"#;
        let st = parse_stats_line(line).expect("stats line");
        assert_eq!(st.percent, 34);
        assert_eq!((st.bytes, st.total), (50, 200));
        assert!((st.speed_bps - 1024.5).abs() < f64::EPSILON);
        assert_eq!(st.eta_s, Some(12));
    }

    #[test]
    fn stats_line_falls_back_to_bytes_ratio_and_null_eta() {
        // UNIVERSAL: thiếu percentage → bytes/totalBytes; eta null → None.
        let line = r#"{"stats":{"bytes":1.0,"totalBytes":2.0,"speed":0.0,"eta":null}}"#;
        let st = parse_stats_line(line).expect("stats line");
        assert_eq!(st.percent, 50);
        assert_eq!(st.eta_s, None);
        // UNIVERSAL: thiếu cả percentage lẫn total → không đủ dữ kiện.
        assert!(parse_stats_line(r#"{"stats":{"bytes":5.0}}"#).is_none());
    }

    #[test]
    fn progress_line_ignores_non_stats() {
        // UNIVERSAL: dòng log thường / thiếu stats thì bỏ qua, không phá progress.
        assert_eq!(parse_progress_line("not json"), None);
        assert_eq!(parse_progress_line(r#"{"level":"error","msg":"x"}"#), None);
        assert_eq!(stats_percent(&serde_json::json!({"bytes": 50.0, "totalBytes": 200.0})), Some(25));
    }

    #[test]
    fn smart_compact_only_new_percent_emits() {
        // UNIVERSAL: đổi % mới emit; spam cùng % bị bỏ (đỡ persist/event).
        let mut last = None;
        assert!(should_emit_percent(&mut last, 10));
        assert!(!should_emit_percent(&mut last, 10));
        assert!(!should_emit_percent(&mut last, 10));
        assert!(should_emit_percent(&mut last, 11));
        assert!(!should_emit_percent(&mut last, 11));
    }

    #[test]
    fn tracker_reads_raw_lines_to_latest_percent_only() {
        // UNIVERSAL: Tracker đọc log thô → % (nơi duy nhất tính %).
        let mut t = Tracker::new();
        assert!(t.next_stats("not json").is_none());
        assert!(t.next_stats(r#"{"level":"error","msg":"x"}"#).is_none());
        let st = t
            .next_stats(r#"{"stats":{"bytes":50.0,"totalBytes":200.0,"speed":1.0}}"#)
            .expect("first stats emits");
        assert_eq!(st.percent, 25);
        // UNIVERSAL: trùng % bị lọc, % mới mới emit.
        assert!(t.next_stats(r#"{"stats":{"bytes":50.0,"totalBytes":200.0}}"#).is_none());
        let st2 = t
            .next_stats(r#"{"stats":{"bytes":100.0,"totalBytes":200.0}}"#)
            .expect("new percent emits");
        assert_eq!(st2.percent, 50);
    }
}
