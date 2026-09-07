use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

// ============================================================
// ARBITER — trọng tài chấm điểm model
// ============================================================
//
// Nguyên tắc (khác "test bằng chính model đó"):
//   Model TRỌNG TÀI (arbiter) — một model do người dùng chọn — nhận DỮ LIỆU
//   metadata của các model (khả năng khai báo, giới hạn, giá từ models.dev)
//   và chấm điểm số 0-100 theo từng phạm vi (coding, reasoning, tool, vision).
//   Arbiter KHÔNG tự làm bài thi; nó đánh giá dữ liệu.
//
// Không chấm lại từ đầu mỗi lần:
//   Kết quả mỗi lần chạy được LƯU (arbiter.json), giữ tối đa 5 lần gần nhất.
//   Một lần chạy không đủ kết luận → "kết quả cuối cùng" (definitive) của
//   model = phân tích các lần chạy trước CÙNG lần hiện tại:
//   - Lần 1: chính là kết quả lần 1.
//   - Lần N: trung vị từng hạng mục trên các lần 1..N (tối đa 5) — run mới
//     không tự huỷ các run cũ, và run lệch (outlier) bị trung vị triệt tiêu.
//   Độ ổn định (spread = max-min) hiện "ổn định/biên động" để người dùng biết
//   khi nào kết luận đáng tin.

/// Số lần chạy giữ lại cho phân tích đồng thuận.
pub const ARBITER_MAX_RUNS: usize = 5;

/// Điểm một model do arbiter chấm (mỗi hạng mục 0-100).
///
/// Ngoài điểm, arbiter còn báo `context`/`output` mà NÓ TIN là đúng (từ kiến
/// thức huấn luyện) — nguồn ĐỐI CHIẾU độc lập với models.dev và suy đoán theo
/// tên. None = arbiter không chắc → không so bì.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArbiterScores {
    pub coding: u8,
    pub reasoning: u8,
    pub tool_use: u8,
    pub vision: u8,
    pub overall: u8,
    #[serde(default)]
    pub context: Option<u64>,
    #[serde(default)]
    pub output: Option<u64>,
    #[serde(default)]
    pub note: String,
}

/// Một lần chạy arbiter: điểm của mọi model được chấm (khóa "pid/mid").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbiterRun {
    /// Thời điểm chạy (text, hiển thị).
    pub at: String,
    /// Arbiter đã chấm lần này ("pid/mid").
    pub arbiter: String,
    pub scores: HashMap<String, ArbiterScores>,
}

/// Cấu hình + lịch sử arbiter — lưu `~/.config/opencode-manager/arbiter.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArbiterConfig {
    /// Model trọng tài lần dùng gần nhất ("pid/mid") — để form mở ra đúng.
    #[serde(default)]
    pub arbiter: Option<String>,
    /// Các lần chạy gần nhất (mới nhất ở CUỐI; giữ tối đa ARBITER_MAX_RUNS).
    #[serde(default)]
    pub runs: Vec<ArbiterRun>,
}

/// Kết quả cuối cùng của một model sau phân tích đồng thuận.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ArbiterVerdict {
    pub model: String,
    pub coding: u8,
    pub reasoning: u8,
    pub tool_use: u8,
    pub vision: u8,
    pub overall: u8,
    /// Số lần chạy có chấm model này (tối đa ARBITER_MAX_RUNS).
    pub runs: usize,
    /// `true` = điểm ổn định giữa các lần (spread ≤ 10/100).
    pub stable: bool,
    /// Trung vị ước lượng context/output của arbiter qua các lần (None nếu
    /// arbiter chưa từng chắc chắn về model này).
    pub context: Option<u64>,
    pub output: Option<u64>,
    /// Ghi chú của lần chạy MỚI NHẤT (nguyên văn arbiter).
    pub note: String,
}

impl ArbiterConfig {
    /// Vị trí MỚI: `~/.config/opencode-manager/arbiter.json` (dữ liệu riêng
    /// của manager — báo cáo trọng tài không thuộc về opencode).
    pub fn file_path() -> PathBuf {
        crate::storage::manager_data_path("arbiter.json")
    }

    /// Vị trí CŨ (trước khi tách thư mục) — chỉ còn dùng cho migration.
    fn legacy_file_path() -> PathBuf {
        let home = crate::config::get_home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        home.join(".config").join("opencode").join("arbiter.json")
    }

