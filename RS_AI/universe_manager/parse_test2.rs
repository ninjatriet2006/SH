use std::process::Command;

fn main() {
    let out = Command::new("winget").args(&["upgrade", "--include-unknown"]).output().unwrap();
    let combined = String::from_utf8_lossy(&out.stdout).to_string();
    let mut started = false;
    let mut count = 0;
    println!("Raw Winget Output:\n{}", combined);
    println!("=====================================");

    for line in combined.lines() {
        let tline = line.trim();
        if tline.contains("Name") && tline.contains("Id") && tline.contains("Version") {
            started = true;
            continue;
        }
        if started {
            if tline.starts_with('-') || tline.is_empty() { continue; }
            if tline.ends_with("upgrades available.") || tline.contains("package(s) have version numbers that cannot be determined") || tline.starts_with("The following packages") {
                continue;
            }
            if tline.contains("Name") && tline.contains("Id") && tline.contains("Version") {
                continue;
            }

            let parts: Vec<&str> = tline.split_whitespace().collect();
            if parts.len() >= 4 {
                let source = parts.last().unwrap().to_string();
                if parts.len() >= 5 {
                    let available = parts[parts.len()-2].to_string();
                    let current = parts[parts.len()-3].to_string();
                    let id = parts[parts.len()-4].to_string();
                    let name = parts[..parts.len()-4].join(" ");
                    println!("Parsed: {} | {} | {} | {} | {}", name, id, current, available, source);
                    count += 1;
                }
            }
        }
    }
    println!("Total parsed: {}", count);
}
