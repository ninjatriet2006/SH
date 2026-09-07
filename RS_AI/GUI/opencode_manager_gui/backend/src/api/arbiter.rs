/*
[INTEGRITY NOTES]
- Mục đích: ARBITER — model trọng tài chấm điểm các model theo dữ liệu metadata
  (khả năng khai báo + giá/limit đã làm giàu), lưu kết quả theo lần chạy và
  phân tích đồng thuận 5 lần gần nhất.
- Trách nhiệm: `get_arbiter_state` (đọc, không gọi mạng), `run_arbiter_evaluation`
  (gọi arbiter qua chat/completions của provider được chọn, chia lô nếu nhiều
  model), `clear_arbiter_history`.
- Tương tác: `api::models::matrix_rows` (dữ liệu đầu vào), crate
  `opencode_manager::arbiter` (client + lịch sử + đồng thuận), frontend
  `pages/ModelsPage.tsx`.

Nguyên tắc đồng thuận (theo thiết kế):
  - Lần 1 → kết quả cuối = kết quả lần 1.
  - Lần N (giữ tối đa 5) → trung vị từng hạng mục của N lần gần nhất; spread
    lớn → đánh dấu chưa ổn định (cần chạy thêm).
  - Chấm điểm bởi ARBITER, không phải model tự làm bài: arbiter nhận metadata
    và xuất điểm số 0-100 từng phạm vi.
*/

use crate::api::models::matrix_rows;
use opencode_manager::app::App;
use opencode_manager::arbiter::{
    ArbiterClient, ArbiterConfig, ArbiterModelInput, ArbiterRun, ArbiterVerdict, ARBITER_BATCH_LIMIT,
};
use serde::Serialize;