    pub fn load() -> Result<Self, String> {
        // Migration không xoá: copy bản cũ sang vị trí mới, bản cũ thành .legacy.
        crate::storage::migrate_legacy(&Self::legacy_file_path(), &Self::file_path());

        let path = Self::file_path();
        if !path.exists() {
            return Ok(ArbiterConfig::default());
        }
        let text = fs::read_to_string(&path).map_err(|e| format!("Không đọc được arbiter.json: {e}"))?;
        serde_json::from_str(&text).map_err(|e| format!("Lỗi parse arbiter.json: {e}"))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path();
        // Backup xoay vòng + ghi atomic (xem `storage` — chống mất dữ liệu).
        crate::storage::backup_rotate(&path, crate::storage::BACKUP_KEEP);
        let text = serde_json::to_string_pretty(self).map_err(|e| format!("Serialize arbiter lỗi: {e}"))?;
        crate::storage::atomic_write(&path, &text).map_err(|e| format!("Ghi arbiter.json lỗi: {e}"))
    }

    /// Thêm một lần chấm mới (đứng cuối), giữ tối đa ARBITER_MAX_RUNS lần.
    pub fn push_run(&mut self, run: ArbiterRun) {
        self.arbiter = Some(run.arbiter.clone());
        self.runs.push(run);
        let len = self.runs.len();
        if len > ARBITER_MAX_RUNS {
            // Giữ N lần gần nhất (bỏ CŨ nhất ở đầu).
            self.runs.drain(0..len - ARBITER_MAX_RUNS);
        }
    }

    /// KẾT QUẢ CUỐI CÙNG: phân tích các lần chạy (lần mới nhất + các lần trước)
    /// cho mọi model từng được chấm.
    ///
    /// - Lần 1 → chính kết quả đó.
    /// - Lần N (≤5) → trung vị từng hạng mục của N lần; spread lớn → `stable=false`
    ///   (điểm chưa hội tụ, cần chạy thêm để kết luận).
    /// Model có mặt ở một số lần (danh sách model đổi giữa các lần) → chỉ tính
    /// các lần có chấm nó, ghi rõ `runs`.
    pub fn definitive(&self) -> Vec<ArbiterVerdict> {
        let mut models: Vec<&String> = self
            .runs
            .iter()
            .flat_map(|r| r.scores.keys())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        models.sort();

        let mut out = Vec::with_capacity(models.len());
        for mid in models {
            // Điểm từng hạng mục qua các lần có model này.
            let series = |pick: fn(&ArbiterScores) -> u8| -> Vec<u8> {
                self.runs.iter().filter_map(|r| r.scores.get(mid).map(pick)).collect()
            };
            let mut latest_note = String::new();
            for run in self.runs.iter().rev() {
                if let Some(s) = run.scores.get(mid) {
                    latest_note = s.note.clone();
                    break;
                }
            }

            let coding = series(|s| s.coding);
            let reasoning = series(|s| s.reasoning);
            let tool_use = series(|s| s.tool_use);
            let vision = series(|s| s.vision);
            let overall = series(|s| s.overall);
            // Ước lượng limit: chỉ tính các lần arbiter CHẮC (Some).
            let ctx_est: Vec<u64> = self
                .runs
                .iter()
                .filter_map(|r| r.scores.get(mid).and_then(|s| s.context))
                .collect();
            let out_est: Vec<u64> = self
                .runs
                .iter()
                .filter_map(|r| r.scores.get(mid).and_then(|s| s.output))
                .collect();

            // Trung vị của mọi hạng mục phải cùng số lần chạy.
            let runs = overall.len();
            let spread = |v: &Vec<u8>| v.iter().copied().max().unwrap_or(0) - v.iter().copied().min().unwrap_or(0);
            let stable = spread(&coding) <= 10
                && spread(&reasoning) <= 10
                && spread(&tool_use) <= 10
                && spread(&vision) <= 10
                && spread(&overall) <= 10;

            out.push(ArbiterVerdict {
                model: mid.clone(),
                coding: median_u8(&coding),
                reasoning: median_u8(&reasoning),
                tool_use: median_u8(&tool_use),
                vision: median_u8(&vision),
                overall: median_u8(&overall),
                runs,
                stable,
                context: median_opt(&ctx_est),
                output: median_opt(&out_est),
                note: latest_note,
            });
        }
        out
    }
}

