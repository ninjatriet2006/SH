#!/usr/bin/env bash
# =============================================================================
# build_release.sh — Build rclone_gui (frontend + Tauri backend) và xuất gọn gàng
# vào release/rclone_gui/ ở workspace root.
# =============================================================================
set -euo pipefail

# --- Định vị workspace root ------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$SCRIPT_DIR"                                   
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"       
FRONTEND_DIR="$APP_DIR/frontend"
TAURI_DIR="$APP_DIR/backend"
RELEASE_DIR="$WORKSPACE_ROOT/release/rclone_gui"

echo "==> Workspace root : $WORKSPACE_ROOT"
echo "==> App dir        : $APP_DIR"
echo "==> Release dir    : $RELEASE_DIR"

# --- 1. Build frontend ----------------------------------------
echo "──────────────────────────────────────────────"
echo "▶ [1/6] Cài đặt & Build frontend"
(
    cd "$FRONTEND_DIR"
    npm install
    npm run build
)

# --- 2. Build Tauri backend -----------------------------------
echo "──────────────────────────────────────────────"
echo "▶ [2/6] Build backend (Mở cửa sổ Terminal...)"
(
    cd "$APP_DIR/backend"
    gnome-terminal --wait --title="Rclone GUI Builder" -- bash -c "npx @tauri-apps/cli build --no-bundle || { echo 'Build LỖI!'; read -n 1; exit 1; }; echo ''; echo 'Build hoàn tất. Nhấn phím bất kỳ để tiếp tục...'; read -n 1"
)

# --- 3. Tạo thư mục release -----------------------------------
echo "──────────────────────────────────────────────"
echo "▶ [3/6] Tạo thư mục release/rclone_gui/"
mkdir -p "$RELEASE_DIR"

# --- 4. Copy binary -------------------------------------------
echo "──────────────────────────────────────────────"
echo "▶ [4/6] Copy binary → release/rclone_gui/rclone_gui"
BIN_SRC="$WORKSPACE_ROOT/target/release/rclone_gui"
if [[ ! -f "$BIN_SRC" ]]; then
    echo "✖ Không tìm thấy binary: $BIN_SRC" >&2
    exit 1
fi
cp -f "$BIN_SRC" "$RELEASE_DIR/rclone_gui"
chmod +x "$RELEASE_DIR/rclone_gui"

# --- 5. Copy frontend dist, langs, themes, fonts --------------
echo "──────────────────────────────────────────────"
echo "▶ [5/6] Copy thư mục phụ trợ (dist, langs, themes, fonts)"
rm -rf "$RELEASE_DIR/dist" "$RELEASE_DIR/langs" "$RELEASE_DIR/themes" "$RELEASE_DIR/fonts"
cp -r "$FRONTEND_DIR/dist" "$RELEASE_DIR/dist"
cp -r "$APP_DIR/langs" "$RELEASE_DIR/langs"
cp -r "$APP_DIR/themes" "$RELEASE_DIR/themes"
[ -d "$APP_DIR/fonts" ] && cp -r "$APP_DIR/fonts" "$RELEASE_DIR/fonts"

# --- 6. Copy icons --------------------------------------------
echo "──────────────────────────────────────────────"
echo "▶ [6/6] Copy file icons"
mkdir -p "$RELEASE_DIR/icons"
cp -f "$TAURI_DIR"/icons/*.png "$RELEASE_DIR/icons/" 2>/dev/null || true
cp -f "$TAURI_DIR"/icons/icon.ico "$RELEASE_DIR/icons/" 2>/dev/null || true
cp -f "$TAURI_DIR"/icons/icon.icns "$RELEASE_DIR/icons/" 2>/dev/null || true

echo "──────────────────────────────────────────────"
echo "✅ Hoàn tất. Cấu trúc release/rclone_gui/:"
(
    cd "$RELEASE_DIR"
    find . -maxdepth 2 | sort | sed 's|^\./||; s|[^/]*/|  |g'
)
