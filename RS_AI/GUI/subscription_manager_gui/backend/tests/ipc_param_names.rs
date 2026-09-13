//! Chặn regression cho lớp lỗi "tên tham số IPC lệch nhau".
//!
//! Bug đã gặp: Tauri v2 mặc định `rename_all = "camelCase"`, tự đổi tham số
//! Rust `lang_code` thành `langCode` khi expose ra JS. Bridge gửi `lang_code`
//! nên IPC trả "command get_lang_content missing required key langCode", hàm
//! không bao giờ chạy. Hậu quả: từ điển rỗng → toàn bộ UI hiện raw key
//! ("sidebar.dashboard"). Cùng lỗi làm hỏng lưu settings, gán gói, gia hạn,
//! thêm/sửa user, thêm package — tức gần như mọi thao tác ghi.
//!
//! Test tĩnh (đọc source, không cần chạy app) gồm 2 lớp bảo vệ:
//!   1. Mọi `#[tauri::command]` phải khai báo `rename_all = "snake_case"`.
//!   2. Mọi key mà bridge TS gửi phải tồn tại trong tham số của command Rust.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn command_source() -> (String, String) {
    let path = crate_dir().join("src/lib.rs");
    ("lib.rs".to_string(), std::fs::read_to_string(path).expect("đọc lib.rs"))
}

/// Map: tên command Rust -> tập tên tham số.
fn rust_commands() -> HashMap<String, HashSet<String>> {
    let mut map = HashMap::new();
    let (file, src) = command_source();
    let macro_body = src
        .split_once("macro_rules! command")
        .expect("thiếu command macro")
        .1
        .split_once("command!(")
        .expect("thiếu command registry")
        .0;
    assert!(
        macro_body.contains(r#"rename_all = "snake_case""#),
        "{file}: command macro thiếu rename_all snake_case"
    );
    assert!(macro_body.contains("request: Req<$request>"));

    for chunk in src.split("command!(").skip(1) {
        let mut parts = chunk.split(',');
        let wrapper = parts.next().expect("wrapper").trim();
        if !wrapper.starts_with("ipc_") || wrapper.contains(':') {
            continue;
        }
        let command = parts.next().expect("command").trim().trim_matches('"');
        map.insert(command.to_string(), HashSet::from(["request".to_string()]));
    }
    assert!(!map.is_empty(), "không parse được command nào từ source Rust");
    map
}

/// Đọc payload DTO mà bridge TS gửi qua helper `invokeCommand`.
fn bridge_calls() -> Vec<(String, String, Vec<String>)> {
    let bridge = crate_dir().parent().expect("thư mục app").join("bridge");
    let mut calls = Vec::new();
    for entry in std::fs::read_dir(&bridge).expect("đọc bridge/") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("ts") {
            continue;
        }
        let file = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if file == "types.ts" {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("đọc bridge ts");
        collect_invokes(&file, &src, &mut calls);
    }
    assert!(!calls.is_empty(), "không tìm thấy lời gọi invoke nào trong bridge/");
    calls
}

fn collect_invokes(file: &str, src: &str, out: &mut Vec<(String, String, Vec<String>)>) {
    let mut cursor = 0usize;
    while let Some(rel) = src[cursor..].find("invokeCommand<") {
        let start = cursor + rel;
        let rest = &src[start..];
        // Bỏ qua dòng import và chú thích.
        let Some(call_rel) = rest.find(">(") else { break };
        if call_rel > 300 {
            cursor = start + "invokeCommand<".len();
            continue;
        }
        let after_call = &rest[call_rel + 2..];
        let Some(quote_rel) = after_call.find('\'') else { break };
        let name_start = start + call_rel + 2 + quote_rel + 1;
        let Some(name_len) = src[name_start..].find('\'') else {
            break;
        };
        let cmd = src[name_start..name_start + name_len].to_string();
        // Tên command hợp lệ: snake_case ascii.
        if cmd.is_empty() || !cmd.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            cursor = name_start;
            continue;
        }

        // Payload là đối số object thứ hai của invokeCommand(command, payload).
        let after_name = name_start + name_len + 1;
        let tail = &src[after_name..];
        let keys = match tail.find(|c: char| !c.is_whitespace()) {
            Some(i) if tail.as_bytes()[i] == b',' => {
                let after_comma = &tail[i + 1..];
                match after_comma.find(|c: char| !c.is_whitespace()) {
                    Some(j) if after_comma.as_bytes()[j] == b'{' => {
                        let abs = after_name + i + 1 + j;
                        parse_object_keys(src, abs)
                    }
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        };

        out.push((file.to_string(), cmd, keys));
        cursor = after_name;
    }
}

/// Lấy key cấp 1 của object literal bắt đầu tại `open_brace`.
fn parse_object_keys(src: &str, open_brace: usize) -> Vec<String> {
    let bytes = src.as_bytes();
    let mut depth = 0i32;
    let mut end = open_brace;
    for (i, b) in bytes.iter().enumerate().skip(open_brace) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let body = &src[open_brace + 1..end];

    let mut keys = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for ch in body.chars() {
        match ch {
            '{' | '(' | '[' => {
                depth += 1;
                current.push(ch);
            }
            '}' | ')' | ']' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => keys.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    keys.push(current);

    keys.into_iter()
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() || part.starts_with("//") || part.starts_with("...") {
                return None;
            }
            let key = part.split(':').next()?.trim();
            let key = key.trim_end_matches('?');
            if key.is_empty() || !key.chars().all(|c| c.is_alphanumeric() || c == '_') {
                None
            } else {
                Some(key.to_string())
            }
        })
        .collect()
}

#[test]
fn moi_command_khai_bao_snake_case() {
    // Assert nằm trong `rust_commands()`; gọi ở đây để test chạy độc lập.
    let cmds = rust_commands();
    // Con số này là chốt có chủ đích: thêm/bớt command phải sửa test, buộc
    // người sửa nhìn lại xem command mới đã đăng ký và đúng quy ước chưa.
    assert_eq!(
        cmds.len(),
        31,
        "số command thay đổi ({}) — cập nhật test nếu thêm/bớt command có chủ đích",
        cmds.len()
    );
}

#[test]
fn key_bridge_gui_khop_tham_so_rust() {
    let cmds = rust_commands();
    let mut loi = Vec::new();

    for (file, cmd, keys) in bridge_calls() {
        let Some(params) = cmds.get(&cmd) else {
            loi.push(format!("{file}: gọi `{cmd}` nhưng không có command Rust nào tên vậy"));
            continue;
        };
        for key in keys {
            if !params.contains("request") {
                loi.push(format!("{file}: `{cmd}` không nhận đúng một tham số `request`"));
                continue;
            }
            let expected = request_fields(&cmd);
            if !expected.contains(&key) {
                let mut goi_y = String::new();
                // Gợi ý khi lệch do camelCase.
                let snake = to_snake(&key);
                if expected.contains(&snake) {
                    goi_y = format!(" (ý bạn là `{snake}`?)");
                }
                loi.push(format!(
                    "{file}: `{cmd}` nhận key `{key}` nhưng tham số Rust là {:?}{goi_y}",
                    sorted(&expected)
                ));
            }
        }
    }

    assert!(loi.is_empty(), "Lệch tên tham số IPC:\n  - {}", loi.join("\n  - "));
}

fn request_fields(command: &str) -> HashSet<String> {
    let (_, src) = command_source();
    let marker = format!("ipc_{command},");
    let after = src
        .split_once(&marker)
        .unwrap_or_else(|| panic!("thiếu khai báo {command}"))
        .1;
    let request_type = after.split(',').nth(1).expect("request DTO").trim();
    let dto_marker = format!("request_dto!({request_type} {{");
    let body = src
        .split_once(&dto_marker)
        .unwrap_or_else(|| panic!("thiếu DTO {request_type}"))
        .1
        .split_once("});")
        .expect("đóng DTO")
        .0;
    body.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#[") {
                return None;
            }
            line.split_once(':').map(|(name, _)| name.trim().to_string())
        })
        .collect()
}

