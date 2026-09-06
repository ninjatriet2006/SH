/*
[INTEGRITY NOTES]
- Mục đích: Hộp thoại xác nhận dùng chung cho các thao tác không hoàn tác được.
- Trách nhiệm: Hiển thị tiêu đề/nội dung và trả về lựa chọn của người dùng.
- Tương tác: các trang có thao tác xoá.
*/

interface ConfirmModalProps {
    isOpen: boolean;
    title: string;
    message: string;
    onConfirm: () => void;
    onCancel: () => void;
    confirmText?: string;
    cancelText?: string;
    isDanger?: boolean;
}

export function ConfirmModal({
    isOpen,
    title,
    message,
    onConfirm,
    onCancel,
    confirmText = 'Xác nhận',
    cancelText = 'Hủy',
    isDanger = false,
}: ConfirmModalProps) {
    if (!isOpen) return null;

    return (
        <div className="modal-overlay">
            <div className="modal-content" style={{ maxWidth: '440px' }}>
                <h2 style={{ marginTop: 0, color: isDanger ? 'var(--danger)' : 'inherit' }}>{title}</h2>
                <p style={{ margin: '1rem 0 2rem 0', color: 'var(--text-secondary)', lineHeight: 1.5 }}>
                    {message}
                </p>

                <div className="modal-actions" style={{ justifyContent: 'flex-end', marginTop: '1.5rem' }}>
                    <button className="btn" onClick={onCancel}>{cancelText}</button>
                    <button
                        className="btn btn-primary"
                        style={isDanger ? { background: 'var(--danger)', border: 'none' } : {}}
                        onClick={onConfirm}
                    >
                        {confirmText}
                    </button>
                </div>
            </div>
        </div>
    );
}