/// Trạng thái arbiter cho UI (đọc từ arbiter.json — không gọi mạng).
#[derive(Debug, Clone, Serialize)]
pub struct ArbiterState {
    /// Model trọng tài lần dùng gần nhất ("pid/mid"); None = chưa từng chạy.
    pub arbiter: Option<String>,
    /// Số lần chạy đang giữ (≤ 5).
    pub run_count: usize,
    /// Lần chạy cuối (thời điểm text + arbiter) — null nếu chưa từng chạy.
    pub last_run: Option<ArbiterLastRun>,
    /// Kết quả cuối cùng sau đồng thuận (rỗng nếu chưa từng chạy).
    pub verdicts: Vec<ArbiterVerdict>,
    /// Danh sách model đủ điều kiện làm trọng tài (đang có trong config).
    pub candidates: Vec<ArbiterCandidate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArbiterLastRun {
    pub at: String,
    pub arbiter: String,
}

/// Một lựa chọn trọng tài trong dropdown.
#[derive(Debug, Clone, Serialize)]
pub struct ArbiterCandidate {
    pub provider_id: String,
    pub model_id: String,
    /// Mặc định gợi ý: đang là model chính.
    pub is_primary: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_arbiter_state() -> Result<ArbiterState, String> {
    let cfg = ArbiterConfig::load()?;
    let rows = matrix_rows()?;

    let candidates: Vec<ArbiterCandidate> = rows
        .iter()
        .map(|r| ArbiterCandidate {
            provider_id: r.provider_id.clone(),
            model_id: r.model_id.clone(),
            is_primary: r.is_primary,
        })
        .collect();

    let last_run = cfg.runs.last().map(|run| ArbiterLastRun {
        at: run.at.clone(),
        arbiter: run.arbiter.clone(),
    });

    Ok(ArbiterState {
        arbiter: cfg.arbiter.clone(),
        run_count: cfg.runs.len(),
        last_run,
        verdicts: cfg.definitive(),
        candidates,
    })
}

/// Sự kiện tiến trình chấm — frontend `listen('arbiter://progress')` để hiện
/// log/progress thật trong lúc đánh giá (nhiều lô, mỗi lô một request).
#[derive(Clone, Serialize)]
pub struct ArbiterProgress {
    /// "start" | "batch" | "scored" | "warn" | "error" | "done"
    pub stage: String,
    pub message: String,
    /// Lô hiện tại (1-based; 0 khi chưa bắt đầu lô nào).
    pub current: usize,
    /// Tổng số lô.
    pub total: usize,
    /// Số model đã chấm tích luỹ.
    pub scored: usize,
}

fn emit_progress(app: &tauri::AppHandle, stage: &str, message: String, current: usize, total: usize, scored: usize) {
    use tauri::Emitter;
    let _ = app.emit(
        "arbiter://progress",
        ArbiterProgress {
            stage: stage.to_string(),
            message,
            current,
            total,
            scored,
        },
    );
}

/// Chạy arbiter: chọn model trọng tài bằng (`arbiter_provider`, `arbiter_model`)
/// — phải là một model đang có trong config.
///
/// Trả về kết quả cuối cùng sau khi cộng dồn lần chạy mới vào lịch sử.
///
/// ASYNC + spawn_blocking: lệnh chạy lâu (nhiều lô request, timeout 180s/lô)
/// — nếu để đồng bộ trên main thread thì UI ĐÓNG BĂNG, spinner không nhấp
/// nháy và event không tới được webview. Toàn bộ tiến trình được phát qua
/// event `arbiter://progress` (frontend hiện log từng lô + lỗi từng lô).
#[tauri::command(rename_all = "snake_case")]
pub async fn run_arbiter_evaluation(
    app: tauri::AppHandle,
    arbiter_provider: String,
    arbiter_model: String,
) -> Result<Vec<ArbiterVerdict>, String> {
    tauri::async_runtime::spawn_blocking(move || run_arbiter_evaluation_blocking(app, arbiter_provider, arbiter_model))
        .await
        .map_err(|e| format!("Task trọng tài sụp: {e}"))?
}

fn run_arbiter_evaluation_blocking(
    app: tauri::AppHandle,
    arbiter_provider: String,
    arbiter_model: String,
) -> Result<Vec<ArbiterVerdict>, String> {
    let arbiter_provider = arbiter_provider.trim().to_string();
    let arbiter_model = arbiter_model.trim().to_string();
    if arbiter_provider.is_empty() || arbiter_model.is_empty() {
        return Err("Vui lòng chọn model trọng tài.".to_string());
    }

    // Lấy base_url + api_key của provider trọng tài từ config đã merge.
    let presets = App::load_dynamic_presets();
    let (config, _auth) = crate::core::store::load_merged(&presets)?;
    let provider = config
        .provider
        .get(&arbiter_provider)
        .ok_or_else(|| format!("Không tìm thấy provider trọng tài '{arbiter_provider}'."))?;
    // Trọng tài hợp lệ: model đã khai, HOẶC model catalogue của provider
    // builtin key-only (opencode tự resolve model đó khi gọi).
    let in_catalog = provider.models.is_empty()
        && opencode_manager::model_knowledge::catalog_models_by_provider()
            .get(arbiter_provider.as_str())
            .map(|list| list.iter().any(|(m, _)| *m == arbiter_model))
            .unwrap_or(false);
    if !provider.models.contains_key(&arbiter_model) && !in_catalog {
        return Err(format!(
            "Model trọng tài '{arbiter_model}' chưa có trong provider '{arbiter_provider}'."
        ));
    }
    let base_url = provider.options.base_url.trim().to_string();
    let api_key = provider.options.api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(format!(
            "Provider '{arbiter_provider}' chưa có API key — không gọi được model trọng tài."
        ));
    }

    // Dữ liệu để chấm = ma trận model (metadata + giá đã làm giàu).
    // Arbiter tự chấm chính nó cũng được (không loại) — trọng tài chỉ ĐÁNH GIÁ
    // dữ liệu, không làm bài thi.
    emit_progress(&app, "start", "Đang thu thập dữ liệu model...".to_string(), 0, 0, 0);
    let rows = matrix_rows()?;
    if rows.is_empty() {
        return Err("Chưa có model nào trong config để chấm.".to_string());
    }

    let batch: Vec<ArbiterModelInput> = rows
        .iter()
        .map(|r| ArbiterModelInput {
            id: format!("{}/{}", r.provider_id, r.model_id),
            display_name: r.display_name.clone(),
            tool_call: r.tool_call,
            reasoning: r.reasoning,
            vision: r.vision,
            context: r.context,
            output: r.output,
            price_input: r.price_input,
            price_output: r.price_output,
        })
        .collect();

