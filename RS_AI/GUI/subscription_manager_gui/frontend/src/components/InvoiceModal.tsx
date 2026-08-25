import { useState, useEffect } from 'react';
import type { Transaction, Package } from '../../../bridge/types';
import { formatDateTime, formatCurrency } from '../utils/i18n';

interface InvoiceModalProps {
    isOpen: boolean;
    transactions: Transaction[];
    username: string;
    packages: Package[];
    onClose: () => void;
}

export function InvoiceModal({ isOpen, transactions, username, packages, onClose }: InvoiceModalProps) {
    const [banks, setBanks] = useState<{bin: string, shortName: string}[]>([]);
    
    // Form States
    const [bankBin, setBankBin] = useState('');
    const [accountNo, setAccountNo] = useState('');
    const [accountName, setAccountName] = useState('');
    const [transferContent, setTransferContent] = useState('');

    useEffect(() => {
        // Fetch bank list from VietQR API
        fetch('https://api.vietqr.io/v2/banks')
            .then(res => res.json())
            .then(data => {
                if (data.code === '00' && data.data) {
                    setBanks(data.data);
                }
            })
            .catch(err => console.error("Error fetching banks:", err));
        
        // Load settings from localStorage
        const savedBin = localStorage.getItem('vietqr_bank_bin');
        const savedAccNo = localStorage.getItem('vietqr_account_no');
        const savedAccName = localStorage.getItem('vietqr_account_name');
        
        if (savedBin) setBankBin(savedBin);
        if (savedAccNo) setAccountNo(savedAccNo);
        if (savedAccName) setAccountName(savedAccName);
    }, []);

    // Set default transfer content when transaction changes
    useEffect(() => {
        if (transactions.length > 0) {
            if (transactions.length === 1) {
                setTransferContent(`Thanh toan don hang ${transactions[0].id}`);
            } else {
                setTransferContent(`Thanh toan gop ${transactions.length} don hang`);
            }
        }
    }, [transactions]);

    const handleSaveSettings = () => {
        localStorage.setItem('vietqr_bank_bin', bankBin);
        localStorage.setItem('vietqr_account_no', accountNo);
        localStorage.setItem('vietqr_account_name', accountName);
    };

    if (!isOpen || transactions.length === 0) return null;

    const totalAmount = transactions.reduce((sum, tx) => sum + tx.amount, 0);

    // Generate VietQR Link
    const qrUrl = bankBin && accountNo 
        ? `https://img.vietqr.io/image/${bankBin}-${accountNo}-compact2.png?amount=${totalAmount}&addInfo=${encodeURIComponent(transferContent)}&accountName=${encodeURIComponent(accountName)}`
        : '';

    const handlePrint = () => {
        handleSaveSettings();
        window.print();
    };

    return (
        <div className="modal-overlay">
            <div className="modal-content" style={{ maxWidth: '850px', display: 'flex', gap: '2rem' }}>
                
                {/* Form Nhập Thông Tin (Ẩn khi In) */}
                <div className="no-print" style={{ flex: 1 }}>
                    <h2>Cấu hình Hóa Đơn & Mã QR</h2>
                    
                    <div style={{ marginBottom: '1rem' }}>
                        <label className="form-label">Ngân Hàng (Bank):</label>
                        <select 
                            className="input-field"
                            value={bankBin}
                            onChange={(e) => setBankBin(e.target.value)}
                        >
                            <option value="">-- Chọn ngân hàng --</option>
                            {banks.map(b => (
                                <option key={b.bin} value={b.bin}>{b.shortName} ({b.bin})</option>
                            ))}
                        </select>
                    </div>

                    <div style={{ marginBottom: '1rem' }}>
                        <label className="form-label">Số Tài Khoản:</label>
                        <input 
                            type="text" 
                            className="input-field" 
                            value={accountNo}
                            onChange={(e) => setAccountNo(e.target.value)}
                            placeholder="Nhập số tài khoản..."
                        />
                    </div>

                    <div style={{ marginBottom: '1rem' }}>
                        <label className="form-label">Tên Chủ Tài Khoản:</label>
                        <input 
                            type="text" 
                            className="input-field" 
                            value={accountName}
                            onChange={(e) => setAccountName(e.target.value)}
                            placeholder="Nhập tên chủ tài khoản (Không dấu)..."
                        />
                    </div>

                    <div style={{ marginBottom: '1.5rem' }}>
                        <label className="form-label">Nội Dung Chuyển Khoản:</label>
                        <input 
                            type="text" 
                            className="input-field" 
                            value={transferContent}
                            onChange={(e) => setTransferContent(e.target.value)}
                        />
                    </div>

                    <div className="modal-actions" style={{ justifyContent: 'flex-start' }}>
                        <button className="btn btn-primary" onClick={handlePrint} disabled={!bankBin || !accountNo}>
                            🖨️ In Hóa Đơn
                        </button>
                        <button className="btn" onClick={onClose}>
                            Đóng
                        </button>
                    </div>
                    <p style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', marginTop: '1rem' }}>
                        * Cấu hình Ngân hàng và STK sẽ tự động được lưu vào trình duyệt của bạn cho lần in sau.
                    </p>
                </div>

                {/* Khu Vực Hóa Đơn (Phần được in) */}
                <div className="print-area" style={{ 
                    flex: 1.2, 
                    background: 'white', 
                    color: 'black',
                    padding: '2rem',
                    borderRadius: '8px',
                    boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
                    maxHeight: '80vh',
                    overflowY: 'auto'
                }}>
                    <div style={{ textAlign: 'center', marginBottom: '2rem' }}>
                        <h1 style={{ color: 'black', margin: 0, fontSize: '1.8rem' }}>HÓA ĐƠN THANH TOÁN</h1>
                        {transactions.length === 1 ? (
                            <p style={{ margin: '0.5rem 0', color: '#666' }}>Mã GD: {transactions[0].id}</p>
                        ) : (
                            <p style={{ margin: '0.5rem 0', color: '#666' }}>Hóa đơn gộp ({transactions.length} giao dịch)</p>
                        )}
                        <p style={{ margin: '0.5rem 0', color: '#666' }}>Ngày: {formatDateTime(Date.now())}</p>
                    </div>

                    <div style={{ marginBottom: '2rem' }}>
                        <table style={{ width: '100%', borderCollapse: 'collapse', color: 'black' }}>
                            <tbody>
                                <tr style={{ borderBottom: '1px solid #eee' }}>
                                    <td style={{ padding: '0.8rem 0', fontWeight: 'bold' }}>Khách hàng:</td>
                                    <td style={{ padding: '0.8rem 0', textAlign: 'right' }}>{username}</td>
                                </tr>
                            </tbody>
                        </table>
                        
                        <div style={{ marginTop: '1rem' }}>
                            <p style={{ fontWeight: 'bold', marginBottom: '0.5rem' }}>Chi tiết dịch vụ:</p>
                            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.9rem' }}>
                                <thead>
                                    <tr style={{ borderBottom: '2px solid #ccc' }}>
                                        <th style={{ textAlign: 'left', padding: '0.5rem 0' }}>STT</th>
                                        <th style={{ textAlign: 'left', padding: '0.5rem 0' }}>Gói Dịch Vụ</th>
                                        <th style={{ textAlign: 'right', padding: '0.5rem 0' }}>Thành tiền</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {transactions.map((tx, index) => {
                                        const pkg = packages.find(p => p.id === tx.package_id);
                                        const pkgName = pkg ? pkg.name : tx.package_id;
                                        return (
                                            <tr key={tx.id} style={{ borderBottom: '1px solid #eee' }}>
                                                <td style={{ padding: '0.5rem 0' }}>{index + 1}</td>
                                                <td style={{ padding: '0.5rem 0' }}>{pkgName}</td>
                                                <td style={{ padding: '0.5rem 0', textAlign: 'right' }}>{formatCurrency(tx.amount)}</td>
                                            </tr>
                                        );
                                    })}
                                </tbody>
                            </table>
                        </div>

                        <table style={{ width: '100%', borderCollapse: 'collapse', color: 'black', marginTop: '1rem' }}>
                            <tbody>
                                <tr>
                                    <td style={{ padding: '0.8rem 0', fontWeight: 'bold', fontSize: '1.2rem' }}>Tổng tiền:</td>
                                    <td style={{ padding: '0.8rem 0', textAlign: 'right', fontSize: '1.2rem', fontWeight: 'bold' }}>
                                        {totalAmount.toLocaleString('vi-VN')} VNĐ
                                    </td>
                                </tr>
                            </tbody>
                        </table>
                    </div>

                    <div style={{ textAlign: 'center' }}>
                        <p style={{ fontWeight: 'bold', marginBottom: '1rem' }}>Quét mã để thanh toán</p>
                        {qrUrl ? (
                            <img src={qrUrl} alt="VietQR" style={{ width: '250px', height: '250px', objectFit: 'contain' }} />
                        ) : (
                            <div style={{ width: '250px', height: '250px', border: '1px dashed #ccc', margin: '0 auto', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                                <span style={{ color: '#999' }}>Vui lòng chọn Ngân hàng & STK</span>
                            </div>
                        )}
                    </div>
                </div>

            </div>
        </div>
    );
}
