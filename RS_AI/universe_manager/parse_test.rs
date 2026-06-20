fn main() {
    let combined = r#"Name                                             Id                                Version     Available   Source
-----------------------------------------------------------------------------------------------------------------
Air Explorer                                     AirExplorer.AirExplorer           5.9.0       5.10.0      winget
Notepad++ (64-bit x64)                           Notepad++.Notepad++               8.9.5       8.9.6.4     winget
StartAllBack                                     StartIsBack.StartAllBack          3.9.21      3.9.23      winget
Free Download Manager                            SoftDeluxe.FreeDownloadManager    6.34.0.6878 6.34.1.6907 winget
BCUninstaller 6.1.0.0                            Klocman.BulkCrapUninstaller       6.1.0.0     6.2         winget
Lenovo System Update                             Lenovo.SystemUpdate               5.07.0139   5.08.03.59  winget
Microsoft Windows Desktop Runtime - 8.0.27 (x64) Microsoft.DotNet.DesktopRuntime.8 8.0.27      8.0.28      winget
Enpass                                           Sinew.Enpass                      6.12.1.2417 6.12.2.2551 winget
Telegram Desktop                                 Telegram.TelegramDesktop          6.8.2       6.9.3       winget
Antigravity IDE (User)                           Google.AntigravityIDE             2.0.3       2.0.4       winget
OpenAL                                           CreativeTechnology.OpenAL         Unknown     1.1         winget
12 upgrades available.

The following packages have an upgrade available, but require explicit targeting for upgrade:
Name  Id          Version  Available Source
-------------------------------------------
MSYS2 MSYS2.MSYS2 20260322 20260611  winget
1 package(s) have version numbers that cannot be determined. Use --include-unknown to see all results.
"#;
    let mut started = false;
    for line in combined.lines() {
        let tline = line.trim();
        if tline.starts_with("Name") && tline.contains("Id") && tline.contains("Version") {
            started = true;
            continue;
        }
        if started {
            if tline.starts_with('-') || tline.is_empty() { continue; }
            // Skip summary and warning lines
            if tline.ends_with("upgrades available.") || tline.contains("package(s) have version numbers that cannot be determined") || tline.starts_with("The following packages") {
                continue;
            }
            if tline.starts_with("Name") && tline.contains("Id") && tline.contains("Version") {
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
                }
            }
        }
    }
}
