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
    onSave: (name: string, durationDays: number, price: number, description?: string) => Promise<void>;
}

export function PackageModal({ isOpen, packageData, onClose, onSave }: PackageModalProps) {
    const { t } = useTranslation();
    const [name, setName] = useState('');
    const [durationDays, setDurationDays] = useState(30);
    // Giữ raw chuỗi để phân biệt "bỏ trống" với nhập 0 — trước đây
    // `Number('') === 0` nên xóa trắng ô giá vẫn lưu 0 mà không hay biết.
    const [priceInput, setPriceInput] = useState<string>('0');
    const [description, setDescription] = useState('');

    // Khôi phục dữ liệu lên form nếu là chế độ chỉnh sửa
    useEffect(() => {
        if (packageData) {
            setName(packageData.name);
            setDurationDays(packageData.duration_days);
            setPriceInput(String(packageData.price ?? 0));
            setDescription(packageData.description || '');
        } else {
            setName('');
            setDurationDays(30);
            setPriceInput('0');
            setDescription('');
        }
    }, [packageData, isOpen]);

    if (!isOpen) return null;

    const handleSave = async (e: React.FormEvent) => {
        e.preventDefault();
        // Ô number bỏ trống cho NaN — chặn trước khi gửi sang backend (u32/u64).
        if (!name.trim()) {
            alert("Tên gói không được để trống!");
            return;
        }
        if (!Number.isFinite(durationDays) || durationDays < 1) {
            alert("Thời hạn gói phải là số nguyên ≥ 1 ngày!");
            return;
        }
        // Ô bỏ trống = 0 (miễn phí) một cách tường minh, có comment thay vì
        // `Number('')` ngầm định. Chữ không phải số → báo lỗi.
        const price = priceInput.trim() === '' ? 0 : Number(priceInput);
        if (!Number.isFinite(price) || price < 0) {
            alert("Giá tiền không hợp lệ!");
            return;
        }
        try {
            // description "" = xóa mô tả (backend hiểu "" = None).
            await onSave(name, Math.trunc(durationDays), Math.trunc(price), description);
            onClose();
        } catch (err) {
            alert(`Lưu thất bại: ${err instanceof Error ? err.message : String(err)}`);
        }
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
                                value={priceInput} 
                                onChange={(e) => setPriceInput(e.target.value)} 
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
