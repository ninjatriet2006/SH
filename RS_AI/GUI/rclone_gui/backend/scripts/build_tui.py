#!/usr/bin/env python3
import sys
import subprocess
import json
import time
import os

def get_total_packages():
    try:
        output = subprocess.check_output(["cargo", "metadata", "--format-version=1"], stderr=subprocess.DEVNULL, cwd="backend")
        data = json.loads(output)
        return len(data.get("packages", []))
    except Exception:
        return 200 # Fallback 

def draw_progress(current, total, message):
    bar_len = 40
    percent = min(100.0, 100.0 * current / total) if total > 0 else 0.0
    filled = int(bar_len * percent / 100.0)
    bar = "█" * filled + "░" * (bar_len - filled)
    
    # Truncate message to avoid wrapping
    msg = message[:50].ljust(50)
    
    # ANSI clear line and print
    sys.stdout.write(f"\r\033[K[\033[36m{bar}\033[0m] \033[32m{percent:5.1f}%\033[0m | {msg}")
    sys.stdout.flush()

def main():
    print("\033[36m[TUI Builder]\033[0m Đang phân tích metadata dự án...", flush=True)
    total_packages = get_total_packages()
    # Typically, cargo builds depend on profiles, features, etc. 
    # Total units might be slightly higher or lower, we will cap at 99.9% until finished.
    print(f"\033[36m[TUI Builder]\033[0m Tìm thấy khoảng {total_packages} packages cần xử lý. Bắt đầu biên dịch...", flush=True)

    cmd = [
        "cargo", "build", "--release", "--message-format=json-render-diagnostics"
    ]
    
    env = os.environ.copy()
    env["TAURI_ENV_DEBUG"] = "0"
    
    process = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd="backend",
        env=env
    )

    compiled_count = 0

    # Dùng non-blocking io hoặc readline
    while True:
        line = process.stdout.readline()
        if not line and process.poll() is not None:
            break
        
        if not line.strip():
            continue

        try:
            msg = json.loads(line)
            reason = msg.get("reason")
            
            if reason == "compiler-artifact":
                compiled_count += 1
                pkg_name = msg.get("package_id", "").split(" ")[0]
                if not pkg_name:
                    pkg_name = "unknown"
                draw_progress(compiled_count, total_packages, f"Đang biên dịch: {pkg_name}")
                
            elif reason == "compiler-message":
                # Lọc ra các Warning / Error quan trọng
                message = msg.get("message", {})
                level = message.get("level")
                rendered = message.get("rendered")
                if level in ["error", "warning"] and rendered:
                    # Xóa dòng progress bar hiện tại
                    sys.stdout.write("\r\033[K")
                    # In warning/error ra
                    print(rendered.strip(), flush=True)
                    # Vẽ lại progress bar
                    draw_progress(compiled_count, total_packages, f"Tiếp tục biên dịch...")

        except json.JSONDecodeError:
            # Những dòng không phải JSON (log của Tauri, hoặc Node.js)
            line = line.strip()
            # Bỏ qua dòng Tauri báo npm, node, hoặc [Info] nếu không quan trọng
            if line:
                sys.stdout.write("\r\033[K")
                print(f"\033[90m{line}\033[0m", flush=True)
                draw_progress(compiled_count, total_packages, "Đang xử lý...")

    # Đọc stderr phòng trường hợp có lỗi panic
    err = process.stderr.read()
    if err.strip():
        sys.stdout.write("\r\033[K")
        print(f"\033[31m[LỖI]\033[0m {err}", flush=True)

    retcode = process.wait()
    sys.stdout.write("\r\033[K")
    if retcode == 0:
        print("\033[32m[TUI Builder]\033[0m Biên dịch hoàn tất thành công! (100.0%)", flush=True)
    else:
        print(f"\033[31m[TUI Builder]\033[0m Quá trình biên dịch thất bại với mã lỗi {retcode}.", flush=True)
        sys.exit(retcode)

if __name__ == "__main__":
    main()
