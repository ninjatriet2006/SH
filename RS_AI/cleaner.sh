#!/usr/bin/env bash
# =============================================================================
# cleaner.sh — Dọn dẹp toàn bộ file rác sinh ra khi build trong RS_AI.
#
# TỰ ĐỘNG BẢO VỆ: Tuyệt đối KHÔNG BAO GIỜ chạm vào thư mục `release/` chứa thành phẩm.
#
# Các thành phần được dọn dẹp:
#   1. Mọi thư mục `target/` (cả root lẫn các workspace lồng như universe_manager_gui).
#   2. Các thư mục build frontend trung gian `GUI/*/frontend/dist`.
#   3. Cache của bundler / compiler: `node_modules/.cache`, `.vite`, `tsconfig.tsbuildinfo`.
#   4. Cache Python (nếu có): `__pycache__`, `*.pyc`, `.pytest_cache`.
#
# Cách dùng:
#   ./cleaner.sh            # Quét, hiển thị dung lượng và hỏi xác nhận
#   ./cleaner.sh --dry-run  # Chỉ kiểm tra dung lượng rác, không xóa gì
#   ./cleaner.sh -y         # Xóa ngay không cần xác nhận
#   ./cleaner.sh --keep-builder # Dọn rác nhưng giữ lại gui_builder
# =============================================================================
set -euo pipefail
cd "$(dirname "$0")"

ROOT_DIR="$(pwd)"
RELEASE_DIR="$ROOT_DIR/release"

# Tự mở trong terminal emulator nếu click từ file manager (thiếu TTY)
if [[ ! -t 0 || ! -t 1 ]] && [[ -z "${CLEANER_WRAPPED:-}" ]]; then
    for term in gnome-terminal konsole xfce4-terminal x-terminal-emulator alacritty kitty xterm; do
        if command -v "$term" &>/dev/null; then
            export CLEANER_WRAPPED=1
            exec "$term" -- "$0" "$@" || exec "$term" -e "$0" "$@"
        fi
    done
fi

DRY_RUN=0
AUTO_YES=0
KEEP_BUILDER=0

for arg in "$@"; do
    case "$arg" in
        --dry-run|-n) DRY_RUN=1 ;;
        -y|--yes)     AUTO_YES=1 ;;
        --keep-builder) KEEP_BUILDER=1 ;;
        --help|-h)
            echo "Cách dùng: $0 [TÙY CHỌN]"
            echo "  --dry-run, -n      Chỉ quét dung lượng rác, không xóa"
            echo "  -y, --yes          Xóa không cần hỏi xác nhận"
            echo "  --keep-builder     Giữ lại target/release/gui_builder"
            exit 0
            ;;
    esac
done

echo "=========================================================="
echo "          🧹 RS_AI BUILD CLEANER TOOL                    "
echo "=========================================================="
echo "▶ Thư mục gốc:    $ROOT_DIR"
echo "▶ Khu vực bảo vệ: $RELEASE_DIR (BẢO VỆ TUYỆT ĐỐI)"
echo "----------------------------------------------------------"

# 1. Tìm tất cả các thư mục target (ngoại trừ trong release)
TARGET_DIRS=()
while IFS= read -r -d '' d; do
    # Chốt chặn an toàn: không bao giờ quét trong release
    if [[ "$d" != "$RELEASE_DIR"* && "$d" != "$ROOT_DIR/release"* ]]; then
        TARGET_DIRS+=("$d")
    fi
done < <(find . -not -path "./release/*" -not -path "./.git/*" -name "target" -type d -print0)

# 2. Tìm tất cả các thư mục dist trung gian của frontend
DIST_DIRS=()
while IFS= read -r -d '' d; do
    if [[ "$d" != "$RELEASE_DIR"* && "$d" != "$ROOT_DIR/release"* ]]; then
        DIST_DIRS+=("$d")
    fi
done < <(find ./GUI -not -path "*/node_modules/*" -not -path "./release/*" -name "dist" -type d -print0 2>/dev/null || true)

# 3. Tìm các thư mục cache bundler/compiler
CACHE_DIRS=()
while IFS= read -r -d '' d; do
    if [[ "$d" != "$RELEASE_DIR"* && "$d" != "$ROOT_DIR/release"* ]]; then
        CACHE_DIRS+=("$d")
    fi
done < <(find . -not -path "./release/*" -not -path "./.git/*" \( -name ".vite" -o -name ".cache" -o -name "__pycache__" -o -name ".pytest_cache" \) -type d -print0 2>/dev/null || true)

