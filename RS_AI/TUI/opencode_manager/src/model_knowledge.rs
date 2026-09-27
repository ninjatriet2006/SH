/*
[INTEGRITY NOTES]
- Mục đích: Suy đoán thông số model từ TÊN (heuristic) — nguồn kiến thức song
  song với cache models.dev, dùng để (1) điền chỗ trống khi config và
  models.dev đều không có, (2) ĐỐI CHIẾU chéo để lộ bất đồng.
- Trách nhiệm: Hàm THUẦN, deterministic, offline — không gọi mạng, không đọc
  file; unit test chốt từng họ model.
- Tương tác: GUI `api/models.rs` (fallback + tooltip đối chiếu), arbiter
  (nguồn thứ ba để so khớp).

ĐỘ TIN CẬY (thứ tự ưu tiên hiển thị: config > models.dev > tên):
  - Heuristic theo bảng họ model công khai: minh bạch, áp dụng được cho model
    mà models.dev CHƯA có (model router/tự host). NHƯNG chỉ đúng tới khi nhà
    cung cấp đổi thông số thật — THẤP HƠN models.dev (catalog cộng đồng cập
    nhật theo docs nhà cung cấp).
  - Vì vậy heuristic KHÔNG BAO GIỜ đè giá trị người dùng hay models.dev; chỉ
    điền chỗ trống và hiện giá trị của nó trong tooltip để so sánh.
*/

/// Thông số suy đoán từ tên model. Mọi field số/khả năng đều Option — chỉ
/// khẳng định cái tên nói rõ; họ model lạ → trả về None (không bịa số).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct NameFacts {
    pub family: String,
    pub context: Option<u64>,
    pub output: Option<u64>,
    pub tool_call: Option<bool>,
    pub reasoning: Option<bool>,
    pub vision: Option<bool>,
}

/// Tên có chứa token ĐÚNG SAI BÁN KHÔNG? (tách theo ký tự không alnum).
///
/// `contains` thường bị giả tín hiệu: "o1"/"o3" ăn trúng mảnh tên, "r1" ăn
/// trúng "mirror1". Token có biên thì "-r1"/"r1-" khớp còn "mirror1" không.
fn has_token(s: &str, tok: &str) -> bool {
    s.split(|c: char| !c.is_ascii_alphanumeric()).any(|seg| seg == tok)
}