/// Trung vị cho Vec<u64> (chẵn số phần tử → trung bình 2 giá trị giữa, làm tròn).
fn median_opt(v: &[u64]) -> Option<u64> {
    if v.is_empty() {
        return None;
    }
    let mut s = v.to_vec();
    s.sort_unstable();
    let n = s.len();
    Some(if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2
    })
}

/// Trung vị (giá trị giữa; chẵn số phần tử thì lấy trung bình 2 giá trị giữa).
fn median_u8(v: &[u8]) -> u8 {
    if v.is_empty() {
        return 0;
    }
    let mut s = v.to_vec();
    s.sort_unstable();
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        // u8 trung bình có thể tròn — đủ dùng cho thang 0-100.
        ((s[n / 2 - 1] as u16 + s[n / 2] as u16) / 2) as u8
    }
}

// ============================================================
// ARBITER CLIENT — gọi model trọng tài qua chat/completions
// ============================================================

/// Dữ liệu một model gửi cho arbiter (dạng gọn để prompt không phình).
#[derive(Debug, Clone, Serialize)]
pub struct ArbiterModelInput {
    /// Khóa "pid/mid" — arbiter trả về đúng khóa này.
    pub id: String,
    pub display_name: String,
    pub tool_call: Option<bool>,
    pub reasoning: Option<bool>,
    pub vision: Option<bool>,
    pub context: Option<u64>,
    pub output: Option<u64>,
    pub price_input: Option<f64>,
    pub price_output: Option<f64>,
}

/// Gọi model trọng tài: gửi metadata các model, nhận điểm.
///
/// Dùng endpoint OpenAI-compatible `/chat/completions` của provider arbiter.
/// Timeout dài (đánh giá nhiều model mất thời gian suy nghĩ).
pub struct ArbiterClient;

const ARBITER_SYSTEM_PROMPT: &str = r#"You are an expert AI-model arbiter ( evaluator). You will receive metadata of several language models (declared capabilities, context/output limits, USD prices per 1M tokens where known). Your job is to evaluate THE DATA and assign scores 0-100 (integers) for each model in these scopes:
- coding: software engineering / code generation & repair ability
- reasoning: logical reasoning, math, long-horizon planning
- tool_use: reliability in agentic tool-calling workflows
- vision: image/multimodal input capability
- overall: your overall judgment for agentic coding use
Base scores on the metadata AND your knowledge of model families/versions. Unknown or undeclared ("null") fields mean "not declared" — do not assume the worst, use your knowledge and the model name. ALSO report, from your own knowledge, the model's context window and max output tokens (null if unsure).
IMPORTANT — score each scope INDEPENDENTLY of context size: judge real-world skill from your knowledge of the model (coding track record, benchmarks like SWE-bench/LiveCodeBench, agentic reliability), NOT from its limits. A 64k-context coding specialist MUST outscore a 1M-context weak generalist on "coding". Limits are already provided as data — never let them inflate or deflate capability scores:
[{"id":"<exact id>","coding":0-100,"reasoning":0-100,"tool_use":0-100,"vision":0-100,"overall":0-100,"context":<int or null>,"output":<int or null>,"note":"<max 15 words justification>"}]
No markdown, no commentary, JSON array only."#;

/// Giới hạn số model mỗi lần chấm — prompt quá dài làm arbiter cắt bớt đầu ra
/// (model cuối mất điểm) và tốn token vô ích; chia lô nếu cần.
pub const ARBITER_BATCH_LIMIT: usize = 40;

impl ArbiterClient {
    /// Chấm một lô model. Trả về map "pid/mid" → điểm (chỉ model arbiter trả
    /// lời hợp lệ; model bị bỏ sót sẽ không có trong map).
    pub async fn evaluate(
        base_url: &str,
        api_key: &str,
        arbiter_model: &str,
        batch: &[ArbiterModelInput],
    ) -> Result<HashMap<String, ArbiterScores>, String> {
        if base_url.trim().is_empty() || api_key.trim().is_empty() {
            return Err("Arbiter provider thiếu Base URL hoặc API key.".to_string());
        }
        let url = format!("{}/chat/completions", base_url.trim().trim_end_matches('/'));

        let user_payload = serde_json::to_string(batch).map_err(|e| format!("Serialize dữ liệu model lỗi: {e}"))?;

        let body = serde_json::json!({
            "model": arbiter_model,
            "temperature": 0.2,
            "messages": [
                { "role": "system", "content": ARBITER_SYSTEM_PROMPT },
                { "role": "user", "content": user_payload },
            ],
        });

        let client = Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|e| format!("Tạo HTTP client lỗi: {e}"))?;

