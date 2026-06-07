#!/bin/bash

# --- Cấu hình ---
# Bạn có thể thêm các ứng dụng khác vào đây theo cú pháp: "Tên_Hiển_Thị" "Lệnh_Flatpak_ID"
APPS=("Fcitx5" "org.fcitx.Fcitx5")

# --- Giao diện ---
clear
echo "======================================"
echo "    HỆ THỐNG QUẢN LÝ ỨNG DỤNG CLI"
echo "======================================"
echo "Chọn ứng dụng muốn can thiệp:"
echo "1) ${APPS[0]} (Flatpak)"
# Sau này thêm 2) App khác ở đây
echo "--------------------------------------"
read -p "Nhập lựa chọn (mặc định 1): " APP_CHOICE
APP_CHOICE=${APP_CHOICE:-1}

if [ "$APP_CHOICE" -eq 1 ]; then
    APP_ID=${APPS[1]}
    APP_NAME=${APPS[0]}
else
    echo "Lựa chọn không hợp lệ!"
    exit 1
fi

echo -e "\nBạn muốn làm gì với $APP_NAME?"
echo "1) Tắt (Kill)"
echo "2) Bật (Run)"
echo "3) Khởi động lại (Restart)"
read -p "Lựa chọn: " ACTION

case $ACTION in
    1)
        echo "Đang tắt $APP_NAME..."
        flatpak kill $APP_ID
        ;;
    2)
        echo "Đang bật $APP_NAME..."
        flatpak run $APP_ID -rd
        ;;
    3)
        echo "Đang khởi động lại $APP_NAME..."
        flatpak kill $APP_ID
        sleep 1
        flatpak run $APP_ID -rd
        ;;
    *)
        echo "Lệnh không hợp lệ!"
        ;;
esac

echo "======================================"
echo "Xử lý hoàn tất!"
