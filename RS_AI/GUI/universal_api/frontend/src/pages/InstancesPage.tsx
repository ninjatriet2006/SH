import { useEffect, useState } from 'react';
import {
    Play,
    Square,
    Copy,
    Trash2,
    Plus,
    RefreshCw,
    Folder,
    Shield,
    Cpu,
    AlertCircle,
    CheckCircle2,
    Radio,
} from 'lucide-react';
import { useProfileStore } from '../store/useProfileStore';
import { listAccounts } from '../../../bridge/accounts_bridge';
import type { AccountInfo } from '../../../bridge/types';

export function InstancesPage() {
    const {
        profiles,
        runningInstances,
        fetchProfiles,
        createProfile,
        deleteProfile,
        cloneProfile,
        bindAccount,
        launchInstance,
        stopInstance,
    } = useProfileStore();

    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [modalOpen, setModalOpen] = useState(false);
    const [cloneModalOpen, setCloneModalOpen] = useState(false);
    const [targetProfileId, setTargetProfileId] = useState('');
    const [newProfileName, setNewProfileName] = useState('');
    const [selectedPlatform, setSelectedPlatform] = useState('codebuddy_cn');
    const [selectedAccount, setSelectedAccount] = useState('');
    const [extraArgs, setExtraArgs] = useState('');
    const [actionMsg, setActionMsg] = useState<{ text: string; ok: boolean } | null>(null);

    useEffect(() => {
        fetchProfiles();
        listAccounts().then(setAccounts).catch(() => {});
    }, []);

    const showMsg = (text: string, ok: boolean) => {
        setActionMsg({ text, ok });
        setTimeout(() => setActionMsg(null), 4000);
    };

    const handleCreate = async () => {
        if (!newProfileName.trim()) {
            showMsg('Vui lòng nhập tên profile', false);
            return;
        }
        try {
            const argsList = extraArgs.split(' ').map((s) => s.trim()).filter(Boolean);
            await createProfile(newProfileName, selectedPlatform, selectedAccount || undefined, argsList);
            setModalOpen(false);
            setNewProfileName('');
            setExtraArgs('');
            showMsg('Đã tạo profile mô phỏng môi trường mới thành công!', true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi tạo profile', false);
        }
    };

    const handleClone = async () => {
        if (!newProfileName.trim() || !targetProfileId) return;
        try {
            await cloneProfile(targetProfileId, newProfileName);
            setCloneModalOpen(false);
            setNewProfileName('');
            showMsg('Đã nhân bản profile với mã máy ảo mới thành công!', true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi nhân bản', false);
        }
    };

    const handleLaunch = async (profileId: string) => {
        try {
            const pid = await launchInstance(profileId);
            showMsg(`Đã khởi chạy tiến trình IDE thành công (PID: ${pid})`, true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi khởi chạy', false);
        }
    };

    const handleStop = async (profileId: string) => {
        try {
            await stopInstance(profileId);
            showMsg('Đã dừng tiến trình IDE thành công', true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi dừng', false);
        }
    };

    const handleBind = async (profileId: string, accountUid: string) => {
        try {
            await bindAccount(profileId, accountUid);
            showMsg('Đã tiêm tài khoản vào SQLite state.vscdb thành công!', true);
        } catch (e: any) {
            showMsg(e.message || 'Lỗi khi tiêm tài khoản', false);
        }
    };

    return (
        <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
            {/* Header */}
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
                <div>
                    <h1 style={{ fontSize: '1.75rem', fontWeight: 700, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                        <Cpu color="var(--primary)" size={28} />
                        Instance Profiles & Giả lập Môi trường
                    </h1>
                    <p style={{ color: 'var(--text-secondary)', marginTop: '0.25rem', fontSize: '0.9rem' }}>
                        Quản lý các phiên IDE chạy song song với thư mục cô lập (<code style={{ color: 'var(--primary)' }}>--user-data-dir</code>), ID phần cứng giả lập và tiêm session tự động.
                    </p>
                </div>
                <div style={{ display: 'flex', gap: '0.75rem' }}>
                    <button className="btn" onClick={() => fetchProfiles()} title="Làm mới">
                        <RefreshCw size={16} />
                    </button>
                    <button className="btn btn-primary" onClick={() => setModalOpen(true)} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <Plus size={16} /> Tạo Profile Mới
                    </button>
                </div>
            </div>

            {/* Notification alert */}
            {actionMsg && (
                <div style={{
                    padding: '0.75rem 1rem',
                    borderRadius: '0.5rem',
                    marginBottom: '1rem',
                    background: actionMsg.ok ? 'rgba(16, 185, 129, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                    border: `1px solid ${actionMsg.ok ? 'var(--success)' : 'var(--danger)'}`,
                    color: actionMsg.ok ? 'var(--success)' : 'var(--danger)',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.5rem'
                }}>
                    {actionMsg.ok ? <CheckCircle2 size={18} /> : <AlertCircle size={18} />}
                    {actionMsg.text}
                </div>
            )}

            {/* Stats summary bar */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: '1rem', marginBottom: '1.5rem' }}>
                <div className="card" style={{ padding: '1rem', display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <div style={{ background: 'rgba(99, 102, 241, 0.15)', padding: '0.75rem', borderRadius: '0.5rem', color: 'var(--primary)' }}>
                        <Folder size={24} />
                    </div>
                    <div>
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Tổng số Profile</div>
                        <div style={{ fontSize: '1.5rem', fontWeight: 700 }}>{profiles.length}</div>
                    </div>
                </div>

                <div className="card" style={{ padding: '1rem', display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <div style={{ background: 'rgba(16, 185, 129, 0.15)', padding: '0.75rem', borderRadius: '0.5rem', color: 'var(--success)' }}>
                        <Radio size={24} />
                    </div>
                    <div>
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Đang chạy song song</div>
                        <div style={{ fontSize: '1.5rem', fontWeight: 700 }}>{Object.keys(runningInstances).length}</div>
                    </div>
                </div>

                <div className="card" style={{ padding: '1rem', display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <div style={{ background: 'rgba(245, 158, 11, 0.15)', padding: '0.75rem', borderRadius: '0.5rem', color: 'var(--warning)' }}>
                        <Shield size={24} />
                    </div>
                    <div>
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Hardware Spoofing</div>
                        <div style={{ fontSize: '1.5rem', fontWeight: 700, color: 'var(--success)' }}>Kích hoạt</div>
                    </div>
                </div>
            </div>

            {/* Profiles list */}
            {profiles.length === 0 ? (
                <div className="card" style={{ textAlign: 'center', padding: '3rem 1rem' }}>
                    <Cpu size={48} style={{ opacity: 0.3, marginBottom: '1rem' }} />
                    <p style={{ color: 'var(--text-secondary)', marginBottom: '1rem' }}>Chưa có Profile nào được tạo. Hãy tạo profile đầu tiên để chạy IDE cô lập nhiều tài khoản!</p>
                    <button className="btn btn-primary" onClick={() => setModalOpen(true)}>
                        <Plus size={16} /> Tạo Profile Ngay
                    </button>
                </div>
            ) : (
                <div style={{ display: 'grid', gridTemplateColumns: '1fr', gap: '1rem' }}>
                    {profiles.map((p) => {
                        const pid = runningInstances[p.id];
                        const isRunning = typeof pid === 'number';
                        return (
                            <div key={p.id} className="card" style={{ padding: '1.25rem', borderLeft: isRunning ? '4px solid var(--success)' : '1px solid var(--border)' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                                    <div>
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', marginBottom: '0.25rem' }}>
                                            <h3 style={{ fontSize: '1.1rem', fontWeight: 600 }}>{p.name}</h3>
                                            <span style={{
                                                fontSize: '0.75rem',
                                                padding: '0.2rem 0.6rem',
                                                borderRadius: '1rem',
                                                background: isRunning ? 'rgba(16, 185, 129, 0.2)' : 'rgba(148, 163, 184, 0.1)',
                                                color: isRunning ? 'var(--success)' : 'var(--text-secondary)',
                                                fontWeight: 600
                                            }}>
                                                {isRunning ? `Đang chạy (PID: ${pid})` : 'Đã dừng'}
                                            </span>
                                            <span style={{
                                                fontSize: '0.75rem',
                                                padding: '0.2rem 0.6rem',
                                                borderRadius: '1rem',
                                                background: 'rgba(99, 102, 241, 0.15)',
                                                color: 'var(--primary)',
                                                textTransform: 'uppercase',
                                                fontWeight: 600
                                            }}>
                                                {p.platform_id}
                                            </span>
                                        </div>
                                        <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', display: 'flex', gap: '1rem', flexWrap: 'wrap', marginTop: '0.5rem' }}>
                                            <span><strong>Thư mục:</strong> <code style={{ color: 'var(--text-primary)' }}>{p.user_data_dir}</code></span>
                                        </div>
                                        <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', display: 'flex', gap: '1rem', marginTop: '0.4rem', fontFamily: 'monospace' }}>
                                            <span>MachineID: {p.hardware_fingerprint.machine_id.slice(0, 12)}...</span>
                                            <span>DevDeviceID: {p.hardware_fingerprint.dev_device_id.slice(0, 13)}...</span>
                                        </div>
                                    </div>

                                    {/* Action buttons */}
                                    <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                                        {isRunning ? (
                                            <button className="btn btn-danger" onClick={() => handleStop(p.id)} style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
                                                <Square size={14} /> Dừng
                                            </button>
                                        ) : (
                                            <button className="btn btn-success" onClick={() => handleLaunch(p.id)} style={{ display: 'flex', alignItems: 'center', gap: '0.35rem' }}>
                                                <Play size={14} /> Khởi chạy
                                            </button>
                                        )}
                                        <button className="btn" onClick={() => { setTargetProfileId(p.id); setNewProfileName(`${p.name} (Clone)`); setCloneModalOpen(true); }} title="Nhân bản profile với ID máy mới">
                                            <Copy size={14} />
                                        </button>
                                        <button className="btn btn-danger" onClick={() => deleteProfile(p.id)} title="Xóa profile">
                                            <Trash2 size={14} />
                                        </button>
                                    </div>
                                </div>

                                {/* Bound account selector */}
                                <div style={{ marginTop: '1rem', paddingTop: '0.75rem', borderTop: '1px solid rgba(255,255,255,0.05)', display: 'flex', alignItems: 'center', gap: '1rem' }}>
                                    <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>Tài khoản gán vào profile:</span>
                                    <select
                                        style={{
                                            padding: '0.35rem 0.75rem',
                                            borderRadius: '0.4rem',
                                            background: 'rgba(255,255,255,0.06)',
                                            border: '1px solid var(--border)',
                                            color: 'var(--text-primary)',
                                            fontSize: '0.85rem'
                                        }}
                                        value={p.bound_account_id || ''}
                                        onChange={(e) => handleBind(p.id, e.target.value)}
                                    >
                                        <option value="">-- Chưa gắn tài khoản nào --</option>
                                        {accounts.map((a) => (
                                            <option key={a.uid} value={a.uid}>
                                                {a.nickname} ({a.uid}) [{a.domain}]
                                            </option>
                                        ))}
                                    </select>
                                    {p.bound_account_id && (
                                        <span style={{ fontSize: '0.75rem', color: 'var(--success)', display: 'flex', alignItems: 'center', gap: '0.25rem' }}>
                                            <CheckCircle2 size={14} /> Đã tiêm vào SQLite state.vscdb
                                        </span>
                                    )}
                                </div>
                            </div>
                        );
                    })}
                </div>
            )}

            {/* Modal Tạo Profile Mới */}
            {modalOpen && (
                <div style={{
                    position: 'fixed',
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    background: 'rgba(0,0,0,0.7)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    zIndex: 1000
                }}>
                    <div className="card" style={{ width: '480px', padding: '1.75rem', background: '#1e293b', border: '1px solid var(--border)' }}>
                        <h2 style={{ fontSize: '1.25rem', marginBottom: '1.25rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            <Cpu size={20} color="var(--primary)" />
                            Tạo Instance Profile Mới
                        </h2>

                        <div style={{ marginBottom: '1rem' }}>
                            <label style={{ display: 'block', fontSize: '0.85rem', marginBottom: '0.4rem', color: 'var(--text-secondary)' }}>
                                Tên gợi nhớ Profile:
                            </label>
                            <input
                                type="text"
                                style={{
                                    width: '100%',
                                    padding: '0.5rem',
                                    borderRadius: '0.4rem',
                                    background: 'rgba(255,255,255,0.06)',
                                    border: '1px solid var(--border)',
                                    color: 'var(--text-primary)'
                                }}
                                placeholder="Ví dụ: CodeBuddy CN - Dự án A"
                                value={newProfileName}
                                onChange={(e) => setNewProfileName(e.target.value)}
                            />
                        </div>

                        <div style={{ marginBottom: '1rem' }}>
                            <label style={{ display: 'block', fontSize: '0.85rem', marginBottom: '0.4rem', color: 'var(--text-secondary)' }}>
                                Nền tảng IDE mục tiêu:
                            </label>
                            <select
                                style={{
                                    width: '100%',
                                    padding: '0.5rem',
                                    borderRadius: '0.4rem',
                                    background: 'rgba(255,255,255,0.06)',
                                    border: '1px solid var(--border)',
                                    color: 'var(--text-primary)'
                                }}
                                value={selectedPlatform}
                                onChange={(e) => setSelectedPlatform(e.target.value)}
                            >
                                <option value="codebuddy_cn">CodeBuddy CN (Tencent Copilot)</option>
                                <option value="codebuddy_global">CodeBuddy Global (codebuddy.ai)</option>
                                <option value="cursor">Cursor IDE</option>
                                <option value="windsurf">Windsurf IDE</option>
                                <option value="vscode">VS Code / Copilot</option>
                                <option value="zed">Zed Editor</option>
                            </select>
                        </div>

                        <div style={{ marginBottom: '1rem' }}>
                            <label style={{ display: 'block', fontSize: '0.85rem', marginBottom: '0.4rem', color: 'var(--text-secondary)' }}>
                                Gán tài khoản ban đầu:
                            </label>
                            <select
                                style={{
                                    width: '100%',
                                    padding: '0.5rem',
                                    borderRadius: '0.4rem',
                                    background: 'rgba(255,255,255,0.06)',
                                    border: '1px solid var(--border)',
                                    color: 'var(--text-primary)'
                                }}
                                value={selectedAccount}
                                onChange={(e) => setSelectedAccount(e.target.value)}
                            >
                                <option value="">-- Để trống (chưa gán) --</option>
                                {accounts.map((a) => (
                                    <option key={a.uid} value={a.uid}>
                                        {a.nickname} ({a.uid}) [{a.domain}]
                                    </option>
                                ))}
                            </select>
                        </div>

                        <div style={{ marginBottom: '1.5rem' }}>
                            <label style={{ display: 'block', fontSize: '0.85rem', marginBottom: '0.4rem', color: 'var(--text-secondary)' }}>
                                Tham số dòng lệnh bổ sung (Tùy chọn):
                            </label>
                            <input
                                type="text"
                                style={{
                                    width: '100%',
                                    padding: '0.5rem',
                                    borderRadius: '0.4rem',
                                    background: 'rgba(255,255,255,0.06)',
                                    border: '1px solid var(--border)',
                                    color: 'var(--text-primary)'
                                }}
                                placeholder="--proxy-server=http://127.0.0.1:7890"
                                value={extraArgs}
                                onChange={(e) => setExtraArgs(e.target.value)}
                            />
                        </div>

                        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.75rem' }}>
                            <button className="btn" onClick={() => setModalOpen(false)}>Hủy</button>
                            <button className="btn btn-primary" onClick={handleCreate}>Tạo Ngay</button>
                        </div>
                    </div>
                </div>
            )}

            {/* Modal Nhân Bản Profile */}
            {cloneModalOpen && (
                <div style={{
                    position: 'fixed',
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    background: 'rgba(0,0,0,0.7)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    zIndex: 1000
                }}>
                    <div className="card" style={{ width: '420px', padding: '1.5rem', background: '#1e293b', border: '1px solid var(--border)' }}>
                        <h2 style={{ fontSize: '1.15rem', marginBottom: '1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            <Copy size={18} color="var(--primary)" />
                            Nhân Bản Profile (Hardware Spoofed)
                        </h2>
                        <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '1rem' }}>
                            Profile mới sẽ được cấp thư mục riêng và một bộ định danh phần cứng (MachineID, MacID, DevDeviceID) ngẫu nhiên mới hoàn toàn.
                        </p>
                        <div style={{ marginBottom: '1.25rem' }}>
                            <label style={{ display: 'block', fontSize: '0.85rem', marginBottom: '0.4rem' }}>Tên profile mới:</label>
                            <input
                                type="text"
                                style={{
                                    width: '100%',
                                    padding: '0.5rem',
                                    borderRadius: '0.4rem',
                                    background: 'rgba(255,255,255,0.06)',
                                    border: '1px solid var(--border)',
                                    color: 'var(--text-primary)'
                                }}
                                value={newProfileName}
                                onChange={(e) => setNewProfileName(e.target.value)}
                            />
                        </div>
                        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.75rem' }}>
                            <button className="btn" onClick={() => setCloneModalOpen(false)}>Hủy</button>
                            <button className="btn btn-primary" onClick={handleClone}>Nhân Bản</button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
