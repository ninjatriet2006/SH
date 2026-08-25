

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
    confirmText = "Xác nhận",
    cancelText = "Hủy",
    isDanger = false
}: ConfirmModalProps) {
    if (!isOpen) return null;

    return (
        <div className="modal-overlay">
            <div className="modal-content" style={{ maxWidth: '400px' }}>
                <h2 style={{ marginTop: 0, color: isDanger ? '#ef4444' : 'inherit' }}>{title}</h2>
                <p style={{ margin: '1rem 0 2rem 0', color: 'var(--text-secondary)', lineHeight: '1.5' }}>
                    {message}
                </p>
                
                <div className="modal-actions" style={{ justifyContent: 'flex-end', marginTop: '1.5rem' }}>
                    <button className="btn" onClick={onCancel}>
                        {cancelText}
                    </button>
                    <button 
                        className="btn btn-primary" 
                        style={isDanger ? { background: '#ef4444', border: 'none' } : {}} 
                        onClick={onConfirm}
                    >
                        {confirmText}
                    </button>
                </div>
            </div>
        </div>
    );
}
