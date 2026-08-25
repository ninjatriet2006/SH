# sys/doc_search.rs
Tài liệu tham chiếu các hàm bóc tách văn bản.

- **Tên hàm**: `extract_text`
- **Mô tả**: Đọc và bóc tách nội dung thô từ các định dạng tài liệu phục vụ cho công cụ Full-text Search nội bộ.
  - Xử lý `.pdf` qua CLI `pdftotext`.
  - Xử lý `.docx` và `.epub` bằng cách dùng CLI `unzip` bung các file XML/HTML bên trong.
  - Fallback lại việc đọc text thuần cho các file khác.
- **Tham số đầu vào**:
  - `path: &Path` (Bắt buộc)
- **Đầu ra**: `Option<String>`

- **Tên hàm**: `strip_xml_tags`
- **Mô tả**: (Hàm nội bộ) Hàm phụ trợ để loại bỏ hoàn toàn các thẻ HTML/XML, chỉ giữ lại nội dung hiển thị thật sự.
- **Tham số đầu vào**: `input: &str`
- **Đầu ra**: `String`
