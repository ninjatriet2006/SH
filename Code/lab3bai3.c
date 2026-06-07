#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

void *thread_print(void *threadid) {
    long tid = (long)threadid;
    printf("Hello IT007! I'm Detached Thread #%ld\n", tid);
    sleep(2);
    printf("Thread #%ld is exiting and auto-releasing resources.\n", tid);
    pthread_exit(NULL);
}

int main() {
    pthread_t thread;
    pthread_attr_t attr; // Khai báo biến thuộc tính
    int check;
    long tID = 1;

    // 1. Khởi tạo đối tượng thuộc tính với giá trị mặc định
    pthread_attr_init(&attr);

    // 2. Thiết lập thuộc tính Detach (gỡ tiểu trình) cho attr
    // Thuộc tính PTHREAD_CREATE_DETACHED giúp tiểu trình tự động thu hồi tài nguyên sau khi xong
    pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_DETACHED);

    printf("I'm Main Thread: creating detached Thread.\n");
    
    // 3. Truyền biến attr vào hàm tạo tiểu trình thay vì dùng NULL
    check = pthread_create(&thread, &attr, thread_print, (void *)tID);
    
    if (check != 0) {
        printf("ERROR!!! Can't create thread.\n");
        exit(-1);
    }

    // 4. Hủy đối tượng thuộc tính sau khi đã tạo tiểu trình xong (không ảnh hưởng tới tiểu trình đang chạy)
    pthread_attr_destroy(&attr);

    // Vì tiểu trình chạy độc lập, main thread phải sleep một chút để kịp thấy kết quả
    // trước khi main kết thúc (vì ta không thể dùng pthread_join với detached thread)
    sleep(3); 
    printf("Main thread exiting.\n");
    
    return 0;
}