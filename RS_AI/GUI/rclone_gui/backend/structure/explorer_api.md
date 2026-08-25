[Pattern Docs]
# explorer_api.md

- **Tên hàm**: `list_remote`
- **Mô tả**: Lấy danh sách thư mục và file từ một remote, hỗ trợ phân trang và tùy chọn hiển thị metadata đặc thù của cloud.
- **Tham số đầu vào**:
  - `remote_path` (Bắt buộc): Đường dẫn dạng `Remote:Path`.
  - `recurse` (Tùy chọn): true/false để quét đệ quy thư mục.
  - `show_metadata` (Tùy chọn): true/false để lấy metadata (VD: shortcut.dangling).
- **Đầu ra**: Mảng đối tượng JSON (FileNode) đại diện cho danh sách file/thư mục.

- **Tên hàm**: `file_operations`
- **Mô tả**: Xử lý đa tác vụ cơ bản (Copy, Move, Delete, Mkdir, Rename) trên File/Folder. Hỗ trợ batch bằng cách truyền danh sách mảng.
- **Tham số đầu vào**:
  - `action_type` (Bắt buộc): enum (copy, move, delete, mkdir, rename).
  - `source_paths` (Bắt buộc): Mảng đường dẫn nguồn.
  - `dest_path` (Tùy chọn): Đường dẫn đích (Bắt buộc nếu là copy/move).
- **Đầu ra**: ID của Job bất đồng bộ để tiện theo dõi tiến trình (nếu là copy/move), hoặc kết quả tức thì (nếu delete/mkdir).

- **Tên hàm**: `get_public_link`
- **Mô tả**: Kế thừa từ TUI, dùng để gọi lệnh public link của rclone cho một tệp tin/thư mục cụ thể.
- **Tham số đầu vào**:
  - `remote_path` (Bắt buộc): Đường dẫn đến tệp/thư mục.
- **Đầu ra**: Chuỗi String chứa đường dẫn công khai (URL) hoặc lỗi nếu Remote không hỗ trợ.

- **Tên hàm**: `clear_vfs_cache`
- **Mô tả**: Xóa bộ đệm fscache/clear của thư mục hoặc toàn bộ remote để buộc rclone phải cập nhật mới dữ liệu.
- **Tham số đầu vào**:
  - `remote_path` (Tùy chọn): Tên remote hoặc path để clear (để trống sẽ clear toàn bộ).
- **Đầu ra**: Kết quả thành công hoặc thất bại.

- **Tên hàm**: `hash_check`
- **Mô tả**: So sánh hoặc trích xuất mã băm (MD5, SHA1) của file trên remote để đối chiếu tính toàn vẹn (từ ModularTUI).
- **Tham số đầu vào**:
  - `file_path` (Bắt buộc): Đường dẫn file.
  - `hash_type` (Tùy chọn): md5, sha1, v.v.
- **Đầu ra**: Chuỗi Hash.
