import re

with open("backend/src/logic/file_ops.rs", "r") as f:
    content = f.read()

new_fn = """pub async fn check_conflicts(app_handle: tauri::AppHandle, srcs: Vec<String>, dest_path: String) -> Result<Vec<String>, String> {
    use std::process::{Command, Stdio};
    use std::io::{BufRead, BufReader};
    use tauri::Emitter;

    let mut conflicts = Vec::new();

    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for src in srcs {
        let (remote, path) = parse_remote_path(&src);
        let path = path.trim_end_matches('/');
        let (parent, base) = match path.rfind('/') {
            Some(idx) => (&path[..idx], &path[idx+1..]),
            None => ("", path),
        };
        
        let parent_remote = if parent.is_empty() {
            format!("{}::/", remote)
        } else {
            format!("{}::{}", remote, parent)
        };
        
        groups.entry(parent_remote).or_insert_with(Vec::new).push(base.to_string());
    }

    let (dest_remote, dest_real) = parse_remote_path(&dest_path);
    let dest_target = crate::core::rclone::build_target(&dest_remote, &dest_real);

    for (parent_src, bases) in groups {
        let (src_remote, src_real) = parse_remote_path(&parent_src);
        let src_target = crate::core::rclone::build_target(&src_remote, &src_real);
        
        let mut string_args = vec![
            "check".to_string(), 
            src_target.clone(), 
            dest_target.clone(), 
            "--combined".to_string(), 
            "-".to_string(),
            "--use-json-log".to_string(),
            "--stats".to_string(),
            "0.5s".to_string(),
        ];
        
        for base in &bases {
            string_args.push("--include".to_string());
            string_args.push(base.clone());
            string_args.push("--include".to_string());
            string_args.push(format!("{}/**", base));
        }
        
        let mut child = Command::new("rclone")
            .args(&string_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Lỗi khởi chạy rclone check: {}", e))?;

        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;

        let app_handle_clone = app_handle.clone();
        
        // Luồng đọc stderr để lấy progress json
        let stderr_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line_str) = line {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line_str) {
                        if let Some(stats) = json.get("stats") {
                            let payload = serde_json::json!({
                                "stats": stats
                            });
                            let _ = app_handle_clone.emit("conflict_check_progress", payload);
                        }
                    }
                }
            }
        });

        // Luồng chính đọc stdout để bắt kết quả conflict
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line_str) = line {
                if line_str.starts_with("* ") {
                    let conflict_path = line_str[2..].trim_end().to_string();
                    conflicts.push(conflict_path);
                }
            }
        }

        stderr_thread.join().unwrap();
        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            // Rclone check trả về mã lỗi 1 nếu có difference, điều này là bình thường
            // Nên chúng ta không bắt lỗi ở đây trừ khi status != 1 và status != 0
            if status.code() != Some(0) && status.code() != Some(1) {
                return Err(format!("Lỗi kiểm tra trùng lặp, mã lỗi: {}", status));
            }
        }
    }

    Ok(conflicts)
}"""

# Find the start and end of the function
start_idx = content.find("pub async fn check_conflicts(")
end_idx = content.find("}\n\n#[cfg(test)]", start_idx) + 1

if start_idx != -1 and end_idx != 0:
    content = content[:start_idx] + new_fn + content[end_idx:]
    with open("backend/src/logic/file_ops.rs", "w") as f:
        f.write(content)
    print("Success")
else:
    print(f"Failed to find indices: {start_idx}, {end_idx}")

