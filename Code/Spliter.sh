#!/bin/bash

# =================================================================
# SCRIPT XỬ LÝ FILE THEO QUY TẮC N (BẢN FIX: FFMPEG STDIN & TỰ TẮT)
# =================================================================

real_progress() {
    local current=$1
    local total=$2
    [ "$total" -eq 0 ] && return
    local percent=$(( current * 100 / total ))
    local filled=$(( percent / 2 ))
    local empty=$(( 50 - filled ))
    local bar_filled=$(printf '#%.0s' $(seq 1 $filled 2>/dev/null))
    local bar_empty=$(printf -- '-%.0s' $(seq 1 $empty 2>/dev/null))
    printf "\r   -> Tiến trình: [%s%s] %3d%% (%d/%d file)" "$bar_filled" "$bar_empty" "$percent" "$current" "$total"
    [ "$current" -eq "$total" ] && echo -e " - Xong!\n"
}

# --- BƯỚC 1: Lấy vị trí và di chuyển ---
SCRIPT_NAME=$(basename "$0")
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
cd "$SCRIPT_DIR" || exit
echo "1. Đã chuyển đến thư mục: $SCRIPT_DIR"

# --- BƯỚC 0: Lựa chọn chế độ ---
echo "----------------------------------------------------"
echo "BƯỚC 0: CHỌN CHẾ ĐỘ XỬ LÝ TÊN FILE"
echo "1. Cắt lấy X ký tự đầu của tên cũ."
echo "2. Đánh số thứ tự mới hoàn toàn (0001, 0002...)."
read -p "Lựa chọn của bạn (1 hoặc 2): " mode_choice

mapfile -t files_to_process < <(ls -v1A | grep -v "^$SCRIPT_NAME$")
total_files=${#files_to_process[@]}

if [ "$total_files" -eq 0 ]; then
    echo "LỖI: Thư mục trống!"
    read -p "Nhấn Enter để thoát..."
    exit 1
fi

# --- BƯỚC 1: Thực hiện Rename ---
if [ "$mode_choice" == "2" ]; then
    padding=${#total_files}
    echo "1. Đang đánh số thứ tự mới (Độ dài: $padding chữ số)..."
    idx=1
    for file in "${files_to_process[@]}"; do
        if [ -f "$file" ]; then
            ext="${file##*.}"
            new_name=$(printf "%0${padding}d.%s" "$idx" "$ext")
            mv "$file" "$new_name"
            idx=$((idx + 1))
            real_progress "$((idx-1))" "$total_files"
        fi
    done
else
    read -p "Nhập số lượng ký tự (X) muốn giữ lại: " x_char
    echo "1. Đang cắt lấy $x_char ký tự đầu..."
    count=0
    for file in "${files_to_process[@]}"; do
        if [ -f "$file" ]; then
            ext="${file##*.}"
            filename="${file%.*}"
            new_name="${filename:0:$x_char}.$ext"
            if [ "$file" != "$new_name" ]; then
                mv "$file" "$new_name"
            fi
            count=$((count + 1))
            real_progress "$count" "$total_files"
        fi
    done
fi

# --- BƯỚC 2: Convert WebP sang JPG (Vá lỗi Stdin) ---
echo "2. Đang kiểm tra và convert WebP sang JPG..."
mapfile -t webp_files < <(ls -1 *.webp 2>/dev/null)
total_webp=${#webp_files[@]}
if [ "$total_webp" -gt 0 ]; then
    current_webp=0
    for file in "${webp_files[@]}"; do
        # Tham số -nostdin chặn FFmpeg phá hỏng luồng nhập liệu của script
        ffmpeg -nostdin -i "$file" "${file%.webp}.jpg" -y -loglevel quiet
        rm "$file"
        current_webp=$((current_webp + 1))
        real_progress "$current_webp" "$total_webp"
    done
else
    echo "   -> Không có file .webp. Bỏ qua."
fi

# --- BƯỚC 3: Tạo Source of Truth ---
TMP_FILE=".danh_sach_tam.txt"
# Tham số -i giúp nhận diện cả đuôi chữ hoa như .JPG, .PNG
ls -v1 | grep -iE '\.(jpg|jpeg|png)$' > "$TMP_FILE"
TOTAL_FINAL=$(wc -l < "$TMP_FILE")

if [ "$TOTAL_FINAL" -eq 0 ]; then
    echo "LỖI: Không tìm thấy file ảnh nào trong thư mục sau khi xử lý."
    rm -f "$TMP_FILE"
    read -p "Nhấn Enter để thoát..."
    exit 1
fi

echo -e "3. Đã chốt danh sách chia folder. Tổng cộng: $TOTAL_FINAL file ảnh.\n"

# --- BƯỚC 4: Chia folder theo QUY TẮC N ---
read -p "4. Nhập số Chương (X): " chapter_x

n=1
while [ $(( (TOTAL_FINAL + n - 1) / n )) -gt 80 ]; do
    n=$((n + 1))
done
files_per_folder=$(( (TOTAL_FINAL + n - 1) / n ))

echo "   -> Phân tích Lý trí: Chia $TOTAL_FINAL file vào $n folder (~$files_per_folder file/folder)."

curr_idx=1
curr_count=0
total_moved=0
folder_name="Chapter ${chapter_x}.${curr_idx}"
mkdir -p "$folder_name"

while IFS= read -r file; do
    if [ "$curr_count" -ge "$files_per_folder" ]; then
        curr_idx=$((curr_idx + 1))
        folder_name="Chapter ${chapter_x}.${curr_idx}"
        mkdir -p "$folder_name"
        curr_count=0
    fi
    mv "$file" "$folder_name/"
    curr_count=$((curr_count + 1))
    total_moved=$((total_moved + 1))
    real_progress "$total_moved" "$TOTAL_FINAL"
done < "$TMP_FILE"

rm -f "$TMP_FILE"
echo "----------------------------------------------------"
echo "HOÀN TẤT THEO QUY TẮC N!"

# --- BƯỚC KHÓA MÀN HÌNH ---
read -p "Tiến trình hoàn thành. Nhấn Enter để đóng cửa sổ..."
