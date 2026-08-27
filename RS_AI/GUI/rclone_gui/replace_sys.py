import re

with open("backend/src/core/sys.rs", "r") as f:
    content = f.read()

# Replace struct OSClipboardData
old_struct = """#[derive(Serialize, Deserialize)]
pub struct OSClipboardData {
    // Danh sách các đường dẫn file đã copy/cut
    pub paths: Vec<String>,
    // Cờ đánh dấu là hành động Cắt (true) hay Copy (false)
    pub is_cut: bool,
}"""

new_struct = """#[derive(Serialize, Deserialize, Clone)]
pub struct OSClipboardItem {
    pub pane: String,
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct OSClipboardData {
    pub items: Vec<OSClipboardItem>,
    pub is_cut: bool,
}"""

content = content.replace(old_struct, new_struct)

old_fn = """pub async fn os_clipboard_set(paths: Vec<String>, is_cut: bool) -> Result<(), String> {
    // Khởi tạo đối tượng dữ liệu clipboard
    let data = OSClipboardData { paths, is_cut };"""

new_fn = """pub async fn os_clipboard_set(items: Vec<OSClipboardItem>, is_cut: bool) -> Result<(), String> {
    let data = OSClipboardData { items, is_cut };"""

content = content.replace(old_fn, new_fn)

with open("backend/src/core/sys.rs", "w") as f:
    f.write(content)
print("Success")