    // Id hợp lệ = id ta đã gửi (arbiter có thể "ảo tưởng" ra id lạ — lọc bỏ
    // để verdicts không xuất hiện model ma).
    let sent_ids: std::collections::HashSet<String> = batch.iter().map(|m| m.id.clone()).collect();

    // Gọi theo lô (prompt quá dài làm arbiter bỏ sót model cuối). Lô nào lỗi
    // thì GHI LOG rồi CHẠI TIẾP các lô sau — một lô hỏng không đốt cả lần
    // đánh giá; chỉ khi TẤT CẢ đều hỏng mới trả lỗi.
    // MỘT runtime cho cả vòng lặp (dựng per-lô là phí thread).
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| format!("Không tạo được runtime đánh giá: {e}"))?;
    let mut merged = std::collections::HashMap::new();
    let mut last_err: Option<String> = None;
    let total_batches = batch.len().div_ceil(ARBITER_BATCH_LIMIT);
    emit_progress(
        &app,
        "start",
        format!(
            "Chấm {} model bằng '{}' qua {} lô request...",
            batch.len(),
            arbiter_model,
            total_batches
        ),
        0,
        total_batches,
        0,
    );
    for (i, chunk) in batch.chunks(ARBITER_BATCH_LIMIT).enumerate() {
        emit_progress(
            &app,
            "batch",
            format!("Lô {}/{}: gửi {} model...", i + 1, total_batches, chunk.len()),
            i + 1,
            total_batches,
            merged.len(),
        );
        match rt.block_on(ArbiterClient::evaluate(&base_url, &api_key, &arbiter_model, chunk)) {
            Ok(part) => {
                let dropped = part.keys().filter(|k| !sent_ids.contains(*k)).count();
                if dropped > 0 {
                    emit_progress(
                        &app,
                        "warn",
                        format!("Bỏ {dropped} id lạ ngoài danh sách đã gửi."),
                        i + 1,
                        total_batches,
                        merged.len(),
                    );
                }
                merged.extend(part.into_iter().filter(|(k, _)| sent_ids.contains(k)));
                emit_progress(
                    &app,
                    "scored",
                    format!("Lô {}/{} xong — đã chấm {} model.", i + 1, total_batches, merged.len()),
                    i + 1,
                    total_batches,
                    merged.len(),
                );
            }
            Err(e) => {
                last_err = Some(e.clone());
                emit_progress(
                    &app,
                    "error",
                    format!("Lô {}/{} LỖI: {}", i + 1, total_batches, e),
                    i + 1,
                    total_batches,
                    merged.len(),
                );
            }
        }
    }
    if merged.is_empty() {
        return Err(last_err.unwrap_or_else(|| "Arbiter không chấm được model nào.".to_string()));
    }

    // Ghi lần chạy mới vào lịch sử (giữ tối đa 5 lần).
    // `scored` của lần chạy NÀY = số model arbiter vừa chấm; verdicts là kết
    // quả đồng thuận (gồm cả model của các lần cũ) — hai số khác nhau, tách riêng.
    let scored = merged.len();
    let mut cfg = ArbiterConfig::load()?;
    cfg.push_run(ArbiterRun {
        at: chrono_text_now(),
        arbiter: format!("{arbiter_provider}/{arbiter_model}"),
        scores: merged,
    });
    cfg.save()?;

    let verdicts = cfg.definitive();
    if let Some(e) = last_err {
        emit_progress(
            &app,
            "done",
            format!("Hoàn tất một phần: {} model được chấm (có lô lỗi: {e}).", scored),
            total_batches,
            total_batches,
            scored,
        );
    } else {
        emit_progress(
            &app,
            "done",
            format!("Hoàn tất: {} model đã chấm.", scored),
            total_batches,
            total_batches,
            scored,
        );
    }
    Ok(verdicts)
}

fn chrono_text_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Thời điểm text ngắn gọn (epoch → dd/mm/yyyy HH:MM UTC, không cần chrono).
    chrono_ts_text(secs)
}

/// Format timestamp epoch → "dd/mm/yyyy HH:MM" (UTC) — tránh phụ thuộc timezone
/// của máy test; đủ để hiển thị "lần chạy lúc nào".
fn chrono_ts_text(secs: u64) -> String {
    // Ngày Julian đơn giản: chia tay từ epoch (86400s/ngày).
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, m) = (rem / 3_600, (rem % 3_600) / 60);
    // Chuyển ngày → dd/mm/yyyy (công thức Howard Hinnant, days_from_civil đảo).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mth <= 2 { y + 1 } else { y };
    format!("{d:02}/{mth:02}/{y:04} {h:02}:{m:02}")
}

