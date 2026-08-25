#!/usr/bin/env bash
# =============================================================================
# build_release.sh — Build filen_gui (frontend + Tauri backend) và xuất gọn gàng
# vào release/filen_gui/ ở workspace root.
#
# KHÔNG dùng `cargo tauri build` (bundle). KHÔNG tạo AppImage/deb.
# Chỉ build binary + copy frontend dist + file phụ trợ.
#
# Cách dùng:
#   ./build_release.sh
# =============================================================================
set -euo pipefail

# --- Định vị workspace root (cha của apps_gui) ------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$SCRIPT_DIR"                                   # apps_gui/filen_gui
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"       # RS_AI
FRONTEND_DIR="$APP_DIR/frontend"
TAURI_DIR="$APP_DIR/bridge"
RELEASE_DIR="$WORKSPACE_ROOT/release/filen_gui"

echo "==> Workspace root : $WORKSPACE_ROOT"
echo "==> App dir        : $APP_DIR"
echo "==> Release dir    : $RELEASE_DIR"

# --- 1. Build frontend (tạo dist/) ----------------------------------------
echo "──────────────────────────────────────────────"
echo "▶ [1/6] Build frontend (npm run build)"
(
    cd "$FRONTEND_DIR"
    npm run build
)

# --- 2. Build Tauri backend (binary VỚI assets nhúng, không bundle) -------------
echo "──────────────────────────────────────────────"
echo "▶ [2/6] Build backend (cargo tauri build --no-bundle)"
echo "    (bắt buộc: nhúng frontend assets vào binary, nếu không webview"
echo "     sẽ fallback về devUrl localhost:5173 → Connection refused)"
(
    cd "$TAURI_DIR"
    cargo tauri build --no-bundle
)

# --- 3. Tạo thư mục release/filen_gui/ ----------------------------------------
echo "──────────────────────────────────────────────"
echo "▶ [3/6] Tạo thư mục release/filen_gui/"
mkdir -p "$RELEASE_DIR"

# --- 4. Copy binary ------------------------------------------------------------
echo "──────────────────────────────────────────────"
echo "▶ [4/6] Copy binary → release/filen_gui/filen_gui"
# Binary thực tế của crate src-tauri là `filen_gui_tauri`; xuất với tên `filen_gui`.
BIN_SRC="$TAURI_DIR/target/release/filen_gui_tauri"
if [[ ! -f "$BIN_SRC" ]]; then
    echo "✖ Không tìm thấy binary: $BIN_SRC" >&2
    exit 1
fi
cp -f "$BIN_SRC" "$RELEASE_DIR/filen_gui"
chmod +x "$RELEASE_DIR/filen_gui"
echo "✔ Binary: $(du -h "$RELEASE_DIR/filen_gui" | cut -f1)"

# --- 5. Copy frontend dist -----------------------------------------------------
echo "──────────────────────────────────────────────"
echo "▶ [5/6] Copy frontend/dist/* → release/filen_gui/dist/"
rm -rf "$RELEASE_DIR/dist"
cp -r "$FRONTEND_DIR/dist" "$RELEASE_DIR/dist"
echo "✔ dist/ (assets + themes + screenshots)"

# --- 6. Copy file phụ trợ (icons) ----------------------------------------------
echo "──────────────────────────────────────────────"
echo "▶ [6/6] Copy file phụ trợ (icons)"
mkdir -p "$RELEASE_DIR/icons"
cp -f "$TAURI_DIR"/icons/*.png "$RELEASE_DIR/icons/" 2>/dev/null || true
cp -f "$TAURI_DIR"/icons/icon.ico "$RELEASE_DIR/icons/" 2>/dev/null || true
cp -f "$TAURI_DIR"/icons/icon.icns "$RELEASE_DIR/icons/" 2>/dev/null || true

# --- In cấu trúc kết quả --------------------------------------------------------
echo "──────────────────────────────────────────────"
echo "✅ Hoàn tất. Cấu trúc release/filen_gui/:"
(
    cd "$RELEASE_DIR"
    find . -maxdepth 2 | sort | sed 's|^\./||; s|[^/]*/|  |g'
)