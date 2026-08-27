use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::thread;

fn main() {
    let mut child = Command::new("rclone")
        .args(&["check", "test_src.txt", "test_dst.txt", "--combined", "-"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            println!("LINE: {}", line.unwrap());
        }
    }
    child.wait().unwrap();
}
