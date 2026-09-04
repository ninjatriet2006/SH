#!/usr/bin/env bash
# =============================================================================
# build_release.sh — Khởi chạy GUI BUILDER (TUI) để chọn và build project.
#
# Bất kể mở bằng cách nào (terminal, double-click từ file manager), script luôn
# mở TUI "GUI BUILDER" cho phép chọn project bằng phím. Danh sách project được
# phát hiện động qua `cargo metadata` — thêm project mới vào workspace là tự có,
# không cần sửa file này.
#
# Vì sao cần TUI: bản cũ hard-code 11 project và bỏ sót rclone_gui; đồng thời
# build app Tauri bằng `cargo build` khiến frontend không được nhúng vào binary,
# app chạy lên báo "connection refused" vì rơi về devUrl.
# =============================================================================
set -euo pipefail
cd "$(dirname "$0")"

BUILDER_BIN="target/release/gui_builder"

# Dựng builder nếu chưa có hoặc mã nguồn đã đổi.
needs_build=0
if [[ ! -x "$BUILDER_BIN" ]]; then
    needs_build=1
else
    while IFS= read -r src; do
        if [[ "$src" -nt "$BUILDER_BIN" ]]; then
            needs_build=1
            break
        fi
    done < <(find tools/gui_builder -name '*.rs' -o -name 'Cargo.toml')
fi

if [[ "$needs_build" -eq 1 ]]; then
    echo "▶ Dựng GUI BUILDER..."
    cargo build --release -p gui_builder
fi

exec "./$BUILDER_BIN" "$@"
