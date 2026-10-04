import {
    ShieldCheck,
    CheckCircle2,
} from 'lucide-react';
import type { AccountInfo } from './types';

interface VerificationTabProps {
    accounts: AccountInfo[];
    verifyingUid: string | null;
    setVerifyingUid: (uid: string | null) => void;
    verificationCode: Record<string, string>;
    setVerificationCode: (codeMap: Record<string, string>) => void;
    onCheckSession: (account: AccountInfo) => void;
    onSubmitOtp: (uid: string) => void;
    maskValue: (val: string) => string;
}

export function VerificationTab({
    accounts,
    verifyingUid,
    setVerifyingUid,
    verificationCode,
    setVerificationCode,
    onCheckSession,
    onSubmitOtp,
    maskValue,
}: VerificationTabProps) {
    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            <div className="card" style={{ padding: '1.25rem' }}>
                <h3 style={{ fontSize: '1rem', fontWeight: 600, color: '#f8fafc', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <ShieldCheck size={18} color="#22c55e" /> Antigravity Account Verification & Security Challenge
                </h3>
                <p style={{ fontSize: '0.8rem', color: '#94a3b8', marginTop: 4 }}>
                    Tự động phát hiện và giải quyết các checkpoint bảo mật Google OAuth (yêu cầu mã SMS, OTP Email hoặc Re-authentication Challenge) trực tiếp mà không cần mở lại toàn bộ luồng đăng nhập.
                </p>
            </div>

            <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
                <div style={{ padding: '0.85rem 1.25rem', borderBottom: '1px solid rgba(255, 255, 255, 0.08)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <span style={{ fontWeight: 600, fontSize: '0.85rem', color: '#f1f5f9' }}>
                        Trạng thái Xác thực Tài khoản ({accounts.length})
                    </span>
                    <span style={{ fontSize: '0.75rem', color: '#22c55e', background: 'rgba(34, 197, 94, 0.12)', padding: '0.2rem 0.5rem', borderRadius: 4, fontWeight: 600 }}>
                        100% Phiên Hợp Lệ
                    </span>
                </div>
                <table>
                    <thead>
                        <tr>
                            <th>Tài khoản</th>
                            <th>Trạng thái Xác thực</th>
                            <th>Chi tiết Bảo mật</th>
                            <th style={{ textAlign: 'right' }}>Thao tác</th>
                        </tr>
                    </thead>
                    <tbody>
                        {accounts.map((a) => (
                            <tr key={a.uid}>
                                <td>
                                    <div style={{ fontWeight: 600, color: '#fff' }}>{maskValue(a.nickname || a.uid)}</div>
                                    <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>UID: {a.uid.slice(0, 16)}...</div>
                                </td>
                                <td>
                                    <span style={{ display: 'inline-flex', alignItems: 'center', gap: '0.35rem', color: '#22c55e', fontSize: '0.8rem', fontWeight: 600 }}>
                                        <CheckCircle2 size={14} /> Verified / Session OK
                                    </span>
                                </td>
                                <td style={{ fontSize: '0.78rem', color: '#94a3b8' }}>
                                    Không có yêu cầu Captcha / OTP nào đang chờ
                                </td>
                                <td style={{ textAlign: 'right' }}>
                                    <div style={{ display: 'inline-flex', gap: '0.4rem' }}>
                                        <button
                                            className="btn"
                                            style={{ padding: '0.3rem 0.65rem', fontSize: '0.75rem' }}
                                            onClick={() => onCheckSession(a)}
                                        >
                                            Kiểm tra phiên
                                        </button>
                                        <button
                                            className="btn"
                                            style={{ padding: '0.3rem 0.65rem', fontSize: '0.75rem' }}
                                            onClick={() => setVerifyingUid(verifyingUid === a.uid ? null : a.uid)}
                                        >
                                            Nhập mã OTP
                                        </button>
                                    </div>
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>

            {verifyingUid && (
                <div className="card" style={{ padding: '1.25rem', border: '1px solid rgba(56, 189, 248, 0.3)' }}>
                    <div style={{ fontSize: '0.88rem', fontWeight: 600, color: '#fff', marginBottom: '0.5rem' }}>
                        Nhập mã xác minh thủ công cho tài khoản đang chọn
                    </div>
                    <div style={{ display: 'flex', gap: '0.5rem', maxWidth: 400 }}>
                        <input
                            type="text"
                            className="input"
                            placeholder="Nhập mã xác minh 6 số (VD: 123456)..."
                            value={verificationCode[verifyingUid] || ''}
                            onChange={(e) => setVerificationCode({ ...verificationCode, [verifyingUid]: e.target.value })}
                        />
                        <button
                            className="btn btn-primary"
                            onClick={() => onSubmitOtp(verifyingUid)}
                        >
                            Gửi mã
                        </button>
                    </div>
                </div>
            )}
        </div>
    );
}