# Thống kê dung lượng
calc_size() {
    local dirs=("$@")
    if [[ ${#dirs[@]} -eq 0 ]]; then
        echo "0B"
        return
    fi
    du -csh "${dirs[@]}" 2>/dev/null | tail -n 1 | cut -f1
}

echo "🔍 Đang rà soát file build rác..."
TARGET_SIZE=$(calc_size "${TARGET_DIRS[@]}")
DIST_SIZE=$(calc_size "${DIST_DIRS[@]}")
CACHE_SIZE=$(calc_size "${CACHE_DIRS[@]}")

echo "  • Các thư mục Rust target (${#TARGET_DIRS[@]} vị trí): $TARGET_SIZE"
for t in "${TARGET_DIRS[@]}"; do
    echo "    - $t ($(du -sh "$t" 2>/dev/null | cut -f1))"
done

echo "  • Các thư mục Frontend dist trung gian (${#DIST_DIRS[@]} vị trí): $DIST_SIZE"
for d in "${DIST_DIRS[@]}"; do
    echo "    - $d ($(du -sh "$d" 2>/dev/null | cut -f1))"
done

if [[ ${#CACHE_DIRS[@]} -gt 0 ]]; then
    echo "  • Bộ nhớ đệm bundler/cache (${#CACHE_DIRS[@]} vị trí): $CACHE_SIZE"
fi

ALL_TARGETS=("${TARGET_DIRS[@]}" "${DIST_DIRS[@]}" "${CACHE_DIRS[@]}")
TOTAL_SIZE=$(calc_size "${ALL_TARGETS[@]}")

echo "----------------------------------------------------------"
echo "💥 TỔNG DUNG LƯỢNG RÁC SẼ ĐƯỢC GIẢI PHÓNG: $TOTAL_SIZE"
echo "----------------------------------------------------------"

if [[ ${#ALL_TARGETS[@]} -eq 0 ]]; then
    echo "✔ Workspace hoàn toàn sạch sẽ, không có rác build!"
    if [[ -n "${CLEANER_WRAPPED:-}" ]]; then
        read -rp "Nhấn Enter để đóng..."
    fi
    exit 0
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "ℹ Chế độ dry-run: Không có file/thư mục nào bị xóa."
    if [[ -n "${CLEANER_WRAPPED:-}" ]]; then
        read -rp "Nhấn Enter để đóng..."
    fi
    exit 0
fi

if [[ "$AUTO_YES" -eq 0 ]]; then
    read -rp "❓ Bạn có chắc chắn muốn xóa sạch các file rác trên? (y/N): " confirm
    if [[ "$confirm" != [yY] && "$confirm" != [yY][eE][sS] ]]; then
        echo "❌ Đã hủy thao tác."
        if [[ -n "${CLEANER_WRAPPED:-}" ]]; then
            read -rp "Nhấn Enter để đóng..."
        fi
        exit 0
    fi
fi

# Sao lưu tạm gui_builder nếu yêu cầu --keep-builder
TEMP_BUILDER=""
if [[ "$KEEP_BUILDER" -eq 1 && -x "target/release/gui_builder" ]]; then
    TEMP_BUILDER="$(mktemp)"
    cp "target/release/gui_builder" "$TEMP_BUILDER"
    echo "▶ Đã tạm giữ binary gui_builder"
fi

echo "▶ Đang tiến hành dọn dẹp..."
for item in "${ALL_TARGETS[@]}"; do
    # Chốt chặn kiểm tra an toàn lần cuối
    if [[ "$item" == "$RELEASE_DIR"* || "$item" == "$ROOT_DIR" || "$item" == "/" || "$item" == "." ]]; then
        echo "⛔ CẢNH BÁO NGUY HIỂM: Bỏ qua đường dẫn $item"
        continue
    fi
    echo "  🗑 Xóa $item..."
    rm -rf "$item"
done

# Khôi phục gui_builder nếu có
if [[ -n "$TEMP_BUILDER" && -f "$TEMP_BUILDER" ]]; then
    mkdir -p "target/release"
    mv "$TEMP_BUILDER" "target/release/gui_builder"
    chmod +x "target/release/gui_builder"
    echo "✔ Đã khôi phục target/release/gui_builder"
fi

echo "----------------------------------------------------------"
echo "✅ DỌN DẸP HOÀN TẤT! Đã giải phóng $TOTAL_SIZE dung lượng ổ đĩa."
echo "   Thư mục release/ vẫn an toàn nguyên vẹn ($(du -sh "$RELEASE_DIR" 2>/dev/null | cut -f1))."
echo "=========================================================="

if [[ -n "${CLEANER_WRAPPED:-}" ]]; then
    read -rp "Nhấn Enter để đóng cửa sổ..."
fi
