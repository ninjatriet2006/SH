/*
[INTEGRITY NOTES]
- Mục đích: Tầng Core (Lõi) - Giao tiếp trực tiếp với CLI rclone.
- Trách nhiệm: Xây dựng lệnh, khởi chạy đồng bộ/bất đồng bộ rclone.
- Tương tác: Được gọi bởi tầng `logic/`.
*/

use std::process::{Command, Output};

/// Tên hàm: build_target
/// Mô tả: Trộn tên remote và đường dẫn thành định dạng chuẩn của rclone.
pub fn build_target(remote: &str, path: &str) -> String {
    if remote == "Local" {
        path.to_string()
    } else {
        format!("{}:{}", remote, path)
    }
}

/// Tên hàm: run_cmd
/// Mô tả: Khởi tạo và chạy lệnh `rclone` đồng bộ.
pub fn run_cmd(args: &[&str]) -> Result<Output, String> {
    Command::new("rclone")
        .args(args)
        .output()
        .map_err(|e| format!("Lỗi hệ thống khi gọi rclone: {}", e))
}

/// Tên hàm: spawn_cmd
/// Mô tả: Khởi tạo lệnh `rclone` ở dạng spawn ngầm (không chặn UI).
/// Lưu ý: tiến trình con được reap bởi một thread nền — nếu không, mỗi lần gọi
/// sẽ để lại một zombie process tồn tại đến khi ứng dụng thoát.
pub fn spawn_cmd(args: &[&str]) -> Result<(), String> {
    let mut child = Command::new("rclone")
        .args(args)
        .spawn()
        .map_err(|e| format!("Lỗi hệ thống khi spawn rclone: {}", e))?;

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

/// Tên hàm: run_cmd_with_stdin
/// Mô tả: Chạy lệnh `rclone` đồng bộ, đẩy `input` vào stdin của tiến trình.
/// Dùng cho `rclone rcat` (ghi nội dung file từ stdin) — cách này hoạt động
/// đồng nhất cho cả ổ Local và mọi remote.
pub fn run_cmd_with_stdin(args: &[&str], input: &[u8]) -> Result<Output, String> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new("rclone")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Lỗi hệ thống khi gọi rclone: {}", e))?;

    // `take()` để stdin được đóng sau khi ghi — nếu không, rclone chờ EOF mãi.
    child
        .stdin
        .take()
        .ok_or_else(|| "Không mở được stdin của tiến trình rclone".to_string())?
        .write_all(input)
        .map_err(|e| format!("Lỗi ghi dữ liệu vào rclone: {}", e))?;

    child
        .wait_with_output()
        .map_err(|e| format!("Lỗi khi đợi rclone kết thúc: {}", e))
}

/// Tên hàm: is_dir
/// Mô tả: Xác định một target rclone là thư mục hay file.
///
/// Dùng `lsjson --stat` — lệnh này trả về MỘT object mô tả chính target.
/// (`lsjson` thường sẽ liệt kê các *con*, nên `IsDir` của phần tử đầu tiên là
/// của file con, không phải của target — một lỗi dễ mắc.)
///
/// Trả `None` nếu không xác định được (target không tồn tại, lỗi mạng, ...).
pub fn is_dir(target: &str) -> Option<bool> {
    let output = run_cmd(&["lsjson", "--stat", target]).ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .ok()?
        .get("IsDir")?
        .as_bool()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_target_local() {
        assert_eq!(build_target("Local", "/home/user"), "/home/user");
    }

    #[test]
    fn test_build_target_remote() {
        assert_eq!(build_target("GDrive", "/Documents"), "GDrive:/Documents");
    }

    #[test]
    fn test_is_dir_distinguishes_file_and_dir() {
        // Chỉ chạy nếu có rclone trong PATH.
        if run_cmd(&["version"]).map(|o| !o.status.success()).unwrap_or(true) {
            return;
        }

        let dir = std::env::temp_dir().join("rclone_gui_is_dir_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("child.txt");
        std::fs::write(&file, b"x").unwrap();

        // Thư mục có một file con: nếu dùng `lsjson` (không --stat) thì sẽ đọc
        // sai thành file. `--stat` phải trả về true.
        assert_eq!(is_dir(&dir.to_string_lossy()), Some(true));
        assert_eq!(is_dir(&file.to_string_lossy()), Some(false));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_is_dir_none_for_missing_target() {
        if run_cmd(&["version"]).map(|o| !o.status.success()).unwrap_or(true) {
            return;
        }
        let missing = std::env::temp_dir().join("rclone_gui_definitely_missing_xyz");
        assert_eq!(is_dir(&missing.to_string_lossy()), None);
    }
}
