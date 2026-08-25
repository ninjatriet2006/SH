//! [INTEGRITY NOTES]
//! Mục đích: Trích xuất văn bản thô từ nhiều định dạng tài liệu phục vụ tìm kiếm toàn văn.
//! Trách nhiệm: Sử dụng các công cụ CLI hệ thống (pdftotext, unzip) để đọc file PDF, DOCX, EPUB, hoặc dự phòng bằng đọc văn bản thuần.
//! Tương tác: Giao tiếp với sys/mod.rs

use std::path::Path;
use std::process::Command;
use std::fs;

/// Trích xuất văn bản từ nhiều định dạng file tài liệu phục vụ cho tính năng search.
pub fn extract_text(path: &Path) -> Option<String> {
    // Lấy phần mở rộng (đuôi) của file chuyển sang chữ thường
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    
    // Xử lý luồng theo từng loại đuôi file
    match ext.as_str() {
        "pdf" => {
            // [TODO]: Hiện tại đang gọi công cụ ngoài qua Terminal (pdftotext).
            // Tương lai có thể viết thêm logic _integrate dùng crate nội bộ thay thế để tăng tính độc lập.
            // Sử dụng công cụ dòng lệnh pdftotext để đọc file pdf và xuất ra luồng stdout (tham số -)
            let output = Command::new("pdftotext")
                .arg(path)
                .arg("-")
                .output()
                .ok()?;
            // Kiểm tra trạng thái tiến trình
            if output.status.success() {
                // Chuyển đổi dữ liệu nhị phân trả về thành chuỗi ký tự hợp lệ
                Some(String::from_utf8_lossy(&output.stdout).into_owned())
            } else {
                None
            }
        },
        "docx" => {
            // [TODO]: Hiện tại đang gọi công cụ ngoài qua Terminal (unzip).
            // Tương lai có thể tích hợp crate `zip` để giải nén nội bộ thay vì phụ thuộc HĐH.
            // Bản chất docx là file zip chứa xml, ta giải nén trích xuất file word/document.xml
            let output = Command::new("unzip")
                .arg("-p") // Cờ -p yêu cầu in thẳng nội dung ra stdout
                .arg(path)
                .arg("word/document.xml")
                .output()
                .ok()?;
            if output.status.success() {
                let xml = String::from_utf8_lossy(&output.stdout);
                // Lọc bỏ mã XML chỉ giữ lại văn bản hiển thị
                Some(strip_xml_tags(&xml))
            } else {
                None
            }
        },
        "epub" => {
            // [TODO]: Hiện tại đang gọi công cụ ngoài qua Terminal (unzip).
            // Tương tự docx, epub cũng là file zip, ta giải nén toàn bộ các file .html và .xhtml
            let output = Command::new("unzip")
                .arg("-p")
                .arg(path)
                .arg("*.html")
                .arg("*.xhtml")
                .output()
                .ok()?;
            if output.status.success() {
                let xml = String::from_utf8_lossy(&output.stdout);
                // Lọc thẻ XML tương tự
                Some(strip_xml_tags(&xml))
            } else {
                None
            }
        },
        _ => {
            // Fallback: Đọc thẳng bằng std::fs đối với các định dạng văn bản thô (.txt, .md, .ini,...)
            fs::read_to_string(path).ok()
        }
    }
}

/// Hàm hỗ trợ nội bộ: Quét qua chuỗi và loại bỏ tất cả các thẻ cấu trúc XML/HTML (ví dụ <b>, <i>, <tag>...)
fn strip_xml_tags(input: &str) -> String {
    // Khởi tạo bộ nhớ tạm để chứa văn bản sạch, dung lượng dự kiến bằng nửa ban đầu
    let mut out = String::with_capacity(input.len() / 2);
    // Biến cờ (flag) đánh dấu con trỏ có đang nằm lọt trong một thẻ tag hay không
    let mut in_tag = false;
    
    // Duyệt qua từng ký tự một
    for c in input.chars() {
        if c == '<' {
            in_tag = true; // Ký hiệu bắt đầu thẻ tag
        } else if c == '>' {
            in_tag = false; // Ký hiệu kết thúc thẻ tag
        } else if !in_tag {
            // Nếu không nằm trong thẻ thì đây là nội dung thực tế, đẩy vào chuỗi kết quả
            out.push(c);
        }
    }
    out
}
