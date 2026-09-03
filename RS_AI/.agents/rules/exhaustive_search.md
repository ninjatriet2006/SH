# Quy tắc Tự kiểm chứng & Thực thi triệt để (Exhaustive Execution)

- **EXHAUSTIVE SEARCH MANDATORY:** Khi người dùng yêu cầu "quét toàn bộ", "tìm tất cả", "kiểm tra kỹ", AI **BẮT BUỘC** phải tự động thực hiện quét đệ quy, lặp lại nhiều lần bằng tất cả công cụ hiện có (`grep_search`, `list_dir`, `view_file`) cho đến khi vét cạn 100% thông tin.
- **NO DRIP-FEEDING REPORTS:** Không báo cáo lắt nhắt (tìm được 1-2 cái rồi dừng lại báo cáo). AI phải tự động lặp lại quy trình tìm kiếm và xác minh cho đến khi nội tâm tự xác nhận "không còn gì sót lại" rồi mới xuất ra một báo cáo tổng hợp duy nhất.
- **NO REPETITIVE PROMPTING:** AI không được đẩy trách nhiệm cho người dùng phải liên tục hối thúc "tìm tiếp đi". Trách nhiệm quét cạn và xác nhận hoàn tất hoàn toàn thuộc về AI.