/// Suy đoán từ tên model (id hoặc display name, không phân biệt hoa thường).
pub fn infer_from_name(name: &str) -> Option<NameFacts> {
    let s = name.to_lowercase();

    // Bảng họ model — THỨ TỰ QUAN TRỌNG: họ cụ thể đứng trước họ chung
    // ("gemini-1.5-pro" trước "gemini", "llama-3.1" trước "llama-3").
    let facts = if s.contains("gpt-4.1") {
        NameFacts {
            family: "gpt-4.1".into(),
            context: Some(1_047_576),
            output: Some(32_768),
            tool_call: Some(true),
            vision: Some(true),
            reasoning: None,
        }
    } else if s.contains("gpt-4o") {
        NameFacts {
            family: "gpt-4o".into(),
            context: Some(128_000),
            output: Some(16_384),
            tool_call: Some(true),
            vision: Some(true),
            reasoning: None,
        }
    } else if s.contains("gpt-5") {
        NameFacts {
            family: "gpt-5".into(),
            context: Some(400_000),
            output: Some(128_000),
            tool_call: Some(true),
            vision: Some(true),
            reasoning: None,
        }
    } else if has_token(&s, "o1") || has_token(&s, "o3") || has_token(&s, "o4") {
        NameFacts {
            family: "openai-o".into(),
            context: Some(200_000),
            output: Some(100_000),
            tool_call: Some(true),
            vision: None,
            reasoning: Some(true),
        }
    } else if s.contains("claude") {
        NameFacts {
            family: "claude".into(),
            context: Some(200_000),
            output: None,
            tool_call: Some(true),
            vision: Some(true),
            reasoning: None,
        }
    } else if s.contains("gemini-1.5-pro") {
        NameFacts {
            family: "gemini-1.5-pro".into(),
            context: Some(2_097_152),
            output: Some(8_192),
            tool_call: Some(true),
            vision: Some(true),
            reasoning: None,
        }
    } else if s.contains("gemini-1.5") {
        NameFacts {
            family: "gemini-1.5-flash".into(),
            context: Some(1_048_576),
            output: Some(8_192),
            tool_call: Some(true),
            vision: Some(true),
            reasoning: None,
        }
    } else if s.contains("gemini") {
        // Dòng 2.x+: 1M context, output 64k.
        NameFacts {
            family: "gemini-2.x".into(),
            context: Some(1_048_576),
            output: Some(65_536),
            tool_call: Some(true),
            vision: Some(true),
            reasoning: None,
        }
    } else if s.contains("deepseek") {
        NameFacts {
            family: "deepseek".into(),
            context: Some(65_536),
            output: Some(8_192),
            tool_call: Some(true),
            vision: None,
            reasoning: None,
        }
    } else if s.contains("llama-3.1") || s.contains("llama-3.3") {
        NameFacts {
            family: "llama-3.x".into(),
            context: Some(131_072),
            output: None,
            tool_call: Some(true),
            vision: None,
            reasoning: None,
        }
    } else if s.contains("llama-3") {
        NameFacts {
            family: "llama-3".into(),
            context: Some(8_192),
            output: None,
            tool_call: Some(true),
            vision: None,
            reasoning: None,
        }
    } else if s.contains("qwen") {
        NameFacts {
            family: "qwen".into(),
            context: Some(131_072),
            output: None,
            tool_call: Some(true),
            vision: None,
            reasoning: None,
        }
    } else if s.contains("kimi") {
        NameFacts {
            family: "kimi".into(),
            context: Some(131_072),
            output: None,
            tool_call: Some(true),
            vision: None,
            reasoning: None,
        }
    } else if s.contains("mistral-large") {
        NameFacts {
            family: "mistral-large".into(),
            context: Some(131_072),
            output: None,
            tool_call: Some(true),
            vision: None,
            reasoning: None,
        }
    } else if s.contains("mistral") || s.contains("mixtral") {
        NameFacts {
            family: "mistral".into(),
            context: Some(32_768),
            output: None,
            tool_call: Some(true),
            vision: None,
            reasoning: None,
        }
    } else {
        // Họ lạ (model router, self-host, tên nội bộ) → không bịa số.
        return None;
    };

    // Overlay marker: hậu tố/tiền tố nói rõ hơn họ (vd "-vl" = vision,
    // "r1"/"thinking"/"reasoner" = reasoning).
    let vision = if ["-vl", "vl-", "vision", "omni"].iter().any(|k| s.contains(k)) {
        Some(true)
    } else {
        facts.vision
    };
    let reasoning = if has_token(&s, "r1") || s.contains("thinking") || s.contains("reasoner") {
        Some(true)
    } else {
        facts.reasoning
    };

    Some(NameFacts {
        vision,
        reasoning,
        ..facts
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Họ quen thuộc → đúng limit theo bảng (kiểm chốt từng giá trị).
    #[test]
    fn ho_quen_thong_dung_limit() {
        let g = infer_from_name("gpt-4.1").unwrap();
        assert_eq!(g.context, Some(1_047_576));
        assert_eq!(g.output, Some(32_768));
        assert_eq!(g.vision, Some(true));

        let g = infer_from_name("GPT-4o-mini").unwrap();
        assert_eq!(g.context, Some(128_000));
        assert_eq!(g.output, Some(16_384));

        let c = infer_from_name("claude-opus-4-5").unwrap();
        assert_eq!(c.context, Some(200_000));
        assert_eq!(c.tool_call, Some(true));
        assert_eq!(c.output, None, "output claude đổi theo bản → không đoán");

        let g = infer_from_name("gemini-1.5-pro").unwrap();
        assert_eq!(g.context, Some(2_097_152));
        let g = infer_from_name("gemini-2.5-flash").unwrap();
        assert_eq!(g.context, Some(1_048_576));
        assert_eq!(g.output, Some(65_536));
    }

    /// Thứ tự khớp: họ cụ thể thắng họ chung.
    #[test]
    /// llama-3.1 (131k) không bị nuốt bởi nhánh llama-3 (8k).
    fn thu_tu_ho_cu_the_truoc() {
        let l = infer_from_name("llama-3.1-70b").unwrap();
        assert_eq!(l.context, Some(131_072));
        let l = infer_from_name("llama-3-8b").unwrap();
        assert_eq!(l.context, Some(8_192));
    }

    /// Marker overlay: -vl → vision; r1/thinking → reasoning.
    #[test]
    fn marker_overlay() {
        let q = infer_from_name("qwen2-vl-7b").unwrap();
        assert_eq!(q.vision, Some(true));
        let d = infer_from_name("deepseek-r1").unwrap();
        assert_eq!(d.reasoning, Some(true));
        assert_eq!(d.context, Some(65_536));
        let t = infer_from_name("qwen3-30b-a3b-thinking").unwrap();
        assert_eq!(t.reasoning, Some(true));
    }

    /// Biên token: "o3"/"r1" là TOKEN, không phải mảnh chuỗi.
    #[test]
    fn token_co_bien_khong_an_manh_ten() {
        // Token "o3" đứng riêng → khớp họ openai-o.
        let m = infer_from_name("o3-mini").unwrap();
        assert_eq!(m.family, "openai-o");
        // "mirror1" là MỘT khối liền — không chứa token "r1" → None.
        assert!(infer_from_name("mirror1").is_none());
        // Token "o1" đúng biên trong tên khác.
        let m = infer_from_name("kimi-o1-preview").unwrap();
        assert_eq!(m.family, "openai-o");
    }

    /// Họ lạ → None (không bịa số) — model router/self-host đi đường này.
    #[test]
    fn ho_la_tra_none() {
        assert!(infer_from_name("hy3").is_none());
        assert!(infer_from_name("my-custom-model").is_none());
        assert!(infer_from_name("").is_none());
    }
}

// ============================================================
// DANH SÁCH MODEL CHO PROVIDER BUILT-IN (key-only)
// ============================================================
//
// Provider built-in chỉ có KHOÁ (auth.json) — bản thân model do opencode tự
// quét từ catalogue models.dev lúc chạy. Manager cần "hỏi catalogue" để:
//   + Hiển thị đúng số đếm / danh sách cho provider built-in.
//   + Cho người dùng CHỌN model → ghi danh sách RÕ RÀNG vào opencode.json
//     (provider có models khai báo → opencode dùng đúng danh sách, không
//     quét nữa — cơ chế "cung cấp rõ ràng" thay vì để nó quét tất cả).
// CẦU: manager KHÔNG gọi mạng — đọc lại chính cache local của opencode.

struct CatalogCache {
    mtime: std::time::SystemTime,
    len: u64,
    data: std::collections::HashMap<String, Vec<(String, String)>>,
}

static CATALOG_CACHE: std::sync::Mutex<Option<CatalogCache>> = std::sync::Mutex::new(None);

/// Đọc cache models.dev → map provider_id → (id model, tên hiển thị).
///
/// File không có / hỏng → map rỗng (provider built-in vẫn hoạt động, chỉ
/// không gợi ý được danh sách model).
/// Có cache in-memory kiểm tra mtime + len để tránh đọc lặp lại file 4.7MB.
pub fn catalog_models_by_provider() -> std::collections::HashMap<String, Vec<(String, String)>> {
    let mut out: std::collections::HashMap<String, Vec<(String, String)>> = std::collections::HashMap::new();
    let Some(home) = crate::config::get_home_dir() else {
        return out;
    };
    let path = home.join(".cache").join("opencode").join("models.json");
    let Ok(meta) = std::fs::metadata(&path) else {
        return out;
    };
    let Ok(mtime) = meta.modified() else {
        return out;
    };
    let len = meta.len();

    if let Ok(guard) = CATALOG_CACHE.lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.mtime == mtime && entry.len == len {
                return entry.data.clone();
            }
        }
    }

    let Ok(text) = std::fs::read_to_string(&path) else {
        return out;
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&text) else {
        return out;
    };
    let Some(providers) = root.as_object() else {
        return out;
    };

    for (pid, pv) in providers {
        let Some(models) = pv.get("models").and_then(|m| m.as_object()) else {
            continue;
        };
        let mut list: Vec<(String, String)> = models
            .iter()
            .map(|(mid, mv)| {
                let display = mv
                    .get("name")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(mid)
                    .to_string();
                (mid.clone(), display)
            })
            .collect();
        list.sort();
        out.insert(pid.clone(), list);
    }

    if let Ok(mut guard) = CATALOG_CACHE.lock() {
        *guard = Some(CatalogCache {
            mtime,
            len,
            data: out.clone(),
        });
    }

    out
}

