//! [INTEGRITY NOTES]
//! Mục đích: Quản lý các tác vụ liên quan đến xác thực (Authentication).
//! Trách nhiệm: Xử lý đăng nhập (có 2FA), lấy thông tin tài khoản (whoami), kiểm tra dung lượng (statfs), đăng xuất (logout), xuất cấu hình bảo mật.
//! Tương tác: Giao tiếp trực tiếp với tiến trình CLI `filen` để thực thi xác thực.
//!
//! [KHỐI AUTH]
//! Module này chịu trách nhiệm quản lý các tác vụ liên quan đến xác thực (Authentication),
//! bao gồm: đăng nhập (có xử lý 2FA), lấy thông tin tài khoản (whoami),
//! kiểm tra dung lượng (statfs), đăng xuất (logout), và xuất các cấu hình bảo mật.

use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::models::*;

/// Hàm `whoami` dùng để lấy thông tin tài khoản đang đăng nhập hiện tại.
/// Hàm nhận vào `active_account` (định danh tài khoản) và trả về một chuỗi kết quả.
pub async fn whoami_terminal(active_account: &Option<String>) -> Result<String, String> {
    // 1. Khởi tạo đối tượng Command từ module cloud_fs, đã được gắn sẵn cờ --data-dir tương ứng.
    let mut cmd = crate::cloud_fs::get_command(active_account);
    // 2. Thêm đối số "whoami" để gọi lệnh `filen whoami`.
    cmd.arg("whoami");
    
    // 3. Thực thi lệnh ngầm với thời gian chờ tối đa 15 giây (chống treo process).
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 15).await?;
    
    // 4. Kiểm tra mã thoát (exit code) của tiến trình.
    if output.status.success() {
        // Nếu thành công (exit code 0), chuyển đổi stdout từ byte sang string, loại bỏ khoảng trắng thừa và trả về.
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        // Nếu thất bại (lỗi kết nối, chưa đăng nhập...), lấy thông báo lỗi từ stderr và trả về dạng Err.
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Hàm `statfs` dùng để lấy thông tin dung lượng lưu trữ của tài khoản.
/// Trả về một tuple chứa 2 chuỗi: (Dung lượng đã dùng, Tổng dung lượng).
pub async fn statfs_terminal(active_account: &Option<String>) -> Result<(String, String), String> {
    // 1. Khởi tạo Command với thư mục data tương ứng.
    let mut cmd = crate::cloud_fs::get_command(active_account);
    // 2. Thêm đối số "statfs" để gọi lệnh `filen statfs`.
    cmd.arg("statfs");
    
    // 3. Thực thi lệnh với thời gian chờ tối đa 30 giây (tác vụ này có thể gọi API nên cho thời gian dài hơn).
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    
    // 4. Phân tích kết quả đầu ra nếu lệnh chạy thành công.
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut used = "0 B".to_string();
        let mut max = "20 GiB".to_string();
        
        // 5. Vòng lặp: Đọc từng dòng kết quả stdout để tìm từ khóa "Used:" và "Max:".
        for line in text.lines() {
            if line.contains("Used:") {
                // Nếu dòng chứa "Used:", cắt bỏ chữ "Used:" và lấy phần giá trị.
                used = line.replace("Used:", "").trim().to_string();
            } else if line.contains("Max:") {
                // Nếu dòng chứa "Max:", cắt bỏ chữ "Max:" và lấy phần giá trị.
                max = line.replace("Max:", "").trim().to_string();
            }
        }
        // 6. Trả về kết quả phân tích được.
        Ok((used, max))
    } else {
        // Nếu lỗi, trả về nội dung lỗi từ stderr.
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Hàm `login_new` dùng để thực hiện quá trình đăng nhập tương tác (Interactive Login).
/// Filen CLI không nhận password qua tham số command-line mà bắt buộc nhập qua stdin, 
/// nên hàm này phải spawn process và tương tác trực tiếp qua luồng stdio.
pub async fn login_new_terminal(
    email: &str,
    password: &str,
    twofa_code: Option<&str>,
    keep_logged: &str,
    tx: Option<tokio::sync::mpsc::UnboundedSender<CoreEvent>>,
) -> Result<(), String> {
    // Closure log: Hàm tiện ích để gửi log theo thời gian thực về UI thông qua channel `tx`.
    let log = |msg: String| {
        if let Some(ref tx) = tx {
            let _ = tx.send(CoreEvent::LoginLog(msg));
        }
    };

    // 1. Xác định thư mục lưu trữ cấu hình (data_dir) mặc định.
    if let Some(data_path) = get_default_data_dir() {
        // Tạo thư mục nếu chưa tồn tại.
        std::fs::create_dir_all(&data_path).map_err(|e| e.to_string())?;

        // 2. Xóa các file session cũ nếu có.
        // Điều này ngăn chặn CLI cố đọc file cấu hình hỏng từ phiên trước và tự động báo lỗi crash decryption.
        let keep_file = data_path.join(".filen-cli-keep-me-logged-in");
        let creds_file = data_path.join(".filen-cli-credentials");
        if keep_file.exists() {
            let _ = std::fs::remove_file(keep_file);
        }
        if creds_file.exists() {
            let _ = std::fs::remove_file(creds_file);
        }

        // 3. Khởi tạo tiến trình CLI.
        let bin = resolve_filen_bin();
        let cmd = tokio::process::Command::new(&bin);
        // Cấu hình tiến trình để có thể chạy interactive (ví dụ: tạo pseudo-terminal trên Unix nếu cần).
        let mut cmd = crate::sys::get_interactive_tokio_command(cmd);
        cmd.kill_on_drop(true); // Tự động dọn dẹp process nếu hàm này bị hủy (drop).
        cmd.arg("--data-dir").arg(&data_path).arg("whoami");
        
        // Mở các ống (pipe) để có thể đọc/ghi trực tiếp vào stdin, stdout, stderr của process con.
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

        // Chạy (spawn) tiến trình.
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        
        // Lấy quyền điều khiển các luồng stdio.
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let mut stdin = child.stdin.take().unwrap();

        // 4. Các biến trạng thái để theo dõi quá trình tương tác (state machine).
        let mut email_sent = false;
        let mut pass_sent = false;
        let mut code_sent = false;
        let mut keep_logged_sent = false;

        let mut accumulated = String::new(); // Bộ đệm lưu văn bản đọc được từ CLI.
        let mut stdout_buf = [0u8; 1024];
        let mut stderr_buf = [0u8; 1024];

        // 5. Vòng lặp chính (Event Loop): Liên tục đọc dữ liệu từ stdout và stderr đồng thời.
        loop {
            // Sử dụng tokio::select! để chờ đọc từ stdout HOẶC stderr, cái nào có dữ liệu trước thì xử lý.
            tokio::select! {
                res = stdout.read(&mut stdout_buf) => {
                    match res {
                        Ok(0) => break, // Nếu hàm read trả về 0 byte tức là process đã đóng stdout (EOF) -> thoát vòng lặp.
                        Ok(n) => {
                            // Chuyển đổi byte đọc được thành chuỗi và nối vào bộ đệm (accumulated).
                            let text = String::from_utf8_lossy(&stdout_buf[..n]);
                            accumulated.push_str(&text);
                            // Gửi log về UI.
                            for line in text.lines() {
                                if !line.trim().is_empty() {
                                    log(format!("<- CLI: {}", line.trim()));
                                }
                            }
                        }
                        Err(_) => break, // Có lỗi lúc đọc (như pipe gãy) -> thoát vòng lặp.
                    }
                }
                res = stderr.read(&mut stderr_buf) => {
                    match res {
                        Ok(0) => {}, // Nếu stderr đóng, chỉ bỏ qua (vì luồng chính phụ thuộc vào stdout).
                        Ok(n) => {
                            // Tương tự như stdout, lấy lỗi và nối vào bộ đệm.
                            let text = String::from_utf8_lossy(&stderr_buf[..n]);
                            accumulated.push_str(&text);
                            for line in text.lines() {
                                if !line.trim().is_empty() {
                                    log(format!("<- CLI (err): {}", line.trim()));
                                }
                            }
                        }
                        Err(_) => {},
                    }
                }
            }

            let acc_lower = accumulated.to_lowercase();

            // 6. Xử lý State Machine: Dựa vào nội dung bộ đệm để quyết định bước tiếp theo.
            
            // Bước 1: Nếu chưa gửi email và CLI in ra dòng chứa "email:"
            if !email_sent && acc_lower.contains("email:") {
                log(format!("-> Gửi địa chỉ Email: {}", email));
                // Ghi email vào stdin kèm dấu xuống dòng (\n) tượng trưng cho phím Enter.
                let _ = stdin.write_all(format!("{}\n", email).as_bytes()).await;
                let _ = stdin.flush().await; // Xả bộ đệm để đảm bảo dữ liệu đến được process con.
                email_sent = true;
                accumulated.clear(); // Xóa bộ đệm sau khi đã xử lý xong bước này.
            }
            // Bước 2: Nếu đã gửi email, chưa gửi pass, và CLI đòi "password:"
            else if email_sent && !pass_sent && acc_lower.contains("password:") {
                log("-> Gửi Mật khẩu: [********]".to_string());
                // Ghi password vào stdin (ẩn nội dung password trong log UI).
                let _ = stdin.write_all(format!("{}\n", password).as_bytes()).await;
                let _ = stdin.flush().await;
                pass_sent = true;
                accumulated.clear();
            }
            // Bước 3: Đã gửi password, xử lý các luồng rẽ nhánh (2FA, Keep Logged In, Lỗi)
            else if pass_sent {
                // Khả năng A: CLI đòi mã 2FA hoặc Recovery Key.
                if acc_lower.contains("2fa code") || acc_lower.contains("recovery key") {
                    if let Some(code) = twofa_code {
                        // Nếu UI đã truyền xuống mã 2FA, thì tự động điền vào.
                        if !code_sent {
                            log(format!("-> Gửi mã xác thực 2FA: {}", code));
                            let _ = stdin.write_all(format!("{}\n", code).as_bytes()).await;
                            let _ = stdin.flush().await;
                            code_sent = true;
                            accumulated.clear();
                        }
                    } else {
                        // Nếu UI chưa truyền mã 2FA, báo lỗi về UI để UI hiện popup yêu cầu người dùng nhập.
                        log("=== CLI yêu cầu mã 2FA. Đang tạm dừng để hiển thị màn hình nhập TOTP ===".to_string());
                        let _ = child.kill().await; // Hủy tiến trình login hiện tại.
                        return Err("2FA_REQUIRED".to_string());
                    }
                }
                // Khả năng B: CLI hỏi có lưu phiên đăng nhập không (Keep me logged in?)
                else if acc_lower.contains("keep me logged in") || acc_lower.contains("save credentials") {
                    if !keep_logged_sent {
                        log(format!("-> Gửi Duy trì đăng nhập: {}", keep_logged));
                        // Trả lời "y" hoặc "n" theo biến keep_logged.
                        let response = format!("{}\n", keep_logged);
                        let _ = stdin.write_all(response.as_bytes()).await;
                        let _ = stdin.flush().await;
                        keep_logged_sent = true;
                        accumulated.clear();
                    }
                }
                // Khả năng C: CLI báo sai thông tin đăng nhập.
                else if acc_lower.contains("invalid credentials") {
                    log("=== CLI báo thông tin đăng nhập không chính xác ===".to_string());
                    let _ = child.kill().await; // Chủ động hủy tiến trình.
                    return Err("Email hoặc Mật khẩu không chính xác. Vui lòng kiểm tra lại.".to_string());
                }
            }
        } // Kết thúc vòng lặp loop khi stdout EOF.

        // 7. Đợi tiến trình thực sự kết thúc (wait) và lấy mã thoát.
        let status = child.wait().await.map_err(|e| e.to_string())?;
        if status.success() {
            log("=== Đăng nhập thành công! Phiên làm việc đã được lưu ===".to_string());
            Ok(())
        } else {
            // Nếu exit code khác 0, kiểm tra lại bộ đệm xem có chuỗi báo lỗi nào không.
            let err_lower = accumulated.to_lowercase();
            let err_msg = if err_lower.contains("invalid credentials") {
                "Email hoặc Mật khẩu không chính xác. Vui lòng kiểm tra lại.".to_string()
            } else if !accumulated.trim().is_empty() {
                accumulated.trim().to_string()
            } else {
                "Đăng nhập thất bại. Vui lòng thử lại.".to_string()
            };
            log(format!("=== LỖI: CLI thoát với lỗi: {} ===", err_msg));
            Err(err_msg)
        }
    } else {
        Err("Không tìm thấy thư mục Home".to_string())
    }
}

/// Hàm `logout` dùng để đăng xuất phiên làm việc hiện tại.
pub async fn logout_terminal(active_account: &Option<String>) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("logout");
    
    // Sử dụng hàm helper thay vì cấu trúc match phức tạp
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 10)
        .await
        .map_err(|_| "Quá thời gian chờ đăng xuất (timeout 10s).".to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Hàm `export_auth_config` dùng để xuất file cấu hình xác thực nội bộ.
/// CLI sẽ yêu cầu trả lời một loạt các Prompt có tính rủi ro cao.
/// Cần sử dụng hàm `run_cmd_interactive` với bộ quy tắc tự động trả lời (Rules).
pub async fn export_auth_config_terminal(active_account: &Option<String>) -> Result<String, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("export-auth-config");
    
    // Khởi tạo các bộ quy tắc (rules) để tự động trả lời khi nhận diện được prompt từ CLI.
    let rules = [
        // Rule 1: Tự động trả lời "y" khi CLI hỏi confirm (ví dụ: overwrite file?).
        confirm_prompt_rule(1),
        // Rule 2: Tự động gõ cụm từ xác nhận nguy hiểm.
        PromptRule {
            matcher: looks_like_risks_prompt, // Hàm kiểm tra chuỗi có giống câu hỏi rủi ro không.
            response: b"I am aware of the risks\n", // Câu trả lời gửi vào stdin.
            max: 1, // Chỉ trả lời tối đa 1 lần để tránh kẹt lặp vô hạn.
        },
        // Rule 3: Trả lời vị trí xuất file.
        PromptRule {
            matcher: looks_like_export_location_prompt,
            response: b"1\n", // Chọn tùy chọn số 1: data directory.
            max: 1,
        },
    ];
    
    // Chạy interactive với bộ quy tắc trên và timeout 30s.
    let output = crate::cloud_fs::run_cmd_interactive(cmd, b"", &rules, 30).await?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Hàm `export_api_key` dùng để lấy chuỗi API Key của tài khoản.
/// Tương tự cấu hình auth, CLI sẽ hỏi xác nhận 1 lần "Proceed? (y/N)".
pub async fn export_api_key_terminal(active_account: &Option<String>) -> Result<String, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("export-api-key");
    
    // Sử dụng hàm tiện ích `run_cmd_confirm` chuyên dụng cho những lệnh chỉ đòi hỏi "y\n" 1 lần.
    let output = crate::cloud_fs::run_cmd_confirm(cmd, b"", 1, 30).await?;
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