/// Xoá lịch sử chấm (bắt đầu lại từ đầu — lần chạy kế tiếp là "lần 1").
#[tauri::command(rename_all = "snake_case")]
pub fn clear_arbiter_history() -> Result<(), String> {
    let mut cfg = ArbiterConfig::load()?;
    cfg.runs.clear();
    cfg.arbiter = None;
    cfg.save()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chuyển epoch → dd/mm/yyyy HH:MM đúng với mốc đã kiểm bằng Python.
    #[test]
    fn epoch_sang_ngay_gio() {
        // 0 → 01/01/1970 00:00.
        assert_eq!(chrono_ts_text(0), "01/01/1970 00:00");
        // 86_400 = đúng 1 ngày.
        assert_eq!(chrono_ts_text(86_400), "02/01/1970 00:00");
        // 2024-02-29 (năm nhuận) 12:30 UTC = 1709209800.
        assert_eq!(chrono_ts_text(1_709_209_800), "29/02/2024 12:30");
        // 2025-12-31 23:59 UTC = 1767225540 (cuối năm).
        assert_eq!(chrono_ts_text(1_767_225_540), "31/12/2025 23:59");
    }
}

// ============================================================
// GỢI Ý THEO TÁC VỤ (task-first ranking)
// ============================================================
//
// Triết lý: chọn model theo TÁC VỤ ("tôi cần code") chứ không theo thông số
// thô (context to cỡ nào). Ma trận metadata là bảng thông số; ranking này là
// "leaderboard theo nhiệm vụ":
//   - Điểm CHÍNH của judge cho phạm vi tác vụ THẮNG (dominant).
//   - Context chỉ là BỘ ĐIỀU CHỈNH có chặn (±10) — model chuyên coding context
//     64k vẫn trên model dở context 1M. Đúng câu hỏi: "context nhỏ nhưng code
//     giỏi hơn thì sao?" → giỏi hơn vẫn thắng.
//   - Mọi điều chỉnh đều giải thích được (kind + delta) để UI hiện "tại sao".

/// Danh sách tác vụ hợp lệ.
pub const RECOMMEND_TASKS: &[&str] = &["coding", "agentic", "reasoning", "vision", "long_context", "value"];

