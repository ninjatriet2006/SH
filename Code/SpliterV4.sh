#!/bin/bash

# =================================================================
# SCRIPT QUY TẮC N: ĐA LUỒNG, AUTO-INDEX & AUTO-UPSCALE (1280PX)
# =================================================================

# --- BƯỚC 0.1: Cấu hình kích thước Terminal ---
printf '\033[8;35;110t'
clear

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

# --- BƯỚC 2: Lựa chọn chế độ ---
echo "----------------------------------------------------"
echo "BƯỚC 0: CHỌN CHẾ ĐỘ XỬ LÝ TÊN FILE"
echo "1. Cắt lấy X ký tự đầu của tên cũ."
echo "2. Đánh số thứ tự mới hoàn toàn."
read -p "Lựa chọn của bạn (1 hoặc 2): " mode_choice

mapfile -t files_to_process < <(ls -v1A | grep -v "^$SCRIPT_NAME$")
total_files=${#files_to_process[@]}

if [ "$total_files" -eq 0 ]; then
    echo "LỖI: Thư mục trống!"
    read -p "Nhấn Enter để thoát..."
    exit 1
fi

# --- BƯỚC 3: Thực hiện Rename ---
if [ "$mode_choice" == "2" ]; then
    padding=${#total_files}
    read -p "Nhập số thứ tự bắt đầu (Mặc định là 1): " start_idx
    if ! [[ "$start_idx" =~ ^[0-9]+$ ]]; then start_idx=1; fi
    
    echo "1. Đang đánh số thứ tự mới (Bắt đầu từ $start_idx)..."
    idx=$start_idx
    count=0 
    for file in "${files_to_process[@]}"; do
        if [ -f "$file" ]; then
            ext="${file##*.}"
            new_name=$(printf "%0${padding}d.%s" "$idx" "$ext")
            mv "$file" "$new_name"
            idx=$((idx + 1))
            count=$((count + 1))
            real_progress "$count" "$total_files"
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
            if [ "$file" != "$new_name" ]; then mv "$file" "$new_name"; fi
            count=$((count + 1))
            real_progress "$count" "$total_files"
        fi
    done
fi

# --- BƯỚC 4: Convert WebP sang JPG (ĐA LUỒNG) ---
echo "2. Đang kiểm tra và convert WebP sang JPG..."
mapfile -t webp_files < <(find . -maxdepth 1 -name "*.webp" -exec basename {} \;)
total_webp=${#webp_files[@]}

if [ "$total_webp" -gt 0 ]; then
    threads=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
    # Loại bỏ -n 1 để dọn dẹp cảnh báo xargs
    printf "%s\n" "${webp_files[@]}" | xargs -P "$threads" -I {} sh -c 'ffmpeg -nostdin -i "$1" "${1%.webp}.jpg" -y -loglevel quiet && rm "$1"' _ {} &
    xargs_pid=$!
    while kill -0 $xargs_pid 2>/dev/null; do
        remaining=$(find . -maxdepth 1 -name "*.webp" | wc -l)
        real_progress "$((total_webp - remaining))" "$total_webp"
        sleep 0.2
    done
    real_progress "$total_webp" "$total_webp"
else
    echo "   -> Không có file .webp. Bỏ qua."
fi

# --- BƯỚC 5: AUTO-UPSCALE (Chiều rộng < 600px -> 1280px) ---
echo "3. Đang kiểm tra độ phân giải (Yêu cầu min 600px)..."
# Quét toàn bộ file ảnh hiện có
mapfile -t all_images < <(find . -maxdepth 1 -type f -iregex '.*\.\(jpg\|jpeg\|png\)$' -exec basename {} \;)
total_img=${#all_images[@]}

if [ "$total_img" -gt 0 ]; then
    threads=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
    
    # Logic: Dùng ffprobe kiểm tra width, nếu < 600 thì ffmpeg upscale
    printf "%s\n" "${all_images[@]}" | xargs -P "$threads" -I {} sh -c '
        W=$(ffprobe -v error -select_streams v:0 -show_entries stream=width -of csv=s=x:p=0 "$1")
        if [ "$W" -lt 600 ]; then
            ffmpeg -nostdin -i "$1" -vf "scale=1280:-1" "${1%.*}.upscale.jpg" -y -loglevel quiet && mv "${1%.*}.upscale.jpg" "$1"
        fi
    ' _ {} &
    
    upscale_pid=$!
    # Vì việc upscale diễn ra bên trong xargs, ta theo dõi tiến trình qua số lượng file đã xử lý xong trong danh sách
    # Ở đây dùng một cơ chế đếm file tạm hoặc đơn giản là đợi xargs kết thúc
    echo "   -> Đang quét và xử lý ảnh nhỏ bằng $threads luồng..."
    wait $upscale_pid
    echo "   -> Hoàn tất hậu kiểm độ phân giải."
else
    echo "   -> Không có file ảnh để kiểm tra. Bỏ qua."
fi

# --- BƯỚC 6: Tạo Source of Truth ---
TMP_FILE=".danh_sach_tam.txt"
find . -maxdepth 1 -type f -iregex '.*\.\(jpg\|jpeg\|png\)$' | sed 's/^\.\///' | sort -V > "$TMP_FILE"
TOTAL_FINAL=$(wc -l < "$TMP_FILE")

if [ "$TOTAL_FINAL" -eq 0 ]; then
    echo "LỖI: Không tìm thấy file ảnh!"
    rm -f "$TMP_FILE"
    read -p "Nhấn Enter để thoát..."
    exit 1
fi
echo -e "4. Tổng cộng: $TOTAL_FINAL file ảnh sau xử lý.\n"

# --- BƯỚC 7: Chia folder (AUTO-INDEX) ---
read -p "5. Nhập số Chương (X): " chapter_x

last_sub_idx=$(ls -d "Chapter ${chapter_x}."* 2>/dev/null | sed "s/Chapter ${chapter_x}\.//" | sort -n | tail -1)

if [ -z "$last_sub_idx" ]; then
    curr_sub_idx=1
    echo "   -> Bắt đầu từ Chapter ${chapter_x}.1"
else
    curr_sub_idx=$((last_sub_idx + 1))
    echo "   -> Tiếp tục từ Chapter ${chapter_x}.${curr_sub_idx}"
fi

n=1
while [ $(( (TOTAL_FINAL + n - 1) / n )) -gt 80 ]; do n=$((n + 1)); done
files_per_folder=$(( (TOTAL_FINAL + n - 1) / n ))

echo "   -> Phân tích Lý trí: Chia $TOTAL_FINAL file vào $n folder mới (~$files_per_folder file/folder)."

curr_count=0
total_moved=0
folder_name="Chapter ${chapter_x}.${curr_sub_idx}"
mkdir -p "$folder_name"

while IFS= read -r file; do
    if [ "$curr_count" -ge "$files_per_folder" ]; then
        curr_sub_idx=$((curr_sub_idx + 1))
        folder_name="Chapter ${chapter_x}.${curr_sub_idx}"
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
read -p "Nhấn Enter để đóng..."
