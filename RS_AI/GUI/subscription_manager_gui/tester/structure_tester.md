[Pattern Docs]
# structure_tester.md

- **Tên hàm**: Test_Flow_Hoan_Thien_User_Modal
- **Mô tả**: Kiểm thử toàn bộ quá trình mở UserModal, nhập thông tin hợp lệ, lưu và xác nhận modal đã đóng.
- **Tham số đầu vào**: 
  - `username` (Bắt buộc): Tên người dùng cần nhập.
  - `email` (Tùy chọn): Email người dùng.
- **Đầu ra**: Modal đóng lại, không còn nằm trong giao diện.

- **Tên hàm**: Test_Flow_Huy_Bo_Reset_State
- **Mô tả**: Mở UserModal, nhập dở thông tin, nhấn Hủy, sau đó mở lại để kiểm tra state đã bị reset trắng chưa (tránh lỗi dính dữ liệu cũ).
- **Tham số đầu vào**: 
  - `username_nhap_do` (Bắt buộc): Dữ liệu nhập thử trước khi Hủy.
- **Đầu ra**: Modal lần 2 mở lên phải có các input hoàn toàn trống rỗng.

- **Tên hàm**: Test_Flow_Click_Outside_And_Switch_Tab
- **Mô tả**: Mở UserModal, cố gắng click ra ngoài backdrop (vùng tối) hoặc click vào nút "Gói dịch vụ" trên Sidebar.
- **Tham số đầu vào**: 
  - Không có (Tùy chọn).
- **Đầu ra**: Modal không bị đóng (do chưa có sự kiện onClick trên backdrop), và tab không thể chuyển (do bị chặn bởi thẻ div z-index: 50).

- **Tên hàm**: Test_Flow_Bo_Trong_Truong_Bat_Buoc
- **Mô tả**: Mở UserModal, cố tình để trống tên hiển thị (trường yêu cầu) và nhấn Lưu.
- **Tham số đầu vào**: 
  - Không có (Tùy chọn).
- **Đầu ra**: Form bắt lỗi required của HTML5, không gọi callback onSave, modal vẫn mở.

- **Tên hàm**: Test_Flow_Hoan_Thien_Package_Modal
- **Mô tả**: Mở PackageModal, nhập thông tin gói hợp lệ, lưu và xác nhận modal đã đóng.
- **Tham số đầu vào**:
  - `name` (Bắt buộc): Tên gói dịch vụ.
  - `durationDays` (Bắt buộc): Thời hạn sử dụng (ngày).
  - `description` (Tùy chọn): Mô tả.
- **Đầu ra**: Modal đóng lại, gói được thêm thành công (do đã fix lỗi backend mapping).

- **Tên hàm**: Test_Flow_Huy_Bo_Reset_State_Package
- **Mô tả**: Mở PackageModal, điền thông tin rác vào tên gói, nhấn Hủy. Mở lại Modal xem dữ liệu đã được reset chưa.
- **Tham số đầu vào**:
  - `name_rac` (Bắt buộc): Chuỗi bất kỳ.
- **Đầu ra**: Khi mở lại, form phải trống, `durationDays` quay về mặc định là 30.

- **Tên hàm**: Test_Flow_Bo_Trong_Truong_Bat_Buoc_Package
- **Mô tả**: Mở PackageModal, để trống trường Tên gói, nhấn Lưu.
- **Tham số đầu vào**: 
  - Không có.
- **Đầu ra**: Trình duyệt báo lỗi HTML5 Validation, Modal không đóng.

---
## Báo cáo kết quả Automation (Playwright)
Toàn bộ 4 Edge Cases của **UserModal** và 3 Edge Cases của **PackageModal** đã được ánh xạ thành code kiểm thử tự động.

**Kết quả Test Run mới nhất**:
**USER MODAL**:
- `Test_Flow_Hoan_Thien_User_Modal`: **[PASSED]** 
- `Test_Flow_Huy_Bo_Reset_State`: **[PASSED]**
- `Test_Flow_Click_Outside_And_Switch_Tab`: **[PASSED]**
- `Test_Flow_Bo_Trong_Truong_Bat_Buoc`: **[PASSED]**

**PACKAGE MODAL**:
- `Test_Flow_Hoan_Thien_Package_Modal`: **[PASSED]** 
- `Test_Flow_Huy_Bo_Reset_State_Package`: **[PASSED]**
- `Test_Flow_Bo_Trong_Truong_Bat_Buoc_Package`: **[PASSED]**