/// Một điều chỉnh điểm (giải thích được) — UI định dạng lại theo `kind`.
#[derive(Debug, Clone, Serialize)]
pub struct ScoreAdjustment {
    /// 'ctx' | 'output' | 'tools' | 'coding_bonus' | 'price'
    pub kind: String,
    pub delta: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecommendedModel {
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    /// Điểm tổng hợp 0-100 cho tác vụ đã chọn.
    pub score: u8,
    /// 'S' (top tier) | 'A' | 'B' | 'C'
    pub tier: char,
    /// Điểm gốc của judge cho phạm vi chính của tác vụ.
    pub base: u8,
    pub adjustments: Vec<ScoreAdjustment>,
    pub context: Option<u64>,
    pub price_input: Option<f64>,
    pub price_output: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecommendationView {
    pub task: String,
    /// Xếp theo điểm giảm dần.
    pub items: Vec<RecommendedModel>,
    /// Model trong config CHƯA được judge (không xếp hạng được).
    pub unevaluated: usize,
}

/// Điều chỉnh theo context (thang log2, có chặn): `neutral` = không đổi điểm,
/// mỗi lần gấp đôi context thêm `scale` điểm, bị chặn trong [min, max].
/// Context không biết → 0 (không phạtModel không khai báo).
fn ctx_log_adj(ctx: Option<u64>, neutral: u64, scale: f64, min: i32, max: i32) -> i32 {
    let Some(ctx) = ctx.filter(|c| *c > 0) else { return 0 };
    let raw = (ctx as f64).log2() - (neutral as f64).log2();
    ((raw * scale).round() as i32).clamp(min, max)
}

/// Hạng: S ≥ 85 (top tier), A ≥ 75, B ≥ 60, C còn lại.
fn tier_of(score: u8) -> char {
    match score {
        s if s >= 85 => 'S',
        s if s >= 75 => 'A',
        s if s >= 60 => 'B',
        _ => 'C',
    }
}

/// Điểm tổng hợp cho một tác vụ — trả về (score, base, adjustments).
fn score_for_task(
    task: &str,
    v: &ArbiterVerdict,
    r: &crate::api::models::ModelMatrixRow,
) -> (u8, u8, Vec<ScoreAdjustment>) {
    let mut adj: Vec<ScoreAdjustment> = Vec::new();
    let mut score: i32;
    let base: u8;

    match task {
        // CODING: kỹ năng code của judge là trục chính; context/output/tools
        // chỉ chỉnh nhẹ (chặn ±10) — chuyên gia context nhỏ vẫn thắng.
        "coding" => {
            base = v.coding;
            score = base as i32;
            let a = ctx_log_adj(r.context, 32_768, 2.5, -10, 10);
            if a != 0 {
                score += a;
                adj.push(ScoreAdjustment {
                    kind: "ctx".into(),
                    delta: a,
                });
            }
            if r.output.is_some_and(|o| o >= 16_384) {
                score += 2;
                adj.push(ScoreAdjustment {
                    kind: "output".into(),
                    delta: 2,
                });
            }
            if r.tool_call == Some(true) {
                score += 3;
                adj.push(ScoreAdjustment {
                    kind: "tools".into(),
                    delta: 3,
                });
            }
        }
        // AGENTIC (chạy agent như OpenCode): độ tin cậy gọi tool là trục chính.
        "agentic" => {
            base = ((v.tool_use as u16 + v.overall as u16) / 2) as u8;
            score = base as i32;
            let a = ctx_log_adj(r.context, 65_536, 2.0, -8, 8);
            if a != 0 {
                score += a;
                adj.push(ScoreAdjustment {
                    kind: "ctx".into(),
                    delta: a,
                });
            }
            if v.coding >= 80 {
                score += 3;
                adj.push(ScoreAdjustment {
                    kind: "coding_bonus".into(),
                    delta: 3,
                });
            }
        }
        "reasoning" => {
            base = v.reasoning;
            score = base as i32;
        }
        "vision" => {
            base = v.vision;
            score = base as i32;
        }
        // LONG-CONTEXT: chính là context (thang log chuẩn hoá 0-100; 4k→40,
        // 32k→60, 128k→80, 1M→100).
        "long_context" => {
            base = r
                .context
                .filter(|c| *c > 0)
                .map(|c| (40.0 + ((c as f64).log2() - 12.0) * 10.0).clamp(0.0, 100.0) as u8)
                .unwrap_or(0);
            score = base as i32;
        }
        // VALUE: chất lượng theo overall, trừ phạt giá (log10 tổng giá, bị
        // chặn 40) — model rẻ không bị phạt, model đắt mất điểm.
        "value" => {
            base = v.overall;
            score = base as i32;
            let total = r.price_input.unwrap_or(0.0) + r.price_output.unwrap_or(0.0);
            if total > 0.0 {
                let penalty = -((total.log10() * 12.0).round() as i32).clamp(0, 40);
                if penalty != 0 {
                    score += penalty;
                    adj.push(ScoreAdjustment {
                        kind: "price".into(),
                        delta: penalty,
                    });
                }
            }
        }
        _ => unreachable!("task đã được kiểm tra trong command"),
    }

    ((score.clamp(0, 100)) as u8, base, adj)
}

/// Xếp hạng model cho một tác vụ ("coding" mặc định) từ kết quả đồng thuận
/// của arbiter + dữ liệu ma trận.
///
/// Model chưa được judge không xếp hạng được (đếm trong `unevaluated`) —
/// xếp hạng mà không có judge sẽ thành đọc thông số, thất bại đúng cái
/// người dùng muốn tránh.
#[tauri::command(rename_all = "snake_case")]
pub fn recommend_models(task: Option<String>) -> Result<RecommendationView, String> {
    let task = task
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "coding".to_string());
    if !RECOMMEND_TASKS.contains(&task.as_str()) {
        return Err(format!(
            "Tác vụ không hợp lệ: '{task}'. Có sẵn: {}",
            RECOMMEND_TASKS.join(", ")
        ));
    }

    let cfg = ArbiterConfig::load()?;
    let verdicts = cfg.definitive();
    let rows = matrix_rows()?;

    let by_key: std::collections::HashMap<String, &crate::api::models::ModelMatrixRow> = rows
        .iter()
        .map(|r| (format!("{}/{}", r.provider_id, r.model_id), r))
        .collect();

    let mut items: Vec<RecommendedModel> = Vec::new();
    let mut evaluated = 0usize;
    for v in &verdicts {
        let Some(row) = by_key.get(&v.model) else { continue };
        evaluated += 1;
        let (score, base, adjustments) = score_for_task(&task, v, row);
        items.push(RecommendedModel {
            provider_id: row.provider_id.clone(),
            model_id: row.model_id.clone(),
            display_name: row.display_name.clone(),
            score,
            tier: tier_of(score),
            base,
            adjustments,
            context: row.context,
            price_input: row.price_input,
            price_output: row.price_output,
        });
    }

    items.sort_by(|a, b| b.score.cmp(&a.score).then(a.model_id.cmp(&b.model_id)));

    Ok(RecommendationView {
        task,
        items,
        unevaluated: rows.len().saturating_sub(evaluated),
    })
}

#[cfg(test)]
mod recommend_tests {
    use super::*;
    use opencode_manager::arbiter::{ArbiterRun, ArbiterScores};

