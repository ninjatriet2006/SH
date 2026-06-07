#include <stdlib.h>
#include <signal.h>

// Hàm này chỉ thực hiện đúng yêu cầu c: Tắt vim
void tat_vim(int sig) {
    // Gọi Hệ điều hành quét tên và tự tiêu diệt tiến trình vim, không cần giả định PID
    system("pkill vim"); 
    
    // Thoát chương trình sau khi hoàn thành nhiệm vụ
    exit(0);
}

int main() {
    // Đăng ký: Khi nhận tín hiệu SIGINT từ phím Ctrl+C, sẽ gọi hàm tat_vim
    signal(SIGINT, tat_vim);
    
    // Vòng lặp rỗng giữ chương trình hoạt động để chờ bạn nhấn phím
    while(1) {}
    
    return 0;
}