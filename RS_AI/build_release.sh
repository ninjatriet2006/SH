#!/usr/bin/env bash
# =============================================================================
# build_release.sh — Build các dự án Rust workspace và xuất binary gọn gàng
# vào thư mục release/<tên_dự_án>/<binary> ở workspace root.
#
# Cách dùng:
#   ./build_release.sh              # build TẤT CẢ (10 dự án)
#   ./build_release.sh filen_gui    # chỉ build 1 dự án
#   ./build_release.sh filen_gui filen_tui   # build nhiều dự án
# =============================================================================
set -euo pipefail
cd "$(dirname "$0")"

# Danh sách mặc định: tất cả packages trong workspace
ALL_PROJECTS=(
    filen_gui img_splt_gui opencode_manager_gui universal_converter_gui universe_manager_gui
    filen_tui img_splt opencode_manager universal_converter universe_manager
)

# Nếu có tham số → chỉ build các dự án được yêu cầu
if [[ $# -gt 0 ]]; then
    PROJECTS=("$@")
else
    PROJECTS=("${ALL_PROJECTS[@]}")
fi

# Build xong → binary nằm trong target/release/<tên_package>
mkdir -p release

for p in "${PROJECTS[@]}"; do
    echo "──────────────────────────────────────────────"
    echo "▶ Build: $p"
    cargo build --release -p "$p"

    if [[ -f "target/release/$p" ]]; then
        mkdir -p "release/$p"
        cp -f "target/release/$p" "release/$p/$p"
        chmod +x "release/$p/$p"
        size=$(du -h "release/$p/$p" | cut -f1)
        echo "✔ Xuất: release/$p/$p (${size})"
    else
        echo "⚠ Không tìm thấy target/release/$p (bỏ qua)"
    fi
done

echo "──────────────────────────────────────────────"
echo "✅ Hoàn tất. Các binary nằm trong:"
ls -1 release/*/ 2>/dev/null | head -30
