use serde_json::Value;

fn main() {
    let output = std::process::Command::new("rclone").arg("config").arg("dump").output().unwrap();
    let json_str = String::from_utf8_lossy(&output.stdout);
    let dump: Value = serde_json::from_str(&json_str).unwrap();
    
    let mut remotes = Vec::new();
    if let Value::Object(map) = dump {
        for (name, config) in map {
            if let Value::Object(mut config_map) = config {
                config_map.insert("name".to_string(), Value::String(name));
                remotes.push(Value::Object(config_map));
            }
        }
    }
    
    println!("Remotes: {}", remotes.len());
}
