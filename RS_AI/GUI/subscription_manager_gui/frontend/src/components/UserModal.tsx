/*
[INTEGRITY NOTES]
- Mục đích: Form popup dùng để thêm mới hoặc chỉnh sửa thông tin một người dùng.
- Trách nhiệm: Hiển thị form, lấy dữ liệu nhập vào từ người dùng và gọi các sự kiện onSave hoặc onClose.
- Tương tác: Dùng trong trang quản lý người dùng (UserManagementPage).
*/

import React, { useState, useEffect } from 'react';
import { X } from 'lucide-react';
import type { User } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

interface UserModalProps {
    isOpen: boolean;
    userData: User | null;
    onClose: () => void;
    onSave: (username: string, email?: string, phone?: string, contactUrl?: string) => void;
}

export function UserModal({ isOpen, userData, onClose, onSave }: UserModalProps) {
    const { t } = useTranslation();
    
    // Trạng thái cục bộ cho form
    const [username, setUsername] = useState('');
    const [email, setEmail] = useState('');
    const [phone, setPhone] = useState('');
    const [contactUrl, setContactUrl] = useState('');

    // Mỗi khi modal mở lên hoặc dữ liệu thay đổi, cập nhật lại form
    useEffect(() => {
        if (userData) {
            setUsername(userData.username);
            setEmail(userData.email || '');
            setPhone(userData.phone || '');
            setContactUrl(userData.contact_url || '');
        } else {
            setUsername('');
            setEmail('');
            setPhone('');
            setContactUrl('');
        }
    }, [userData, isOpen]);

    // Nếu modal không mở thì ẩn đi
    if (!isOpen) return null;

    // Xử lý khi nhấn lưu
    const handleSave = (e: React.FormEvent) => {
        e.preventDefault();
        onSave(username, email || undefined, phone || undefined, contactUrl || undefined);
        onClose();
    };

    return (
        <div className="modal-overlay">
            <div className="modal-content animate-fade-in" style={{ maxWidth: '400px' }}>
                <button onClick={onClose} style={{ position: 'absolute', top: '1rem', right: '1rem', background: 'transparent', border: 'none', color: 'white', cursor: 'pointer' }}>
                    <X size={20} />
                </button>
                
                
                <h3>{userData ? t('users.modal_edit_title') : t('users.modal_add_title')}</h3>
                
                <form onSubmit={handleSave}>
                    <div className="form-group">
                        <label className="form-label">{t('users.lbl_username')}</label>
                        <input 
                            type="text" 
                            className="input-field" 
                            value={username} 
                            onChange={(e) => setUsername(e.target.value)} 
                            required 
                            placeholder="Nhập tên người dùng..."
                        />
                    </div>
                    
                    <div className="form-group">
                        <label className="form-label">{t('users.lbl_email')}</label>
                        <input 
                            type="email" 
                            className="input-field" 
                            value={email} 
                            onChange={(e) => setEmail(e.target.value)} 
                            placeholder="Nhập địa chỉ email..."
                        />
                    </div>
                    
                    <div className="form-group">
                        <label className="form-label">{t('users.lbl_phone')}</label>
                        <input 
                            type="tel" 
                            className="input-field" 
                            value={phone} 
                            onChange={(e) => setPhone(e.target.value)} 
                            placeholder="Nhập số điện thoại..."
                        />
                    </div>
                    
                    <div className="form-group">
                        <label className="form-label">{t('users.lbl_contact')}</label>
                        <input 
                            type="url" 
                            className="input-field" 
                            value={contactUrl} 
                            onChange={(e) => setContactUrl(e.target.value)} 
                            placeholder="VD: https://facebook.com/..."
                        />
                    </div>

                    <div className="modal-actions">
                        <button type="button" className="btn btn-danger" onClick={onClose}>{t('common.cancel')}</button>
                        <button type="submit" className="btn btn-primary">{t('common.save')}</button>
                    </div>
                </form>
            </div>
        </div>
    );
}