        let resp = client
            .post(&url)
            .bearer_auth(api_key.trim())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    "Arbiter phản hồi quá hạn (180s) — thử ít model hơn hoặc arbiter nhanh hơn.".to_string()
                } else {
                    format!("Gọi arbiter lỗi: {e}")
                }
            })?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| {
                    v.get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .map(String::from)
                })
                .unwrap_or_default();
            return Err(format!(
                "Arbiter trả HTTP {}: {}",
                status.as_u16(),
                if msg.is_empty() {
                    "(không đọc được thông báo)"
                } else {
                    &msg
                }
            ));
        }

        // choices[0].message.content → JSON array. Một số model reasoning trả
        // content: null (lý do nằm ở trường khác) — dùng Value rồi stringify
        // thay vì String để không crash parse.
        #[derive(Deserialize)]
        struct ChatResp {
            #[serde(default)]
            choices: Vec<ChatChoice>,
        }
        #[derive(Deserialize)]
        struct ChatChoice {
            message: ChatMessage,
        }
        #[derive(Deserialize)]
        struct ChatMessage {
            #[serde(default)]
            content: Option<serde_json::Value>,
        }
        let chat: ChatResp = serde_json::from_str(&text).map_err(|e| format!("Parse phản hồi arbiter lỗi: {e}"))?;
        let content = chat
            .choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .and_then(extract_content_text)
            .ok_or_else(|| {
                "Arbiter không trả nội dung dạng text (content rỗng/null — thử model trọng tài khác).".to_string()
            })?;

        parse_arbiter_reply(&content)
    }
}

/// Parse câu trả lời arbiter: tìm JSON array (bỏ rào markdown nếu có), ép điểm
/// về 0-100, bỏ object thiếu id.
/// Bóc khối JSON khỏi rào markdown ``` (nếu có); không có rào → nguyên bản.
fn strip_code_fence(s: &str) -> &str {
    let t = s.trim();
    let Some(start) = t.find("```") else { return t };
    let after = &t[start + 3..];
    // Bỏ dòng "```json" (ngôn ngữ) — phần thân bắt đầu từ dòng kế.
    let after = after.split_once('\n').map(|(_, rest)| rest).unwrap_or(after);
    let end = after.rfind("```").unwrap_or(after.len());
    after[..end].trim()
}

/// Cắt đoạn từ ký tự MỞ đầu tiên tới ký tự ĐÓNG cuối cùng (bao cả 2 ký tự).
fn slice_between(s: &str, open: char, close: char) -> Option<&str> {
    let a = s.find(open)?;
    let b = s.rfind(close)?;
    (a < b).then(|| &s[a..=b])
}

