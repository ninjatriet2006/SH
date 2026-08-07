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

    # filen_gui là app Tauri (frontend + src-tauri) — build qua script riêng,
    # không dùng `cargo build --release -p` hay `cargo tauri build` (bundle).
    if [[ "$p" == "filen_gui" ]]; then
        if [[ -f "GUI/filen_gui/build_release.sh" ]]; then
            "GUI/filen_gui/build_release.sh"
            continue
        else
            echo "⚠ Không tìm thấy GUI/filen_gui/build_release.sh (bỏ qua)"
            continue
        fi
    fi

    cargo build --release -p "$p"

    # Xác định tên binary thực sự
    bin_name="$p"
    if [[ ! -f "target/release/$bin_name" ]]; then
        if [[ -f "target/release/${p}_tui" ]]; then
            bin_name="${p}_tui"
        elif [[ -f "target/release/${p}_gui" ]]; then
            bin_name="${p}_gui"
        fi
    fi

    if [[ -f "target/release/$bin_name" ]]; then
        mkdir -p "release/$bin_name"
        cp -f "target/release/$bin_name" "release/$bin_name/$bin_name"
        chmod +x "release/$bin_name/$bin_name"
        size=$(du -h "release/$bin_name/$bin_name" | cut -f1)
        echo "✔ Xuất: release/$bin_name/$bin_name (${size})"
    else
        echo "⚠ Không tìm thấy target/release/$p (hay _tui/_gui) (bỏ qua)"
    fi
done

echo "──────────────────────────────────────────────"
echo "✅ Hoàn tất. Các binary nằm trong:"
ls -1 release/*/ 2>/dev/null | head -30
