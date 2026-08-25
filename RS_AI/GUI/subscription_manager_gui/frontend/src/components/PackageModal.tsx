/*
[INTEGRITY NOTES]
- Mục đích: Form popup dùng để tạo hoặc cấu hình gói dịch vụ.
- Trách nhiệm: Thu thập thông tin tên, mô tả, số ngày của gói và gửi sự kiện onSave/onClose.
- Tương tác: Dùng trong trang quản lý Package.
*/

import React, { useState, useEffect } from 'react';
import { X } from 'lucide-react';
import type { Package } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

interface PackageModalProps {
    isOpen: boolean;
    packageData: Package | null;
    onClose: () => void;
    onSave: (name: string, durationDays: number, price: number, description?: string) => void;
}

export function PackageModal({ isOpen, packageData, onClose, onSave }: PackageModalProps) {
    const { t } = useTranslation();
    const [name, setName] = useState('');
    const [durationDays, setDurationDays] = useState(30);
    const [price, setPrice] = useState<number>(0);
    const [description, setDescription] = useState('');

    // Khôi phục dữ liệu lên form nếu là chế độ chỉnh sửa
    useEffect(() => {
        if (packageData) {
            setName(packageData.name);
            setDurationDays(packageData.duration_days);
            setPrice(packageData.price || 0);
            setDescription(packageData.description || '');
        } else {
            setName('');
            setDurationDays(30);
            setPrice(0);
            setDescription('');
        }
    }, [packageData, isOpen]);

    if (!isOpen) return null;

    const handleSave = (e: React.FormEvent) => {
        e.preventDefault();
        onSave(name, durationDays, price, description || undefined);
        onClose();
    };

    return (
        <div className="modal-overlay">
            <div className="modal-content animate-fade-in" style={{ maxWidth: '450px' }}>
                <button onClick={onClose} style={{ position: 'absolute', top: '1rem', right: '1rem', background: 'transparent', border: 'none', color: 'white', cursor: 'pointer' }}>
                    <X size={20} />
                </button>
                
                
                <h3>{packageData ? t('packages.modal_edit_title') : t('packages.modal_add_title')}</h3>
                
                <form onSubmit={handleSave}>
                    <div className="form-group">
                        <label className="form-label">{t('packages.lbl_name')}</label>
                        <input 
                            type="text" 
                            className="input-field" 
                            value={name} 
                            onChange={(e) => setName(e.target.value)} 
                            required 
                            placeholder="Ví dụ: Gói Cơ Bản 1 Tháng"
                        />
                    </div>
                    
                    <div className="form-group">
                        <label className="form-label">{t('packages.lbl_duration')}</label>
                        <input 
                            type="number" 
                            className="input-field" 
                            value={durationDays} 
                            onChange={(e) => setDurationDays(Number(e.target.value))} 
                            required 
                            min="1"
                        />
                    </div>
                    
                    <div className="form-group">
                        <label className="form-label">{t('packages.lbl_price')}</label>
                        <input 
                            type="number" 
                            className="input-field" 
                            value={price === 0 ? '' : price} 
                            onChange={(e) => setPrice(Number(e.target.value))} 
                            placeholder="Nhập giá tiền gốc (Ví dụ: 500000)"
                            min="0"
                        />
                    </div>
                    
                    <div className="form-group">
                        <label className="form-label">{t('packages.lbl_desc')}</label>
                        <textarea 
                            className="input-field" 
                            value={description} 
                            onChange={(e) => setDescription(e.target.value)} 
                            placeholder="Nhập thông tin chi tiết về gói..."
                            rows={3}
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