/// Model (id + tên hiển thị) mà catalogue có cho provider `provider_id`
/// NHƯNG chưa nằm trong `declared` (đã chọn vào opencode.json). Sắp theo id.
/// (Hiện chỉ test dùng — các luồng app lấy danh sách đầy đủ.)
#[allow(dead_code)]
pub fn catalog_models_for(provider_id: &str, declared: &std::collections::HashSet<String>) -> Vec<(String, String)> {
    catalog_models_by_provider()
        .get(provider_id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|(mid, _)| !declared.contains(mid))
        .collect()
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    /// Đọc cache thật của máy (nếu có) — chỉ kiểm contract không gỡ panic;
    /// không assert số lượng vì môi trường mỗi máy khác nhau.
    #[test]
    fn catalog_doc_duoc_khong_panic() {
        let map = catalog_models_by_provider();
        // Mọi entry đã sắp theo id (contract để UI khỏi sắp lại).
        for (pid, list) in &map {
            let mut ids: Vec<&String> = list.iter().map(|(m, _)| m).collect();
            let sorted = ids.clone();
            ids.sort();
            assert_eq!(ids, sorted, "danh sách model của {pid} phải đã sắp");
            // Tên hiển thị không bao giờ rỗng (fallback về id).
            for (mid, name) in list {
                assert!(!name.is_empty(), "tên fallback cho {mid}");
            }
        }
        // Lọc declared hoạt động đúng.
        let first_pid = map.keys().next().cloned();
        if let Some(pid) = first_pid {
            let all = map[&pid].clone();
            if let Some((first_id, _)) = all.first().cloned() {
                let declared: std::collections::HashSet<String> = [first_id].into_iter().collect();
                let rest = catalog_models_for(&pid, &declared);
                assert!(rest.iter().all(|(m, _)| !declared.contains(m)));
            }
        }
    }
}