    /// Seed home test: opencode.json (3 model: big-context yếu code, small-
    /// context giỏi code, chưa-judge) + arbiter.json (1 lần chấm 2 model đầu).
    fn seed(test_dir: &std::path::Path) {
        let opencode_dir = test_dir.join(".config").join("opencode");
        std::fs::create_dir_all(&opencode_dir).unwrap();
        std::fs::write(
            opencode_dir.join("opencode.json"),
            r#"{
                "provider": {
                    "p": {
                        "npm": "@ai-sdk/openai-compatible",
                        "name": "P",
                        "options": { "baseURL": "https://p.example.com/v1", "apiKey": "k" },
                        "models": {
                            "big": {
                                "name": "Big Context Generalist",
                                "tool_call": true,
                                "limit": { "context": 1048576, "output": 32768 }
                            },
                            "small": {
                                "name": "Small Context Coder",
                                "tool_call": true,
                                "limit": { "context": 65536, "output": 16384 }
                            },
                            "nojudge": { "name": "Chưa judge" }
                        }
                    }
                }
            }"#,
        )
        .unwrap();

        // Judge: small giỏi code (92) dù context 64k; big chỉ 55 dù context 1M.
        let mut scores = std::collections::HashMap::new();
        scores.insert(
            "p/big".to_string(),
            ArbiterScores {
                coding: 55,
                reasoning: 60,
                tool_use: 80,
                vision: 40,
                overall: 60,
                context: Some(1_048_576),
                output: Some(32_768),
                note: "generalist".into(),
            },
        );
        scores.insert(
            "p/small".to_string(),
            ArbiterScores {
                coding: 92,
                reasoning: 75,
                tool_use: 70,
                vision: 20,
                overall: 78,
                context: Some(65_536),
                output: Some(16_384),
                note: "coder chuyên".into(),
            },
        );
        let mut cfg = ArbiterConfig::default();
        cfg.push_run(ArbiterRun {
            at: "t1".into(),
            arbiter: "p/big".into(),
            scores,
        });
        cfg.save().unwrap();
    }

