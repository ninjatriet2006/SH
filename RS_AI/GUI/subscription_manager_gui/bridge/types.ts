/*
[INTEGRITY NOTES]
- Mục đích: Định nghĩa các kiểu dữ liệu (Interfaces) TypeScript đồng bộ với các Struct của Backend (Rust).
- Trách nhiệm: Giúp Frontend (React) hiểu và sử dụng đúng kiểu dữ liệu, tránh lỗi Type ở thời gian biên dịch.
- Tương tác: Được Import bởi tất cả các Bridge API và các Component của Frontend.
*/

// Định nghĩa giao diện (Interface) mô tả cấu trúc của một người dùng
export interface User {
    // Mã định danh duy nhất của người dùng
    id: string;
    // Tên đăng nhập / tên hiển thị
    username: string;
    // Địa chỉ email (có thể rỗng hoặc không có)
    email: string | null;
    // Số điện thoại liên hệ (tùy chọn)
    phone: string | null;
    // Đường dẫn liên hệ (Facebook, Zalo,...) (tùy chọn)
    contact_url: string | null;
    // Thời gian tạo tài khoản (lưu dưới dạng timestamp)
    created_at: number;
}

// Định nghĩa giao diện mô tả cấu trúc của một gói dịch vụ
export interface Package {
    // Mã định danh duy nhất của gói
    id: string;
    // Tên của gói dịch vụ
    name: string;
    // Mô tả chi tiết (có thể không có)
    description: string | null;
    // Thời hạn của gói dịch vụ (tính bằng số ngày)
    duration_days: number;
    // Giá tiền của gói (VNĐ)
    price: number;
}

// Định nghĩa giao diện mô tả cấu trúc đăng ký dịch vụ của người dùng
export interface Subscription {
    // Mã định danh duy nhất của đăng ký
    id: string;
    // ID của người dùng sở hữu đăng ký này
    user_id: string;
    // ID của gói dịch vụ được đăng ký
    package_id: string;
    // Thời điểm hết hạn (timestamp)
    expiration_date: number;
    // Trạng thái (đang kích hoạt hay đã vô hiệu hóa)
    is_active: boolean;
}

// Định nghĩa giao diện mô tả cấu trúc Lịch sử giao dịch
export interface Transaction {
    id: string;
    user_id: string;
    package_id: string;
    amount: number;
    action: string;
    created_at: number;
}

// Định nghĩa giao diện Theme
export interface Theme {
    id: string;
    name: string;
    type: 'dark' | 'light';
    colors: Record<string, string>;
}

// Định nghĩa giao diện FontInfo
export interface FontInfo {
    id: string;
    name: string;
    provider: string;
    family: string;
    is_local: boolean;
    src_url: string | null;
}
