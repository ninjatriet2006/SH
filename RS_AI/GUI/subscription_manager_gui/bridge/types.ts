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
    // Số dư (VNĐ), CÓ DẤU: > 0 là tiền khả dụng, < 0 là công nợ.
    balance: number;
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
    // Tự động gia hạn cho riêng đăng ký này (chỉ chạy khi số dư đủ)
    auto_renew: boolean;
    // Thời điểm gia hạn tự động gần nhất; null = chưa từng
    last_auto_renew_at: number | null;
}

// Kết quả một lần rà tự động gia hạn (trả về từ `process_auto_renewals`)
export interface AutoRenewReport {
    // Số đăng ký đã gia hạn thành công
    renewed: number;
    // Tổng tiền đã trừ vào số dư (VNĐ)
    total_charged: number;
    // Các đăng ký bật auto-renew nhưng không gia hạn được, kèm lý do
    skipped: AutoRenewSkip[];
}

export interface AutoRenewSkip {
    subscription_id: string;
    user_id: string;
    package_id: string;
    reason: string;
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

// Mã tra cứu thanh toán — dùng làm NỘI DUNG CHUYỂN KHOẢN trên hóa đơn/QR.
// Dạng: <PREFIX><user_token 4 ký tự><ngẫu nhiên 4 ký tự>, ví dụ "SM7K2MX4B9".
// `user_token` cố định theo khách nên vẫn truy được người chuyển dù mã bị gõ sai.
export interface PaymentRef {
    // Mã đầy đủ, chính là chuỗi khách gõ vào nội dung chuyển khoản
    code: string;
    // 4 ký tự nhận dạng người chuyển (nằm trong `code`)
    user_token: string;
    user_id: string;
    // Tên khách tại thời điểm phát hành (snapshot)
    username: string;
    // Các giao dịch mà mã này thu tiền cho (1 hoặc nhiều khi in gộp)
    transaction_ids: string[];
    // Tổng tiền tại thời điểm phát hành (VNĐ)
    amount: number;
    created_at: number;
    // Thời điểm đối soát xong; null = chưa nhận được tiền
    settled_at: number | null;
}

// Kết quả tra cứu một mã từ sao kê ngân hàng
export interface PaymentLookup {
    // Khớp chính xác mã đã phát hành
    exact: PaymentRef | null;
    // Khi không khớp chính xác: các mã CÙNG người chuyển (theo user_token)
    same_user: PaymentRef[];
    // Token đọc được từ chuỗi đã nhập (rỗng nếu không hợp lệ)
    user_token: string;
}
