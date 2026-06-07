// ==UserScript==
// @name         Tool Đăng Ký Học Phần HCMUS
// @namespace    http://tampermonkey.net/
// @version      v1.1
// @description  Tự động tải lại trang và chọn môn để đăng ký học phần
// @author       You
// @match        https://*.hcmus.edu.vn/*DangKyHocPhan.aspx*
// @icon         https://www.google.com/s2/favicons?sz=64&domain=tampermonkey.net
// @grant        none
// ==/UserScript==

(function () {
    'use strict';

    // Danh sách các môn cần đăng ký (Mã môn: Mã lớp)
    const expectedCourses = {
        "BAA00003": "24DTV_DKD3", // Tư tưởng Hồ Chí Minh (Thứ 2, tiết 1-6)
        "ETC10234": "24DTV_DKD3", // Cấu trúc dữ liệu và giải thuật (Thứ 2, tiết 7-9)
        "ETC00021": "24DTV_DKD1", // Cảm biến, đo, máy đo (Thứ 3, tiết 2-4)
        "ETC00085": "24DTV_DKD3", // Thực hành Cảm biến, đo, máy đo (Thứ 3, tiết 7-12)
        "ETC00002": "25DTV_DKD2", // Điện tử số (Thứ 4, tiết 1-3)
        "ETC10235": "24DTV_DK1A", // Thực hành Cấu trúc dữ liệu và giải thuật (Thứ 4, tiết 4-6)
        "ETC10013": "24DTV_DKD2", // Xử lý tín hiệu số (Thứ 4, tiết 7-12)
        "ETC10014": "24DTV_DKD1"  // Thực hành xử lý tín hiệu số (Chuyển sang Thứ 6, tiết 1-6)
    };

    // Thời gian tự động làm mới trang (tính bằng giây)
    const REFRESH_INTERVAL_SECONDS = 10;

    function register() {
        // Lấy danh sách các dòng trong bảng môn học (bỏ qua bảng môn đã đăng ký)
        const courses = document.querySelectorAll("table:not(#tbDSDaDK) tbody tr");
        // Lấy nút submit thứ 2 (nút "Lưu đăng ký")
        const submitButtons = document.querySelectorAll("input[type='submit']");
        const submitButton = submitButtons.length > 1 ? submitButtons[1] : null;

        if (!courses || courses.length === 0) {
            return false;
        }

        let foundAndChecked = false;

        for (const course of courses) {
            const idElement = course.querySelector("td:nth-of-type(1)");
            const classIdElement = course.querySelector("td:nth-of-type(3)");
            const checkboxElement = course.querySelector("td:last-of-type input[type='checkbox']");

            if (idElement && classIdElement && checkboxElement) {
                const id = idElement.textContent.trim();
                const classId = classIdElement.textContent.trim();

                // Nếu môn học và lớp nằm trong danh sách mong muốn và chưa được tick
                if (expectedCourses[id] === classId && !checkboxElement.checked) {
                    checkboxElement.checked = true;
                    foundAndChecked = true;
                    console.log(`[ĐKHP] Đã tự động chọn môn: ${id} - ${classId}`);
                }
            }
        }

        if (foundAndChecked) {
            if (submitButton) {
                console.log("[ĐKHP] Đang tiến hành Lưu đăng ký...");
                // Ghi đè hàm confirm để tự động bỏ qua hộp thoại xác nhận
                const originalConfirm = window.confirm;
                window.confirm = function () {
                    window.confirm = originalConfirm; // Trả lại hàm confirm gốc
                    return true;
                };

                submitButton.click();
                return true; // Đã submit, không cần reload bằng setTimeout
            } else {
                console.error("[ĐKHP] Không tìm thấy nút Lưu đăng ký!");
            }
        }

        return false; // Không chọn được môn nào, cần reload lại trang
    }

    // Thực thi hàm đăng ký
    const isSubmitted = register();

    // CHỈ làm mới trang khi KHÔNG có hành động submit.
    // Nếu đã submit, trang web sẽ tự động chuyển hướng. Reload lúc này có thể làm hủy request.
    if (!isSubmitted) {
        setTimeout(() => {
            console.log(`[ĐKHP] Đang làm mới trang sau ${REFRESH_INTERVAL_SECONDS} giây...`);
            window.location.reload();
        }, REFRESH_INTERVAL_SECONDS * 1000);
    }

})();