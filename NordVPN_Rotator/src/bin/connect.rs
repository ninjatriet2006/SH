use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::thread::sleep;
use std::time::Duration;

const POOL_FILE: &str = "server_pool.txt";
const USED_FILE: &str = "used_servers.txt";

fn main() -> io::Result<()> {
    // 1. Kiểm tra và nạp lại pool nếu rỗng
    if !Path::new(POOL_FILE).exists() || fs::metadata(POOL_FILE)?.len() == 0 {
        println!("Hết server trong Pool! Đang xáo trộn lại từ đầu...");
        refill_and_shuffle_pool()?;
    }

    // 2. Đọc tất cả các server từ pool
    let pool_content = fs::read_to_string(POOL_FILE)?;
    let mut servers: Vec<&str> = pool_content.lines().filter(|l| !l.trim().is_empty()).collect();

    if servers.is_empty() {
        println!("Lỗi: Pool rỗng kể cả sau khi nạp lại. Vui lòng chạy 'cargo run --bin get_data' trước.");
        return Ok(());
    }

    // 3. Lấy server đầu tiên (Nguồn sự thật) và cập nhật lại file pool
    let target_server = servers.remove(0);
    fs::write(POOL_FILE, servers.join("\n") + if servers.is_empty() { "" } else { "\n" })?;

    // 4. Lưu server vừa lấy vào used_servers.txt
    let mut used_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(USED_FILE)?;
    writeln!(used_file, "{}", target_server)?;

    println!("[+] Đang kết nối tới đích danh Server: {}", target_server);

    // 5. Ép NordVPN kết nối đúng server đó
    let output = Command::new("nordvpn")
        .arg("connect")
        .arg(target_server)
        .output()?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout).trim());
    } else {
        eprintln!("Lỗi khi kết nối NordVPN:\n{}", String::from_utf8_lossy(&output.stderr).trim());
    }

    // 6. Chờ mạng ổn định
    sleep(Duration::from_secs(5));

    // 7. In ra IP thực tế để kiểm chứng
    let curl_output = Command::new("curl")
        .arg("-s")
        .arg("ifconfig.me")
        .output()?;
        
    let current_ip = String::from_utf8_lossy(&curl_output.stdout);
    println!("[+] IP hiện tại của bạn: {}", current_ip.trim());
    println!("[###---] Hoàn tất đổi IP!");

    Ok(())
}

// Hàm phụ để tái nạp (refill) và xáo trộn (shuffle) lại pool
fn refill_and_shuffle_pool() -> io::Result<()> {
    if !Path::new(USED_FILE).exists() {
        // Nếu file used chưa có mà pool cũng rỗng, thì dừng lại
        return Ok(());
    }

    // Đọc used servers
    let used_content = fs::read_to_string(USED_FILE)?;
    let mut used_servers: Vec<&str> = used_content.lines().filter(|l| !l.trim().is_empty()).collect();

    if used_servers.is_empty() {
        return Ok(());
    }

    // Xáo trộn danh sách không hoàn lại
    let mut rng = thread_rng();
    used_servers.shuffle(&mut rng);

    // Nạp lại vào pool file
    fs::write(POOL_FILE, used_servers.join("\n") + "\n")?;

    // Xóa trắng used_servers
    File::create(USED_FILE)?; // Ghi đè file rỗng

    Ok(())
}