fn sorted(set: &HashSet<String>) -> Vec<String> {
    let mut v: Vec<String> = set.iter().cloned().collect();
    v.sort();
    v
}

fn to_snake(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_uppercase() {
            out.push('_');
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Mọi command đăng ký trong `generate_handler!` phải tồn tại và ngược lại —
/// tránh trường hợp viết command rồi quên đăng ký (frontend gọi sẽ lỗi runtime).
#[test]
fn command_dang_ky_day_du_trong_handler() {
    let lib = std::fs::read_to_string(crate_dir().join("src/lib.rs")).expect("đọc lib.rs");
    let start = lib.find("generate_handler![").expect("thiếu generate_handler!");
    let end = lib[start..].find(']').map(|i| start + i).expect("thiếu ]");
    let block = &lib[start..end];

    let registered: HashSet<String> = block
        .lines()
        .filter_map(|l| {
            let l = l.trim().trim_end_matches(',');
            l.rsplit("::")
                .next()
                .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        })
        .map(str::to_string)
        .collect();

    let defined: HashSet<String> = rust_commands().keys().map(|name| format!("ipc_{name}")).collect();

    let thieu: Vec<_> = sorted(&defined.difference(&registered).cloned().collect());
    let du: Vec<_> = sorted(&registered.difference(&defined).cloned().collect());
    assert!(
        thieu.is_empty() && du.is_empty(),
        "generate_handler! không khớp: chưa đăng ký {thieu:?}, đăng ký thừa {du:?}"
    );
}