/// Parse câu trả lời arbiter. Chấp nhận 3 dạng — arbiter thật (đã gặp) trả
/// lộn xộn đủ kiểu:
///   1. MẢNG object (hợp đồng prompt: `[{"id":...}, ...]`).
///   2. MỘT object đơn lẻ (map) — bug người dùng gặp thật: "invalid type:
///      map, expected a sequence" khi arbiter chỉ chấm 1 model.
///   3. JSONL — mỗi dòng một object (một số model trả từng dòng, có thể kèm
///      dấu phẩy cuối dòng; cũng cứu được mảng pretty-print bị hỏng 1 phần tử).
pub fn parse_arbiter_reply(content: &str) -> Result<HashMap<String, ArbiterScores>, String> {
    #[derive(Deserialize)]
    struct RawScore {
        id: String,
        #[serde(default)]
        coding: i32,
        #[serde(default)]
        reasoning: i32,
        #[serde(default)]
        tool_use: i32,
        #[serde(default)]
        vision: i32,
        #[serde(default)]
        overall: i32,
        /// Nhận lỏng lẻo (Value): float kiểu 128000.5 không được làm SỤP cả
        /// mảng parse — chỉ số nguyên mới tính là "chắc", còn lại → None.
        #[serde(default)]
        context: Option<serde_json::Value>,
        #[serde(default)]
        output: Option<serde_json::Value>,
        #[serde(default)]
        note: String,
    }

    let clamp = |v: i32| -> u8 { v.clamp(0, 100) as u8 };
    let collect = |list: Vec<RawScore>| -> Option<HashMap<String, ArbiterScores>> {
        let mut out = HashMap::new();
        for r in list {
            if r.id.trim().is_empty() {
                continue;
            }
            out.insert(
                r.id.trim().to_string(),
                ArbiterScores {
                    coding: clamp(r.coding),
                    reasoning: clamp(r.reasoning),
                    tool_use: clamp(r.tool_use),
                    vision: clamp(r.vision),
                    overall: clamp(r.overall),
                    context: r.context.as_ref().and_then(serde_json::Value::as_u64),
                    output: r.output.as_ref().and_then(serde_json::Value::as_u64),
                    note: r.note,
                },
            );
        }
        (!out.is_empty()).then_some(out)
    };

    let body = strip_code_fence(content);

    // 1. Mảng (đúng hợp đồng) — gồm cả mảng quấn trong text thừa.
    if let Some(arr) = slice_between(body, '[', ']') {
        if let Ok(list) = serde_json::from_str::<Vec<RawScore>>(arr) {
            if let Some(m) = collect(list) {
                return Ok(m);
            }
        }
    }

    // 2. Một object đơn lẻ (kể cả pretty-print nhiều dòng).
    if let Some(obj) = slice_between(body, '{', '}') {
        if let Ok(one) = serde_json::from_str::<RawScore>(obj) {
            if let Some(m) = collect(vec![one]) {
                return Ok(m);
            }
        }
    }

    // 3. JSONL — mỗi dòng một object (bỏ dấu phẩy cuối dòng).
    let lines: Vec<RawScore> = body
        .lines()
        .filter_map(|l| {
            let l = l.trim().trim_end_matches(',');
            if !(l.starts_with('{') && l.ends_with('}')) {
                return None;
            }
            serde_json::from_str::<RawScore>(l).ok()
        })
        .collect();
    if let Some(m) = collect(lines) {
        return Ok(m);
    }

    Err(format!(
        "Arbiter không trả JSON hợp lệ (thử mảng / object đơn / từng dòng đều thất bại; nội dung đầu: {:?})",
        content.chars().take(80).collect::<String>()
    ))
}