    /// Câu hỏi trọng tâm: context NHỎ nhưng code GIỎI → vẫn đứng trên model
    /// context 1M mà code dở. Context chỉ là bộ điều chỉnh bị chặn.
    #[test]
    fn coding_chuyen_gia_context_nho_thang_generalist() {
        let _guard = crate::test_support::TEST_ENV_LOCK.lock().unwrap();
        let test_dir = crate::test_support::isolate_home("rec_coding");
        seed(&test_dir);

        let view = recommend_models(Some("coding".into())).unwrap();
        assert_eq!(view.task, "coding");
        assert_eq!(view.items.len(), 2, "nojudge chưa được chấm");
        assert_eq!(view.unevaluated, 1, "p/nojudge nằm ngoài xếp hạng");

        let (first, second) = (&view.items[0], &view.items[1]);
        assert_eq!(first.model_id, "small", "coder chuyên phải đứng đầu");
        assert_eq!(second.model_id, "big");
        // Điểm: small = 92 + ctx(+3: 64k gấp đôi 32k-neutral, 2.5 làm tròn 3)
        // + out(+2) + tools(+3) = 100 → hạng S.
        assert_eq!(first.base, 92);
        assert_eq!(first.score, 100);
        assert_eq!(first.tier, 'S');
        // big = 55 + ctx(+10, chặn) + out(+2) + tools(+3) = 70 → hạng B:
        // context 1M cộng tối đa 10, KHÔNG đủ lật ngược khoảng cách kỹ năng.
        assert_eq!(second.base, 55);
        assert_eq!(second.score, 70);
        assert_eq!(second.tier, 'B');
        // Điều chỉnh hiện đủ để giải thích "tại sao".
        assert!(first.adjustments.iter().any(|a| a.kind == "ctx" && a.delta > 0));
        assert!(second.adjustments.iter().any(|a| a.kind == "ctx" && a.delta == 10));
    }

    /// Tác vụ khác: long_context thưởng đúng model context lớn; value phạt
    /// model đắt (khi có giá); task lạ → lỗi rõ ràng.
    #[test]
    fn cac_tac_vu_khac_va_task_lai() {
        let _guard = crate::test_support::TEST_ENV_LOCK.lock().unwrap();
        let test_dir = crate::test_support::isolate_home("rec_tasks");
        seed(&test_dir);

        // long_context: big (1M) trên small (64k).
        let view = recommend_models(Some("long_context".into())).unwrap();
        assert_eq!(view.items[0].model_id, "big");
        assert_eq!(view.items[0].base, 100, "1M → thang log chuẩn hoá max 100");
        assert_eq!(view.items[1].model_id, "small");
        assert_eq!(view.items[1].base, 80, "64k → 80");

        // value: không có giá trong dev cache → không phạt, thứ tự theo overall.
        let view = recommend_models(Some("value".into())).unwrap();
        assert_eq!(view.items[0].model_id, "small", "overall 78 > 60");
        assert!(view.items[0].adjustments.is_empty(), "không có giá → không phạt");

        // agentic: base = (tool_use + overall)/2 — big (80+60)/2=70 + ctx 1M
        // (+8), small (70+78)/2=74 + ctx 64k (0, chính là neutral). Judge chấm
        // big tin cậy tool hơn (80>70) và context lớn có ích cho agent chạy
        // dài → big thắng tác vụ này là ĐÚNG thiết kế: mỗi tác vụ thưởng đúng
        // thứ nó cần, không phải coding-point mọi nơi.
        let view = recommend_models(Some("agentic".into())).unwrap();
        assert_eq!(view.items[0].model_id, "big");
        assert_eq!(view.items[0].base, 70);
        assert_eq!(view.items[0].score, 78, "70 + ctx(+8)");
        assert_eq!(view.items[1].model_id, "small");
        assert_eq!(
            view.items[1].score, 77,
            "74 + ctx(0) + coding_bonus(+3 vì coding 92 ≥ 80)"
        );

        // Task lạ → lỗi liệt kê tác vụ hợp lệ.
        let err = recommend_models(Some("tts".into())).unwrap_err();
        assert!(err.contains("coding"), "lỗi phải gợi ý danh sách: {err}");

        // None → mặc định coding.
        assert_eq!(recommend_models(None).unwrap().task, "coding");
    }

    /// Biên công thức điều chỉnh context.
    #[test]
    fn bien_dieu_chinh_context() {
        // Neutral → 0.
        assert_eq!(ctx_log_adj(Some(32_768), 32_768, 2.5, -10, 10), 0);
        // Không biết → 0 (không phạt).
        assert_eq!(ctx_log_adj(None, 32_768, 2.5, -10, 10), 0);
        // 1M so với 32k neutral: log2 = 5 → 12.5 → chặn 10.
        assert_eq!(ctx_log_adj(Some(1_048_576), 32_768, 2.5, -10, 10), 10);
        // 8k so với 32k: log2 = -2 → -5.
        assert_eq!(ctx_log_adj(Some(8_192), 32_768, 2.5, -10, 10), -5);
    }
}
