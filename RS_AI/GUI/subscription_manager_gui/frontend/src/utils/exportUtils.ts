/**
 * Hàm hỗ trợ tải xuống nội dung chuỗi dưới dạng file CSV.
 * Tự động chèn thêm BOM để hỗ trợ Unicode (hiển thị tốt tiếng Việt trong Excel).
 * 
 * @param filename Tên file cần lưu (ví dụ: data.csv)
 * @param csvContent Nội dung CSV dạng chuỗi (đã được nối bằng dấu phẩy và xuống dòng)
 */
export function downloadCSV(filename: string, csvContent: string) {
    // UTF-8 BOM
    const bom = "\uFEFF";
    const fullContent = bom + csvContent;

    const blob = new Blob([fullContent], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.setAttribute("download", filename);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
}
