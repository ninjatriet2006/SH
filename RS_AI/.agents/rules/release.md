# Quy tắc về Quy trình Phát hành (Release Awareness)

- **UNDERSTAND THE INTENT FIRST:** Khi được người dùng yêu cầu "build release" hoặc "chuẩn bị bản release", AI **BẮT BUỘC** phải tự hỏi và tìm kiếm xem người dùng đang ám chỉ quy trình release nào. Không được tự ý giả định và chạy lệnh build gốc của ngôn ngữ (như `cargo tauri build` hay `npm run build`) nếu dự án đã có sẵn các kịch bản/script đóng gói (như `build_release.sh`).
- **USE EXISTING SCRIPTS:** Luôn ưu tiên dùng lệnh tìm kiếm (tìm các file `.sh`, `Makefile`, package scripts) để xem dự án có quy trình xuất bản riêng hay không. Nếu có, phải đọc kỹ script đó để làm theo hoặc hỏi xác nhận người dùng trước khi tự ý thao tác thủ công.
