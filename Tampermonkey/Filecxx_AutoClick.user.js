// ==UserScript==
// @name         Auto Click Yes FileCxx (33423)
// @namespace    http://tampermonkey.net/
// @version      1.0
// @description  Tự động click nút Yes dành riêng cho trang https://w.filecxx.com/33423
// @author       Bạn
// @match        https://w.filecxx.com/33423
// @grant        none
// ==/UserScript==

(function() {
    'use strict';

    // Hàm thực hiện tìm và click
    function autoClickYes() {
        // Tìm tất cả các thẻ có khả năng là nút
        const elements = document.querySelectorAll('button, a, div, span, input[type="button"], input[type="submit"]'); 
        
        for (let el of elements) {
            // Kiểm tra xem chữ hiển thị trên nút có phải là "Yes" không
            const text = el.innerText || el.value;
            if (text && text.trim().toLowerCase() === 'yes') {
                console.log("🚀 Đã tìm thấy nút Yes, tiến hành click...");
                el.click();
                return true; // Click xong thì trả về true
            }
        }
        return false; // Chưa tìm thấy
    }

    // Chạy thử ngay khi trang vừa nạp xong HTML
    window.addEventListener('DOMContentLoaded', () => {
        if (!autoClickYes()) {
            // Cài đặt một bộ theo dõi (MutationObserver) để chực chờ khi nào nút hiện ra là click ngay
            const observer = new MutationObserver(() => {
                if (autoClickYes()) {
                    observer.disconnect(); // Click được rồi thì tắt bộ theo dõi đi
                }
            });
            
            if (document.body) {
                observer.observe(document.body, { childList: true, subtree: true });
            }
        }
    });

    // Dự phòng: Chạy thêm 1 lần nữa sau 2 giây
    setTimeout(autoClickYes, 2000);

})();
