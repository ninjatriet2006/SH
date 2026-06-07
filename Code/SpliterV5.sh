#!/bin/bash

# =================================================================
# SCRIPT QUY TẮC N v2.5: LÝ TRÍ & ĐA LUỒNG TỐI ĐA
# =================================================================

# --- BƯỚC 0.1: Khởi đầu (Dependencies, Terminal, Thư mục) ---
if ! command -v ffmpeg &> /dev/null || ! command -v ffprobe &> /dev/null; then
    echo "LỖI: Script yêu cầu cài đặt 'ffmpeg' và 'ffprobe'."
    exit 1
fi

printf '\033[8;35;110t'
clear

SCRIPT_NAME=$(basename "$0")
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
cd "$SCRIPT_DIR" || exit 1
echo "1. Đã nhận diện và di chuyển đến: $SCRIPT_DIR"

# --- Hàm cập nhật UI tập trung ---
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

# Lấy danh sách file ban đầu
mapfile -t files_to_process < <(find . -maxdepth 1 -type f ! -name "$SCRIPT_NAME" ! -name ".*" | sed 's|^\./||' | sort -V)
total_files=${#files_to_process[@]}

if [ "$total_files" -eq 0 ]; then
    echo "LỖI: Thư mục trống!"
    exit 1
fi

# --- BƯỚC 1: Đổi tên file ---
echo "----------------------------------------------------"
echo "BƯỚC 1: CHẾ ĐỘ ĐỔI TÊN FILE"
echo "1. Cắt lấy X ký tự đầu."
echo "2. Đánh số thứ tự mới hoàn toàn."
while true; do
    read -p "Lựa chọn của bạn (1/2): " mode_choice
    if [[ "$mode_choice" == "1" || "$mode_choice" == "2" ]]; then break; fi
    echo "Lỗi: Chỉ được nhập 1 hoặc 2."
done

count=0
if [ "$mode_choice" == "2" ]; then
    while true; do
        read -p "Nhập số thứ tự bắt đầu: " start_idx
        if [[ "$start_idx" =~ ^[0-9]+$ ]]; then break; fi
        echo "Lỗi: Vui lòng nhập số."
    done
    padding=${#total_files}
    [ "$padding" -lt 3 ] && padding=3
    
    echo "   -> Đang đánh số thứ tự mới..."
    for file in "${files_to_process[@]}"; do
        if [ -f "$file" ]; then
            ext="${file##*.}"
            new_name=$(printf "%0${padding}d.%s" "$start_idx" "$ext")
            mv "$file" "$new_name"
            start_idx=$((start_idx + 1))
            count=$((count + 1))
            real_progress "$count" "$total_files"
        fi
    done
else
    while true; do
        read -p "Nhập số lượng ký tự (X) muốn giữ lại: " x_char
        if [[ "$x_char" =~ ^[1-9][0-9]*$ ]]; then break; fi
        echo "Lỗi: Nhập số nguyên dương."
    done
    
    echo "   -> Đang cắt chuỗi và xử lý trùng lặp..."
    for file in "${files_to_process[@]}"; do
        if [ -f "$file" ]; then
            ext="${file##*.}"
            filename="${file%.*}"
            base_new="${filename:0:$x_char}"
            
            suffix=""
            counter=1
            # Chống trùng tên (Collision Resolution)
            while [ -e "${base_new}${suffix}.${ext}" ] && [ "${base_new}${suffix}.${ext}" != "$file" ]; do
                suffix="_${counter}"
                counter=$((counter + 1))
            done
            new_name="${base_new}${suffix}.${ext}"
            
            if [ "$file" != "$new_name" ]; then mv "$file" "$new_name"; fi
            count=$((count + 1))
            real_progress "$count" "$total_files"
        fi
    done
fi

# --- BƯỚC 2: Chuẩn hóa định dạng (Convert đa luồng) ---
echo "----------------------------------------------------"
echo "BƯỚC 2: CHUẨN HÓA ĐỊNH DẠNG ẢNH"
echo "Các định dạng hỗ trợ: jpg, png, webp, bmp, tiff..."
while true; do
    read -p "Nhập định dạng đích (VD: jpg): " target_ext
    target_ext=$(echo "$target_ext" | tr '[:upper:]' '[:lower:]')
    if [[ "$target_ext" =~ ^[a-z0-9]+$ ]]; then break; fi
    echo "Lỗi: Định dạng không hợp lệ."
done

# Tìm ảnh có đuôi khác định dạng đích
find . -maxdepth 1 -type f -iregex '.*\.\(jpg\|jpeg\|png\|webp\|avif\|heic\|bmp\|tiff\)$' | grep -iv "\.$target_ext$" > .to_convert.txt
total_conv=$(wc -l < .to_convert.txt)

threads=$(nproc 2>/dev/null || echo 4)
if [ "$total_conv" -gt 0 ]; then
    echo "   -> Chuyển đổi định dạng sang .$target_ext bằng $threads luồng..."
    rm -f .conv_done
    touch .conv_done
    
    cat .to_convert.txt | xargs -d '\n' -P "$threads" -I {} sh -c '
        in="$1"
        out="${in%.*}.'"$target_ext"'"
        ffmpeg -nostdin -i "$in" "$out" -y -loglevel quiet && rm "$in"
        echo "." >> .conv_done
    ' _ {} &
    
    pid=$!
    while kill -0 $pid 2>/dev/null; do
        done_count=$(wc -l < .conv_done 2>/dev/null || echo 0)
        real_progress "$done_count" "$total_conv"
        sleep 0.2
    done
    real_progress "$total_conv" "$total_conv"
    rm -f .to_convert.txt .conv_done
else
    echo "   -> Tất cả file đã ở định dạng .$target_ext. Bỏ qua."
    rm -f .to_convert.txt
fi

# --- BƯỚC 3: Upscale ảnh nhỏ (<600px) ---
echo "----------------------------------------------------"
echo "BƯỚC 3: KIỂM TRA ĐỘ PHÂN GIẢI & UPSCALE"
find . -maxdepth 1 -type f -iregex '.*\.\(jpg\|jpeg\|png\|webp\|avif\|heic\|bmp\|tiff\)$' > .to_upscale.txt
total_up=$(wc -l < .to_upscale.txt)

if [ "$total_up" -gt 0 ]; then
    echo "   -> Đang quét kích thước và upscale (min 600px -> 1280px)..."
    rm -f .up_done
    touch .up_done
    
    cat .to_upscale.txt | xargs -d '\n' -P "$threads" -I {} sh -c '
        f="$1"
        W=$(ffprobe -v error -select_streams v:0 -show_entries stream=width -of csv=s=x:p=0 "$f" 2>/dev/null)
        if [[ "$W" =~ ^[0-9]+$ ]] && [ "$W" -lt 600 ]; then
            out="${f%.*}.upscale.${f##*.}"
            ffmpeg -nostdin -i "$f" -vf "scale=1280:-1" "$out" -y -loglevel quiet && mv "$out" "$f"
        fi
        echo "." >> .up_done
    ' _ {} &
    
    pid=$!
    while kill -0 $pid 2>/dev/null; do
        done_count=$(wc -l < .up_done 2>/dev/null || echo 0)
        real_progress "$done_count" "$total_up"
        sleep 0.2
    done
    real_progress "$total_up" "$total_up"
    rm -f .to_upscale.txt .up_done
fi

# --- BƯỚC 4: Clean-up file lỗi ---
deleted_0kb=$(find . -maxdepth 1 -type f -size 0 -delete -print | wc -l)
if [ "$deleted_0kb" -gt 0 ]; then
    echo "   ! Cảnh báo: Đã xóa $deleted_0kb file lỗi (0KB) phát sinh trong quá trình chuyển đổi."
fi

# --- BƯỚC 5: Source of Truth (Tính tổng chính xác) ---
TMP_FILE=".danh_sach_tam.txt"
find . -maxdepth 1 -type f -iregex '.*\.\(jpg\|jpeg\|png\|webp\|avif\|heic\|bmp\|tiff\)$' | sed 's|^\./||' | sort -V > "$TMP_FILE"
TOTAL_FINAL=$(wc -l < "$TMP_FILE")

if [ "$TOTAL_FINAL" -eq 0 ]; then
    echo "LỖI: Mất toàn bộ file ảnh sau khi xử lý!"
    rm -f "$TMP_FILE"
    exit 1
fi
echo "===================================================="
echo "TỔNG HỢP: Có $TOTAL_FINAL file ảnh hợp lệ sau tiền xử lý."

# Tính n
n=1
while [ $(( (TOTAL_FINAL + n - 1) / n )) -gt 80 ]; do n=$((n + 1)); done
files_per_folder=$(( (TOTAL_FINAL + n - 1) / n ))

echo "   -> Lý trí đề xuất: Chia đều vào $n thư mục (~$files_per_folder file/thư mục)."

# --- BƯỚC 6: Logic Chapter 0 & Tên Thư Mục ---
while true; do
    read -p "Nhập số Chương (X): " chapter_x
    if [[ "$chapter_x" =~ ^[0-9]+$ ]]; then
        if [ "$chapter_x" -eq 0 ]; then
            if [ "$n" -eq 1 ]; then
                is_oneshot=true
                break
            else
                echo "Lỗi: Số lượng file lớn ($TOTAL_FINAL), cần $n folder. Không thể làm Oneshot (Chapter 0 bị khóa). Nhập X > 0."
            fi
        else
            is_oneshot=false
            break
        fi
    else
        echo "Lỗi: Vui lòng nhập số nguyên."
    fi
done

# --- BƯỚC 7: AUTO-INDEX & CHUYỂN FILE ---
if [ "$is_oneshot" = true ]; then
    folder_prefix="Oneshot"
    curr_sub_idx=""
    echo "   -> Tiến hành tạo thư mục: Oneshot"
else
    # Quét thư mục cũ để nối index
    last_sub_idx=$(ls -d "Chapter ${chapter_x}."* 2>/dev/null | sed "s/Chapter ${chapter_x}\.//" | sort -n | tail -1)
    if [ -z "$last_sub_idx" ]; then
        curr_sub_idx=1
        echo "   -> Bắt đầu từ: Chapter ${chapter_x}.1"
    else
        curr_sub_idx=$((last_sub_idx + 1))
        echo "   -> Nối tiếp từ: Chapter ${chapter_x}.${curr_sub_idx}"
    fi
    folder_prefix="Chapter ${chapter_x}."
fi

curr_count=0
total_moved=0

if [ "$is_oneshot" = true ]; then
    folder_name="Oneshot"
else
    folder_name="${folder_prefix}${curr_sub_idx}"
fi
mkdir -p "$folder_name"

while IFS= read -r file; do
    if [ "$curr_count" -ge "$files_per_folder" ] && [ "$is_oneshot" = false ]; then
        curr_sub_idx=$((curr_sub_idx + 1))
        folder_name="${folder_prefix}${curr_sub_idx}"
        mkdir -p "$folder_name"
        curr_count=0
    fi
    mv "$file" "$folder_name/"
    curr_count=$((curr_count + 1))
    total_moved=$((total_moved + 1))
    real_progress "$total_moved" "$TOTAL_FINAL"
done < "$TMP_FILE"

# --- BƯỚC 8: Hoàn tất dọn dẹp ---
rm -f "$TMP_FILE"
echo "----------------------------------------------------"
echo "HOÀN TẤT QUY TRÌNH CHIA FOLDER THEO QUY TẮC N!"
