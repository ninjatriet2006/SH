# Quy tắc Hỏi Rõ Ràng (Underspecified Requirements)

- **NO ASSUMPTIONS ON AMBIGUITY:** Nếu yêu cầu của người dùng thiếu các chi tiết quan trọng (ví dụ: edge cases, thông số API, hoặc phương án dự phòng khi lỗi), AI **BẮT BUỘC** phải chủ động hỏi rõ để xác nhận lại trước khi viết code. Không được tự ý đưa ra giả định.
- **EXACTNESS IN LOGIC:** Đối với các tính năng mới liên quan đến luồng dữ liệu phức tạp, tuyệt đối không đoán mò cấu trúc. Hãy đề xuất một cấu trúc và yêu cầu người dùng xác nhận.
