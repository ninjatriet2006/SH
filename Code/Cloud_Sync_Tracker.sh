#!/bin/bash

# ==============================================================================
# 1. CƠ CHẾ MỞ CỬA SỔ (100x15)
# ==============================================================================
if [ "$1" != "--window-ready" ]; then
    gnome-terminal --geometry=100x15 --title="Cloud Sync Tracker" -- bash -c "$0 --window-ready; exec bash"
    exit 0
fi

# ==============================================================================
# 2. CẤU HÌNH ĐƯỜNG DẪN LOG
# ==============================================================================
LOG_FILE="$HOME/cloud_sync.log"

# ==============================================================================
# 3. GIAO DIỆN HIỂN THỊ
# ==============================================================================
draw_header() {
    clear
    echo -e "\e[1;34m====================================================================================================\e[0m"
    echo -e "\e[1;32m                          CLOUD SYNC TRACKER - BẢNG ĐIỀU KHIỂN TĨNH                                 \e[0m"
    echo -e "\e[1;34m====================================================================================================\e[0m"
    echo -e " Trạng thái: \e[33mĐang đồng bộ ngầm (Cập nhật tự động mỗi 1s)\e[0m"
    echo -e " Phím tắt:   \e[31mNhấn Ctrl+C để đóng bảng theo dõi\e[0m"
    echo -e "\e[1;34m----------------------------------------------------------------------------------------------------\e[0m"
}

if [ ! -f "$LOG_FILE" ]; then
    draw_header
    echo -e "\e[31m LỖI: Không tìm thấy file log tại $LOG_FILE \e[0m"
    exit 1
fi

draw_header
echo -e " \e[90mĐang kết nối để lấy nhịp đập (Ping) từ hệ thống... Vui lòng đợi.\e[0m"

# ==============================================================================
# 4. BỘ LỌC TĨNH 1 GIÂY
# ==============================================================================
tail -n 15 -f "$LOG_FILE" | while read -r line; do
    
    # KÍCH HOẠT LÀM MỚI: Mỗi khi Rclone xuất block Stats mới (chứa ETA)
    if [[ "$line" == *"Transferred:"* ]] && [[ "$line" == *"ETA"* ]]; then
        draw_header
        echo -e "\e[1;36m  $line\e[0m"
        
    elif [[ "$line" == *"Transferred:"* ]] && [[ "$line" != *"ETA"* ]]; then
        echo -e "\e[1;36m  $line\e[0m"
        
    elif [[ "$line" == *"*"* ]]; then
        echo -e "\e[1;33m $line\e[0m"
        
    elif [[ "$line" == *"Elapsed time:"* ]]; then
        echo -e "    $line\n"
    fi
done