/// Bóc text từ trường `content` của chat: chuỗi thường, hoặc mảng chunks
/// (dạng hiếm) → ghép lại. Null/kiểu khác → None (model reasoning trả
/// content: null với lý do ở trường khác — báo lỗi thân thiện thay vì crash).
fn extract_content_text(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(parts) => {
            let joined = parts.iter().filter_map(|p| p.as_str()).collect::<Vec<_>>().join("");
            (!joined.is_empty()).then_some(joined)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scores(c: u8, r: u8, t: u8, v: u8, o: u8) -> ArbiterScores {
        ArbiterScores {
            coding: c,
            reasoning: r,
            tool_use: t,
            vision: v,
            overall: o,
            context: None,
            output: None,
            note: String::new(),
        }
    }

    fn scores_with_limits(c: u8, r: u8, t: u8, v: u8, o: u8, ctx: Option<u64>, out: Option<u64>) -> ArbiterScores {
        ArbiterScores {
            coding: c,
            reasoning: r,
            tool_use: t,
            vision: v,
            overall: o,
            context: ctx,
            output: out,
            note: String::new(),
        }
    }

    /// Lần 1 → kết quả cuối = chính kết quả lần 1 (nguyên tắc "lần đầu lấy
    /// luôn kết quả đầu").
    #[test]
    fn lan_mot_lay_nguyen_ket_qua_dau() {
        let mut cfg = ArbiterConfig::default();
        let mut run = ArbiterRun {
            at: "t1".into(),
            arbiter: "p/judge".into(),
            scores: HashMap::new(),
        };
        run.scores.insert("p/alpha".into(), scores(80, 70, 60, 50, 75));
        cfg.push_run(run);

        let v = cfg.definitive();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].model, "p/alpha");
        assert_eq!(v[0].coding, 80);
        assert_eq!(v[0].overall, 75);
        assert_eq!(v[0].runs, 1);
        assert!(v[0].stable, "một lần chạy không có gì so sánh → coi như ổn định");
    }

    /// Lần 5: 4 lần trước CÙNG lần 5 → trung vị từng hạng mục. Outlier bị
    /// triệt tiêu (90,10,80,85,95 → 85 chứ không phải 90).
    #[test]
    fn lan_nam_trung_vi_hang_muc() {
        let mut cfg = ArbiterConfig::default();
        let vals = [(90, 10), (80, 70), (85, 60), (95, 80), (90, 75)];
        for (i, (coding, overall)) in vals.iter().enumerate() {
            let mut run = ArbiterRun {
                at: format!("t{}", i + 1),
                arbiter: "p/judge".into(),
                scores: HashMap::new(),
            };
            run.scores
                .insert("p/alpha".into(), scores(*coding, 50, 50, 50, *overall));
            cfg.push_run(run);
        }
        assert_eq!(cfg.runs.len(), 5, "giữ đúng 5 lần");

        let v = &cfg.definitive()[0];
        // coding [90,80,85,95,90] sort → [80,85,90,90,95] → median 90.
        assert_eq!(v.coding, 90);
        // overall [10,70,60,80,75] sort → [10,60,70,75,80] → median 70 (lần 1
        // lệch 10 không kéo kết luận).
        assert_eq!(v.overall, 70);
        assert_eq!(v.runs, 5);
        assert!(!v.stable, "spread coding 15 > 10 → chưa ổn định");
    }

    /// Quá 5 lần → bỏ CŨ nhất, phân tích chỉ trên 5 lần gần nhất.
    #[test]
    fn qua_nam_lan_bo_cu_nhat() {
        let mut cfg = ArbiterConfig::default();
        for i in 0..7 {
            let mut run = ArbiterRun {
                at: format!("t{}", i + 1),
                arbiter: "p/j".into(),
                scores: HashMap::new(),
            };
            // Lần cũ (0..2) chấm 100; lần mới (3..6) chấm 50 — nếu không bỏ
            // cũ, trung vị sẽ sai.
            let val = if i < 2 { 100 } else { 50 };
            run.scores.insert("p/m".into(), scores(val, val, val, val, val));
            cfg.push_run(run);
        }
        assert_eq!(cfg.runs.len(), ARBITER_MAX_RUNS);
        let v = &cfg.definitive()[0];
        // 5 lần gần nhất đều 50 (i=3..6 là 4 lần 50 + i=2 cũng 50) → 50.
        assert_eq!(v.coding, 50);
        assert_eq!(v.runs, 5);
    }

    /// Model xuất hiện ở một số lần (danh sách model đổi giữa các lần) → chỉ
    /// tính các lần có chấm nó.
    #[test]
    fn model_mat_o_mot_so_lan() {
        let mut cfg = ArbiterConfig::default();
        for i in 0..3 {
            let mut run = ArbiterRun {
                at: format!("t{}", i + 1),
                arbiter: "p/j".into(),
                scores: HashMap::new(),
            };
            // p/old chỉ có ở lần 1; p/new chỉ có ở lần 2,3.
            if i == 0 {
                run.scores.insert("p/old".into(), scores(40, 40, 40, 40, 40));
            }
            run.scores.insert("p/new".into(), scores(80, 80, 80, 80, 90));
            cfg.push_run(run);
        }
        let mut v = cfg.definitive();
        v.sort_by(|a, b| a.model.cmp(&b.model));
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].model, "p/new");
        assert_eq!(v[0].runs, 3, "p/new có ở cả 3 lần");
        assert_eq!(v[0].overall, 90);
        assert_eq!(v[1].model, "p/old");
        assert_eq!(v[1].runs, 1, "p/old chỉ lần 1");
        // Ghi chú lấy từ lần MỚI NHẤT có model đó.
        assert!(!v[1].note.is_empty() || v[1].note.is_empty()); // note rỗng trong test này
    }

    /// Parse JSON thuần và JSON bọc markdown.
    #[test]
    fn parse_json_thuan_va_markdown() {
        let raw =
            r#"[{"id":"p/a","coding":88,"reasoning":72,"tool_use":60,"vision":0,"overall":80,"note":"mạnh code"}]"#;
        let m = parse_arbiter_reply(raw).unwrap();
        assert_eq!(m["p/a"].coding, 88);
        assert_eq!(m["p/a"].overall, 80);

        let fenced = format!("```json\n{raw}\n```");
        let m2 = parse_arbiter_reply(&fenced).unwrap();
        assert_eq!(m2["p/a"].coding, 88);

        // Điểm ngoài thang bị ép về 0-100; id rỗng bị bỏ.
        let clamped = r#"[{"id":"p/b","coding":150,"reasoning":-5,"tool_use":50,"vision":50,"overall":50},{"id":"","coding":10}]"#;
        let m3 = parse_arbiter_reply(clamped).unwrap();
        assert_eq!(m3["p/b"].coding, 100);
        assert_eq!(m3["p/b"].reasoning, 0);
        assert_eq!(m3.len(), 1, "id rỗng bị bỏ");

        assert!(parse_arbiter_reply("không phải json").is_err());
    }

    /// content null / mảng chunks / kiểu lạ — không crash, trả None hoặc text
    /// ghép đúng (hồi quy crash "invalid type: null" với model reasoning).
    #[test]
    fn bóc_content_null_mang_va_string() {
        use serde_json::json;
        assert_eq!(extract_content_text(&json!("hello")), Some("hello".to_string()));
        assert_eq!(extract_content_text(&json!(null)), None);
        assert_eq!(extract_content_text(&json!({})), None);
        assert_eq!(extract_content_text(&json!(42)), None);
        // Mảng chunks → ghép.
        assert_eq!(extract_content_text(&json!(["a", "b"])), Some("ab".to_string()));
        // Mảng không phải text → rỗng → None.
        assert_eq!(extract_content_text(&json!([1, 2])), None);
    }

    /// Ước lượng limit của arbiter qua các lần: trung vị các lần CHẮC (Some);
    /// lần không chắc (None) không kéo kết quả.
    #[test]
    fn limit_uoc_luong_trung_vi_cac_lan_chac() {
        let mut cfg = ArbiterConfig::default();
        // Lần 1: chắc ctx 128k; lần 2: không chắc; lần 3: chắc 200k; lần 4-5: chắc.
        let runs: [Option<u64>; 5] = [Some(128_000), None, Some(200_000), Some(128_000), Some(129_000)];
        for (i, ctx) in runs.iter().enumerate() {
            let mut run = ArbiterRun {
                at: format!("t{}", i + 1),
                arbiter: "p/j".into(),
                scores: HashMap::new(),
            };
            run.scores
                .insert("p/m".into(), scores_with_limits(50, 50, 50, 50, 50, *ctx, Some(8_192)));
            cfg.push_run(run);
        }
        let v = &cfg.definitive()[0];
        // Các lần chắc: [128k, 200k, 128k, 129k] → sort [128k,128k,129k,200k]
        // → trung vị (128k+129k)/2.
        assert_eq!(v.context, Some(128_500));
        assert_eq!(v.output, Some(8_192));
    }

    /// Parse limit từ JSON arbiter: số nguyên OK; float/chuỗi → None.
    #[test]
    fn parse_limit_so_nguyen_va_bo_float() {
        let raw = r#"[
            {"id":"p/a","coding":80,"reasoning":70,"tool_use":60,"vision":50,"overall":75,"context":128000,"output":8192},
            {"id":"p/b","coding":80,"reasoning":70,"tool_use":60,"vision":50,"overall":75,"context":128000.5,"output":"8192"}
        ]"#;
        let m = parse_arbiter_reply(raw).unwrap();
        assert_eq!(m["p/a"].context, Some(128_000));
        assert_eq!(m["p/a"].output, Some(8_192));
        assert_eq!(m["p/b"].context, None, "float → không chắc");
        assert_eq!(m["p/b"].output, None, "chuỗi → không chắc");
    }

    /// HỒI QUY bug người dùng gặp: arbiter trả MỘT object đơn (map) thay vì
    /// mảng → lỗi "invalid type: map, expected a sequence". Phải chấp nhận.
    #[test]
    fn parse_object_don_le_hoi_quy_bug_that() {
        let raw = r#"{"id":"openrouter/moominotai/kimi-k2-0905","coding":78,"reasoning":72,"tool_use":70,"vision":10,"overall":74,"context":262144,"output":16384,"note":"k2 coder"}"#;
        let m = parse_arbiter_reply(raw).unwrap();
        assert_eq!(m.len(), 1);
        let sc = &m["openrouter/moominotai/kimi-k2-0905"];
        assert_eq!(sc.coding, 78);
        assert_eq!(sc.overall, 74);
        assert_eq!(sc.context, Some(262_144));
        assert_eq!(sc.output, Some(16_384));

        // Cùng object đó bọc markdown fence.
        let fenced = format!("```json\n{raw}\n```");
        let m2 = parse_arbiter_reply(&fenced).unwrap();
        assert_eq!(m2["openrouter/moominotai/kimi-k2-0905"].coding, 78);

        // Object pretty-print nhiều dòng.
        let pretty = "{\n  \"id\": \"p/x\",\n  \"coding\": 50\n}";
        let m3 = parse_arbiter_reply(pretty).unwrap();
        assert_eq!(m3["p/x"].coding, 50);
        assert_eq!(m3["p/x"].reasoning, 0, "thiếu field → default 0");
    }

    /// JSONL: mỗi dòng một object (kèm dấu phẩy cuối dòng) — một số model trả
    /// từng dòng thay vì mảng; cũng cứu mảng pretty-print hỏng một phần tử.
    #[test]
    fn parse_jsonl_tung_dong() {
        let jsonl = concat!(
            "{\"id\":\"p/a\",\"coding\":80,\"reasoning\":70,\"tool_use\":60,\"vision\":50,\"overall\":75},\n",
            "{\"id\":\"p/b\",\"coding\":60,\"reasoning\":55,\"tool_use\":50,\"vision\":40,\"overall\":58},"
        );
        let m = parse_arbiter_reply(jsonl).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m["p/a"].overall, 75);
        assert_eq!(m["p/b"].coding, 60);

        // Mảng pretty-print có một PHẦN TỬ hỏng (thiếu ngoặc) → nhánh mảng
        // thất bại, nhánh JSONL cứu được các dòng lành.
        let broken = concat!(
            "[\n",
            "  {\"id\":\"p/ok\",\"coding\":80,\"reasoning\":70,\"tool_use\":60,\"vision\":50,\"overall\":75},\n",
            "  {\"id\":\"p/bad\",\"coding\":60\n",
            "]"
        );
        let m2 = parse_arbiter_reply(broken).unwrap();
        assert_eq!(m2.len(), 1, "chỉ dòng lành được cứu");
        assert_eq!(m2["p/ok"].coding, 80);
    }

    /// Trung vị chẵn số phần tử.
    #[test]
    fn trung_vi_chan() {
        assert_eq!(median_u8(&[10, 20]), 15);
        assert_eq!(median_u8(&[1, 2, 3, 4]), 2); // (2+3)/2 = 2.5 → 2
        assert_eq!(median_u8(&[]), 0);
        assert_eq!(median_u8(&[7]), 7);
    }

    /// Round-trip file arbiter.json.
    #[test]
    fn roundtrip_arbiter_json() {
        // Ghi file theo home → dùng chung khoá env test của app để không cướp
        // home của test khác khi chạy song song.
        let _guard = crate::app::TEST_ENV_LOCK.lock().unwrap();
        let dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("arbiter_test_home");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // SAFETY: test function trong môi trường kiểm soát
        unsafe {
            std::env::set_var("OPENCODE_TEST_HOME", &dir);
        }

        let mut cfg = ArbiterConfig::default();
        let mut run = ArbiterRun {
            at: "t".into(),
            arbiter: "p/j".into(),
            scores: HashMap::new(),
        };
        run.scores.insert("p/m".into(), scores(11, 22, 33, 44, 55));
        cfg.push_run(run);
        cfg.save().unwrap();

        let loaded = ArbiterConfig::load().unwrap();
        assert_eq!(loaded.arbiter.as_deref(), Some("p/j"));
        assert_eq!(loaded.runs.len(), 1);
        assert_eq!(loaded.runs[0].scores["p/m"].overall, 55);

        let _ = fs::remove_dir_all(&dir);
    }
}
