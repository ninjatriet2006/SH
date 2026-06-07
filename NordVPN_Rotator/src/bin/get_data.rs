use std::fs::File;
use std::io::Write;
use reqwest::blocking::get;
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Đang lấy danh sách server từ NordVPN...");
    
    // Gọi API của NordVPN
    // Đã thêm ?limit=20000 để lấy toàn bộ danh sách (hơn 9000 server) thay vì 100 mặc định
    let url = "https://api.nordvpn.com/v1/servers?limit=20000"; 
    let response = get(url)?.text()?;
    
    let json: Value = serde_json::from_str(&response)?;
    
    let mut server_pool = File::create("server_pool.txt")?;
    let mut count = 0;

    if let Some(servers) = json.as_array() {
        for server in servers {
            // Mẹo: "hostname" thường có dạng "us8000.nordvpn.com"
            // Việc bóc tách chữ "us8000" từ hostname sẽ chính xác 100% để gọi lệnh "nordvpn connect us8000"
            if let Some(hostname) = server.get("hostname").and_then(|v| v.as_str()) {
                if let Some(server_id) = hostname.split('.').next() {
                    writeln!(server_pool, "{}", server_id)?;
                    count += 1;
                }
            } else if let Some(name) = server.get("name").and_then(|v| v.as_str()) {
                // Dự phòng lấy theo name
                writeln!(server_pool, "{}", name)?;
                count += 1;
            }
        }
    }

    println!("Đã lưu {} server vào server_pool.txt", count);
    Ok(())
}
