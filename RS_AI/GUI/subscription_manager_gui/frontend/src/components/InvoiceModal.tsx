import { useState, useEffect } from 'react';
import type { Transaction, Package, PaymentRef } from '../../../bridge/types';
import { formatDateTime, formatCurrency } from '../utils/i18n';
import { issuePaymentRef } from '../../../bridge/payment_bridge';

interface InvoiceModalProps {
    isOpen: boolean;
    transactions: Transaction[];
    username: string;
    /// ID khách — cần để phát hành mã thanh toán gắn với người chuyển.
    userId: string;
    packages: Package[];
    onClose: () => void;
}

export function InvoiceModal({ isOpen, transactions, username, userId, packages, onClose }: InvoiceModalProps) {
    const [banks, setBanks] = useState<{bin: string, shortName: string}[]>([]);
    
    // Form States
    const [bankBin, setBankBin] = useState('');
    const [accountNo, setAccountNo] = useState('');
    const [accountName, setAccountName] = useState('');
    const [transferContent, setTransferContent] = useState('');
    // Mã thanh toán đã phát hành cho lần in này (null = chưa phát hành xong).
    const [paymentRef, setPaymentRef] = useState<PaymentRef | null>(null);
    const [refError, setRefError] = useState<string | null>(null);

    // Chỉ fetch khi modal mở — trước đây fetch + đọc localStorage ngay khi
    // mount dù isOpen=false (tốn request VietQR mỗi lần vào trang).
    // AbortController tránh setState sau khi đóng modal.
    useEffect(() => {
        if (!isOpen) return;
        const ctrl = new AbortController();

        // Fetch bank list from VietQR API
        fetch('https://api.vietqr.io/v2/banks', { signal: ctrl.signal })
            .then(res => res.json())
            .then(data => {
                if (data.code === '00' && data.data) {
                    setBanks(data.data);
                }
            })
            .catch(err => {
                if (err?.name !== 'AbortError') console.error("Error fetching banks:", err);
            });

        // Load settings from localStorage
        const savedBin = localStorage.getItem('vietqr_bank_bin');
        const savedAccNo = localStorage.getItem('vietqr_account_no');
        const savedAccName = localStorage.getItem('vietqr_account_name');

        if (savedBin) setBankBin(savedBin);
        if (savedAccNo) setAccountNo(savedAccNo);
        if (savedAccName) setAccountName(savedAccName);

        return () => ctrl.abort();
    }, [isOpen]);

    // Phát hành MÃ TRA CỨU và dùng làm nội dung chuyển khoản.
    //
    // Trước đây nội dung là "Thanh toan don hang tx_1757...": dài quá giới hạn
    // của nhiều ngân hàng, và khi khách gõ thiếu thì không tra được ai chuyển.
    // Mã mới ngắn, chỉ dùng ký tự không gây nhầm lẫn, và mang sẵn dấu hiệu nhận
    // dạng người chuyển. Mỗi lần mở hóa đơn (đơn lẻ hoặc gộp) đều phát hành và
    // LƯU LẠI ở backend để đối soát về sau.
    useEffect(() => {
        if (!isOpen || transactions.length === 0 || !userId) return;
        let cancelled = false;
        setPaymentRef(null);
        setRefError(null);

        (async () => {
            try {
                const ref = await issuePaymentRef(userId, transactions.map(t => t.id));
                if (cancelled) return;
                setPaymentRef(ref);
                setTransferContent(ref.code);
            } catch (err) {
                if (cancelled) return;
                // Không im lặng: hóa đơn không có mã thì không truy vết được.
                const msg = err instanceof Error ? err.message : String(err);
                console.error('Không phát hành được mã thanh toán:', msg);
                setRefError(msg);
                setTransferContent('');
            }
        })();

        return () => { cancelled = true; };
    }, [isOpen, userId, transactions]);

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
                        <label className="form-label">Nội Dung Chuyển Khoản (Mã tra cứu):</label>
                        <input 
                            type="text" 
                            className="input-field" 
                            value={transferContent}
                            onChange={(e) => setTransferContent(e.target.value)}
                        />
                        {paymentRef && (
                            <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '4px' }}>
                                Mã đã lưu để đối soát. <strong>{paymentRef.user_token}</strong> là dấu hiệu
                                nhận dạng của khách này — vẫn truy được người chuyển nếu mã bị gõ sai.
                            </small>
                        )}
                        {refError && (
                            <small style={{ color: 'var(--danger)', display: 'block', marginTop: '4px' }}>
                                Không phát hành được mã tra cứu: {refError}
                            </small>
                        )}
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
                        {paymentRef && (
                            <p style={{ margin: '0.5rem 0', color: '#000', fontWeight: 'bold' }}>
                                Mã tra cứu: <span style={{ fontFamily: 'monospace', letterSpacing: '1px' }}>{paymentRef.code}</span>
                            </p>
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
                        <p style={{ fontWeight: 'bold', marginBottom: '0.5rem' }}>Quét mã để thanh toán</p>
                        {/* In mã tra cứu lên hóa đơn: khách chuyển tay (không quét QR)
                            vẫn có chuỗi để gõ, và ta vẫn đối soát được. */}
                        {transferContent && (
                            <p style={{ margin: '0 0 1rem', fontSize: '0.9rem' }}>
                                Nội dung chuyển khoản:{' '}
                                <strong style={{ fontFamily: 'monospace', letterSpacing: '1px' }}>
                                    {transferContent}
                                </strong>
                            </p>
                        )}
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
