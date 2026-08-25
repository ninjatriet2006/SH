/*
[INTEGRITY NOTES]
- Mục đích: Form popup gán gói dịch vụ cho một User và cho phép thiết lập tùy chỉnh ngày hết hạn.
- Trách nhiệm: Hiển thị danh sách các Package để chọn, nhập ngày giờ nếu cần, và emit sự kiện lưu.
- Tương tác: Dùng trong trang chi tiết User hoặc quản lý Subscription. Lấy danh sách Package từ PackageStore.
*/

import React, { useState, useEffect } from 'react';
import { X } from 'lucide-react';
import type { Subscription } from '../../../bridge/types';
import { usePackageStore } from '../store/usePackageStore';
import { useTranslation } from '../utils/i18n';

interface SubscriptionModalProps {
    isOpen: boolean;
    selectedUserId: string;
    subscriptionData: Subscription | null;
    onClose: () => void;
    onSave: (packageId: string, customExpiry?: number, amount?: number) => void;
}

export function SubscriptionModal({ isOpen, subscriptionData, onClose, onSave }: SubscriptionModalProps) {
    const { t } = useTranslation();
    const { packages, fetchPackages } = usePackageStore();
    const [selectedPackage, setSelectedPackage] = useState('');
    const [customDate, setCustomDate] = useState(''); // Lưu định dạng YYYY-MM-DD
    const [dateParts, setDateParts] = useState({ dd: '', mm: '', yyyy: '' }); // Hiển thị 3 ô tách biệt
    const [displayTime, setDisplayTime] = useState(''); // Hiển thị thời gian HH:mm
    const [timeParts, setTimeParts] = useState({ hh: '', mm: '' }); // Hiển thị 2 ô tách biệt
    const [amount, setAmount] = useState<number>(0); // Số tiền thu thực tế

    // Load danh sách packages khi mở form nếu chưa có
    useEffect(() => {
        if (isOpen && packages.length === 0) {
            fetchPackages();
        }
    }, [isOpen, packages.length, fetchPackages]);

    // Đặt lại form mỗi khi mở
    useEffect(() => {
        if (subscriptionData) {
            setSelectedPackage(subscriptionData.package_id);
            // Lấy ra YYYY, MM, DD, HH, min
            const dateObj = new Date(subscriptionData.expiration_date);
            const yyyy = String(dateObj.getFullYear());
            const mm = String(dateObj.getMonth() + 1).padStart(2, '0');
            const dd = String(dateObj.getDate()).padStart(2, '0');
            const hh = String(dateObj.getHours()).padStart(2, '0');
            const min = String(dateObj.getMinutes()).padStart(2, '0');
            
            setCustomDate(`${yyyy}-${mm}-${dd}`);
            setDateParts({ dd, mm, yyyy });
            setDisplayTime(`${hh}:${min}`);
            setTimeParts({ hh, mm: min });
            setAmount(0); // Mặc định chỉnh sửa thì không tính tiền
        } else {
            setSelectedPackage('');
            setCustomDate('');
            setDateParts({ dd: '', mm: '', yyyy: '' });
            setDisplayTime('');
            setTimeParts({ hh: '', mm: '' });
            setAmount(0);
        }
    }, [subscriptionData, isOpen]);

    // Lắng nghe thay đổi gói để tự điền giá khi tạo mới
    useEffect(() => {
        if (!subscriptionData && selectedPackage) {
            const pkg = packages.find(p => p.id === selectedPackage);
            if (pkg) {
                setAmount(pkg.price || 0);
            }
        }
    }, [selectedPackage, subscriptionData, packages]);

    const handleNativeDateChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const val = e.target.value; // YYYY-MM-DD
        setCustomDate(val);
        if (val) {
            const [y, m, d] = val.split('-');
            setDateParts({ yyyy: y, mm: m, dd: d });
        } else {
            setDateParts({ dd: '', mm: '', yyyy: '' });
        }
        // Ép mất focus sau khi chọn ngày để tự động tắt bảng popup lịch native trên WebKitGTK!
        e.target.blur();
    };

    const handlePartChange = (part: 'dd'|'mm'|'yyyy', value: string) => {
        const num = value.replace(/[^0-9]/g, '');
        let newParts = { ...dateParts };
        if (part === 'dd') {
            let val = num.slice(0, 2);
            if (parseInt(val, 10) > 31) val = '31';
            newParts.dd = val;
        }
        if (part === 'mm') {
            let val = num.slice(0, 2);
            if (parseInt(val, 10) > 12) val = '12';
            newParts.mm = val;
        }
        if (part === 'yyyy') {
            newParts.yyyy = num.slice(0, 4);
        }
        
        setDateParts(newParts);
        
        if (!newParts.yyyy && !newParts.mm && !newParts.dd) {
            setCustomDate('');
        } else {
            const today = new Date();
            const y = newParts.yyyy.length === 4 ? newParts.yyyy : String(today.getFullYear());
            const m = newParts.mm ? newParts.mm.padStart(2, '0') : '01';
            const d = newParts.dd ? newParts.dd.padStart(2, '0') : '01';
            setCustomDate(`${y}-${m}-${d}`);
        }
    };

    const handlePartBlur = (part: 'dd'|'mm'|'yyyy') => {
        let newParts = { ...dateParts };
        if (part === 'dd' && newParts.dd) {
            if (parseInt(newParts.dd, 10) === 0) newParts.dd = '01';
            else newParts.dd = newParts.dd.padStart(2, '0');
        }
        if (part === 'mm' && newParts.mm) {
            if (parseInt(newParts.mm, 10) === 0) newParts.mm = '01';
            else newParts.mm = newParts.mm.padStart(2, '0');
        }
        if (part === 'yyyy' && newParts.yyyy) {
            if (newParts.yyyy.length < 4) newParts.yyyy = String(new Date().getFullYear());
        }
        setDateParts(newParts);
        
        if (newParts.yyyy && newParts.mm && newParts.dd) {
            setCustomDate(`${newParts.yyyy}-${newParts.mm}-${newParts.dd}`);
        }
    };

    const handleTimePartChange = (part: 'hh'|'mm', value: string) => {
        const num = value.replace(/[^0-9]/g, '');
        let newParts = { ...timeParts };
        
        if (part === 'hh') {
            let val = num.slice(0, 2);
            if (parseInt(val, 10) > 23) val = '23';
            newParts.hh = val;
        }
        if (part === 'mm') {
            let val = num.slice(0, 2);
            if (parseInt(val, 10) > 59) val = '59';
            newParts.mm = val;
        }
        
        setTimeParts(newParts);
        
        if (!newParts.hh && !newParts.mm) {
            setDisplayTime('');
        } else {
            const h = newParts.hh ? newParts.hh.padStart(2, '0') : '00';
            const m = newParts.mm ? newParts.mm.padStart(2, '0') : '00';
            setDisplayTime(`${h}:${m}`);
        }
    };

    const handleTimePartBlur = (part: 'hh'|'mm') => {
        let newParts = { ...timeParts };
        if (part === 'hh' && newParts.hh) {
            newParts.hh = newParts.hh.padStart(2, '0');
        }
        if (part === 'mm' && newParts.mm) {
            newParts.mm = newParts.mm.padStart(2, '0');
        }
        setTimeParts(newParts);
        
        if (newParts.hh || newParts.mm) {
            const h = newParts.hh ? newParts.hh.padStart(2, '0') : '00';
            const m = newParts.mm ? newParts.mm.padStart(2, '0') : '00';
            setDisplayTime(`${h}:${m}`);
        }
    };

    // Gia hạn nhanh (Dựa trên ngày hết hạn HIỆN TẠI của gói)
    const handleQuickExtend = (months: number) => {
        if (!subscriptionData) return;
        
        const currentExp = new Date(subscriptionData.expiration_date);
        currentExp.setMonth(currentExp.getMonth() + months);
        
        const yyyy = String(currentExp.getFullYear());
        const mm = String(currentExp.getMonth() + 1).padStart(2, '0');
        const dd = String(currentExp.getDate()).padStart(2, '0');
        const hh = String(currentExp.getHours()).padStart(2, '0');
        const min = String(currentExp.getMinutes()).padStart(2, '0');
        
        setCustomDate(`${yyyy}-${mm}-${dd}`);
        setDateParts({ dd, mm, yyyy });
        setDisplayTime(`${hh}:${min}`);
        setTimeParts({ hh, mm: min });
        
        // Tự động điền giá: Lấy giá gốc nhân với số tháng gia hạn
        const pkg = packages.find(p => p.id === subscriptionData.package_id);
        if (pkg) {
            setAmount((pkg.price || 0) * months);
        }
    };

    if (!isOpen) return null;

    const handleSave = (e: React.FormEvent) => {
        e.preventDefault();
        if (!selectedPackage) {
            alert("Vui lòng chọn một gói dịch vụ!");
            return;
        }
        
        let expiryTimestamp: number | undefined = undefined;
        if (customDate) {
            // Lấy time hoặc mặc định
            const [hr, mn] = displayTime ? displayTime.split(':') : ['00', '00'];
            
            // Đổi chuỗi ngày về timestamp (mili-giây)
            expiryTimestamp = new Date(`${customDate}T${hr.padStart(2, '0')}:${mn.padStart(2, '0')}:00`).getTime();
        }
        
        onSave(selectedPackage, expiryTimestamp, amount);
        onClose();
    };

    return (
        <div className="modal-overlay">
            <div className="modal-content animate-fade-in" style={{ maxWidth: '450px' }}>
                <button onClick={onClose} style={{ position: 'absolute', top: '1rem', right: '1rem', background: 'transparent', border: 'none', color: 'white', cursor: 'pointer' }}>
                    <X size={20} />
                </button>
                
                
                <h3>{subscriptionData ? t('sub_modal.edit_title') : t('sub_modal.add_title')}</h3>
                
                <form onSubmit={handleSave}>
                    {subscriptionData ? (
                        <div className="form-group">
                            <label className="form-label">{t('sub_modal.lbl_current_pkg')}</label>
                            <input 
                                type="text"
                                className="input-field"
                                value={packages.find(p => p.id === selectedPackage)?.name || selectedPackage}
                                disabled
                                style={{ opacity: 0.7 }}
                            />
                        </div>
                    ) : (
                        <div className="form-group">
                            <label className="form-label">{t('sub_modal.lbl_pkg')}</label>
                            <select 
                                className="input-field" 
                                value={selectedPackage} 
                                onChange={(e) => setSelectedPackage(e.target.value)}
                                required
                            >
                                <option value="">-- Chọn một gói dịch vụ --</option>
                                {packages.map(pkg => (
                                    <option key={pkg.id} value={pkg.id}>{pkg.name} ({pkg.duration_days} ngày)</option>
                                ))}
                            </select>
                        </div>
                    )}
                    
                    <div className="form-group">
                        <label className="form-label">
                            {t('sub_modal.lbl_expiry')}
                        </label>
                        <div className="input-field" style={{ position: 'relative', display: 'flex', alignItems: 'center', padding: '0 0.5rem', width: '100%', gap: '4px' }}>
                            {/* Input Ngày */}
                            <input 
                                type="text" 
                                placeholder="DD" 
                                value={dateParts.dd}
                                onChange={(e) => handlePartChange('dd', e.target.value)}
                                onBlur={() => handlePartBlur('dd')}
                                onDoubleClick={(e) => e.currentTarget.select()}
                                style={{ width: '2rem', border: 'none', background: 'transparent', textAlign: 'center', color: 'inherit', fontFamily: 'inherit', fontSize: 'inherit', outline: 'none', padding: 0 }}
                            />
                            <span style={{ color: 'var(--text-secondary)' }}>/</span>
                            {/* Input Tháng */}
                            <input 
                                type="text" 
                                placeholder="MM" 
                                value={dateParts.mm}
                                onChange={(e) => handlePartChange('mm', e.target.value)}
                                onBlur={() => handlePartBlur('mm')}
                                onDoubleClick={(e) => e.currentTarget.select()}
                                style={{ width: '2rem', border: 'none', background: 'transparent', textAlign: 'center', color: 'inherit', fontFamily: 'inherit', fontSize: 'inherit', outline: 'none', padding: 0 }}
                            />
                            <span style={{ color: 'var(--text-secondary)' }}>/</span>
                            {/* Input Năm */}
                            <input 
                                type="text" 
                                placeholder="YYYY" 
                                value={dateParts.yyyy}
                                onChange={(e) => handlePartChange('yyyy', e.target.value)}
                                onBlur={() => handlePartBlur('yyyy')}
                                onDoubleClick={(e) => e.currentTarget.select()}
                                style={{ flex: 1, border: 'none', background: 'transparent', textAlign: 'left', color: 'inherit', fontFamily: 'inherit', fontSize: 'inherit', outline: 'none', padding: 0 }}
                            />
                            
                            {/* Icon Cây lịch */}
                            <div style={{ position: 'absolute', right: '10px', display: 'flex', alignItems: 'center' }}>
                                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ color: 'var(--text-secondary)' }}><rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect><line x1="16" y1="2" x2="16" y2="6"></line><line x1="8" y1="2" x2="8" y2="6"></line><line x1="3" y1="10" x2="21" y2="10"></line></svg>
                                
                                {/* Input date Native ẩn đè lên icon để gọi popup lịch */}
                                <input 
                                    type="date" 
                                    value={customDate} 
                                    onChange={handleNativeDateChange} 
                                    tabIndex={-1}
                                    style={{ position: 'absolute', right: 0, width: '24px', height: '24px', opacity: 0, cursor: 'pointer' }}
                                    title="Mở lịch chọn ngày"
                                />
                            </div>
                        </div>
                        <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '4px', marginBottom: '1rem' }}>
                            {subscriptionData 
                                ? 'Chọn ngày hết hạn.' 
                                : 'Chọn ngày. Nếu bỏ trống, tự cộng số ngày mặc định.'}
                        </small>

                        <label className="form-label">
                            {t('sub_modal.lbl_time')}
                        </label>
                        <div className="input-field" style={{ display: 'flex', alignItems: 'center', padding: '0 0.5rem', width: '100%', gap: '4px' }}>
                            <input 
                                type="text" 
                                placeholder="HH" 
                                value={timeParts.hh}
                                onChange={(e) => handleTimePartChange('hh', e.target.value)}
                                onBlur={() => handleTimePartBlur('hh')}
                                onDoubleClick={(e) => e.currentTarget.select()}
                                style={{ width: '2.5rem', border: 'none', background: 'transparent', textAlign: 'center', color: 'inherit', fontFamily: 'inherit', fontSize: 'inherit', outline: 'none', padding: 0 }}
                            />
                            <span style={{ color: 'var(--text-secondary)' }}>:</span>
                            <input 
                                type="text" 
                                placeholder="MM" 
                                value={timeParts.mm}
                                onChange={(e) => handleTimePartChange('mm', e.target.value)}
                                onBlur={() => handleTimePartBlur('mm')}
                                onDoubleClick={(e) => e.currentTarget.select()}
                                style={{ flex: 1, border: 'none', background: 'transparent', textAlign: 'left', color: 'inherit', fontFamily: 'inherit', fontSize: 'inherit', outline: 'none', padding: 0 }}
                            />
                        </div>
                        <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '4px' }}>
                            Tùy chọn. Nếu bỏ trống, hệ thống sẽ mặc định lấy 00:00.
                        </small>

                        <div style={{ marginTop: '1rem' }}>
                            <label className="form-label">
                                {t('sub_modal.lbl_amount')}
                            </label>
                            <input 
                                type="number" 
                                className="input-field" 
                                value={amount === 0 ? '' : amount} 
                                onChange={(e) => setAmount(Number(e.target.value))} 
                                placeholder="Nhập số tiền (VD: 500000)"
                                min="0"
                            />
                            <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '4px' }}>
                                {subscriptionData ? 'Để trống hoặc 0 nếu không thu thêm tiền.' : 'Mặc định là giá gốc của gói.'}
                            </small>
                        </div>

                        {/* Nút bấm gia hạn nhanh */}
                        {subscriptionData && (
                            <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.75rem', flexWrap: 'wrap' }}>
                                <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', alignSelf: 'center', marginRight: '4px' }}>{t('sub_modal.quick_renew')}:</span>
                                <button type="button" className="btn" style={{ padding: '0.2rem 0.5rem', fontSize: '0.75rem', background: 'rgba(255,255,255,0.1)' }} onClick={() => handleQuickExtend(1)}>{t('sub_modal.renew_1m')}</button>
                                <button type="button" className="btn" style={{ padding: '0.2rem 0.5rem', fontSize: '0.75rem', background: 'rgba(255,255,255,0.1)' }} onClick={() => handleQuickExtend(3)}>{t('sub_modal.renew_3m')}</button>
                                <button type="button" className="btn" style={{ padding: '0.2rem 0.5rem', fontSize: '0.75rem', background: 'rgba(255,255,255,0.1)' }} onClick={() => handleQuickExtend(6)}>{t('sub_modal.renew_6m')}</button>
                                <button type="button" className="btn" style={{ padding: '0.2rem 0.5rem', fontSize: '0.75rem', background: 'rgba(255,255,255,0.1)' }} onClick={() => handleQuickExtend(12)}>{t('sub_modal.renew_1y')}</button>
                            </div>
                        )}
                    </div>

                    <div className="modal-actions">
                        <button type="button" className="btn btn-danger" onClick={onClose}>{t('common.cancel')}</button>
                        <button type="submit" className="btn btn-primary">{t('common.confirm')}</button>
                    </div>
                </form>
            </div>
        </div>
    );
}
