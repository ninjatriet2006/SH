import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
    Users,
    Sparkles,
    RefreshCw,
    Play,
    Square,
    Layers,
    Plus,
    Rocket,
    Zap
} from 'lucide-react';
import { listAccounts } from '../../../bridge/accounts_bridge';
import type { AccountInfo } from '../../../bridge/types';
import { useProfileStore } from '../store/useProfileStore';

// Assets
import codebuddyIcon from '../assets/icons/codebuddy.png';
import zedIcon from '../assets/icons/zed.png';
import cursorIcon from '../assets/icons/cursor-menu.png';

export function DashboardPage() {
    const navigate = useNavigate();
    const { profiles, runningInstances, fetchProfiles, launchInstance, stopInstance } = useProfileStore();
    const [accounts, setAccounts] = useState<AccountInfo[]>([]);
    const [loading, setLoading] = useState(true);
    const [refreshing, setRefreshing] = useState(false);
    const [launchingId, setLaunchingId] = useState<string | null>(null);

    const loadData = async () => {
        try {
            setRefreshing(true);
            const [accs] = await Promise.all([
                listAccounts(),
                fetchProfiles(),
            ]);
            setAccounts(accs);
        } catch (e) {
            console.error('Failed to load dashboard data:', e);
        } finally {
            setLoading(false);
            setRefreshing(false);
        }
    };

    useEffect(() => {
        loadData();
    }, []);

    const codebuddyCnAccounts = accounts.filter((a) => a.domain?.includes('tencent') || a.domain?.includes('cn'));
    const codebuddyGlobalAccounts = accounts.filter((a) => a.domain?.includes('codebuddy.ai') || a.domain?.includes('global'));
    const zedAccounts = accounts.filter((a) => a.domain?.includes('zed'));

    const runningCount = Object.keys(runningInstances).length;
    const totalProfiles = profiles.length;

    const handleQuickLaunch = async (profileId: string) => {
        try {
            setLaunchingId(profileId);
            await launchInstance(profileId);
        } catch (e) {
            console.error('Launch failed:', e);
        } finally {
            setLaunchingId(null);
        }
    };

    const handleQuickStop = async (profileId: string) => {
        try {
            await stopInstance(profileId);
        } catch (e) {
            console.error('Stop failed:', e);
        }
    };

    return (
        <div style={{ maxWidth: 1200, margin: '0 auto', display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
            {/* Header */}
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', flexWrap: 'wrap', gap: '1rem' }}>
                <div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.65rem' }}>
                        <h1 style={{ fontSize: '1.65rem', fontWeight: 700, letterSpacing: '-0.02em', color: '#fff' }}>
                            Universe Cockpit
                        </h1>
                        <span className="badge badge-success">
                            <span className="pulse-dot" style={{ width: 6, height: 6 }} /> Trực tuyến
                        </span>
                    </div>
                    <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginTop: '0.35rem' }}>
                        Trung tâm quản lý tài khoản & mô phỏng môi trường phân thân IDE chuyên dụng
                    </p>
                </div>

                <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                    <button className="btn" onClick={loadData} disabled={refreshing}>
                        <RefreshCw size={14} className={refreshing ? 'spin' : ''} /> Làm mới
                    </button>
                    <button className="btn btn-primary" onClick={() => navigate('/instances')}>
                        <Plus size={14} /> Tạo Profile mới
                    </button>
                </div>
            </div>

            {/* Cockpit Stats Row */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: '1rem' }}>
                {/* Stat 1: Total Accounts */}
                <div className="card" style={{ padding: '1.25rem', display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: 0 }}>
                    <div style={{
                        width: 48,
                        height: 48,
                        borderRadius: 14,
                        background: 'rgba(59, 130, 246, 0.12)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: 'var(--primary)',
                        flexShrink: 0
                    }}>
                        <Users size={24} />
                    </div>
                    <div>
                        <div style={{ fontSize: '1.6rem', fontWeight: 700, lineHeight: 1.1, color: '#fff' }}>
                            {loading ? '--' : accounts.length}
                        </div>
                        <div style={{ fontSize: '0.78rem', color: 'var(--text-secondary)', fontWeight: 500, marginTop: 4 }}>
                            Tài khoản IDE đã lưu
                        </div>
                    </div>
                </div>

                {/* Stat 2: Running Virtual Instances */}
                <div className="card" style={{ padding: '1.25rem', display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: 0 }}>
                    <div style={{
                        width: 48,
                        height: 48,
                        borderRadius: 14,
                        background: runningCount > 0 ? 'rgba(34, 197, 94, 0.14)' : 'rgba(148, 163, 184, 0.1)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: runningCount > 0 ? 'var(--success)' : 'var(--text-muted)',
                        flexShrink: 0
                    }}>
                        <Rocket size={24} />
                    </div>
                    <div>
                        <div style={{ fontSize: '1.6rem', fontWeight: 700, lineHeight: 1.1, color: '#fff' }}>
                            {loading ? '--' : `${runningCount} / ${totalProfiles}`}
                        </div>
                        <div style={{ fontSize: '0.78rem', color: 'var(--text-secondary)', fontWeight: 500, marginTop: 4 }}>
                            Instance đang chạy / Tổng Profiles
                        </div>
                    </div>
                </div>

                {/* Stat 3: Quota Health */}
                <div className="card" style={{ padding: '1.25rem', display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: 0 }}>
                    <div style={{
                        width: 48,
                        height: 48,
                        borderRadius: 14,
                        background: 'rgba(14, 165, 233, 0.12)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: 'var(--accent)',
                        flexShrink: 0
                    }}>
                        <Zap size={24} />
                    </div>
                    <div>
                        <div style={{ fontSize: '1.6rem', fontWeight: 700, lineHeight: 1.1, color: '#fff' }}>
                            100%
                        </div>
                        <div style={{ fontSize: '0.78rem', color: 'var(--text-secondary)', fontWeight: 500, marginTop: 4 }}>
                            Tỷ lệ token hợp lệ & Check-in
                        </div>
                    </div>
                </div>

                {/* Stat 4: Connected Platforms */}
                <div className="card" style={{ padding: '1.25rem', display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: 0 }}>
                    <div style={{
                        width: 48,
                        height: 48,
                        borderRadius: 14,
                        background: 'rgba(168, 85, 247, 0.12)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: '#c084fc',
                        flexShrink: 0
                    }}>
                        <Layers size={24} />
                    </div>
                    <div>
                        <div style={{ fontSize: '1.6rem', fontWeight: 700, lineHeight: 1.1, color: '#fff' }}>
                            4+
                        </div>
                        <div style={{ fontSize: '0.78rem', color: 'var(--text-secondary)', fontWeight: 500, marginTop: 4 }}>
                            Nền tảng IDE cô lập độc lập
                        </div>
                    </div>
                </div>
            </div>

            {/* Cockpit Platforms Cards Grid */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
                <h2 style={{ fontSize: '1.1rem', fontWeight: 600, color: '#fff', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <Sparkles size={18} color="var(--primary)" /> Bảng điều khiển Nền tảng
                </h2>

                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(360px, 1fr))', gap: '1.25rem' }}>
                    {/* Platform 1: CodeBuddy CN */}
                    <div className="card" style={{ display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
                        <div>
                            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '1rem' }}>
                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                                    <img src={codebuddyIcon} alt="CodeBuddy CN" className="nav-item-icon" style={{ width: 28, height: 28 }} />
                                    <div>
                                        <div style={{ fontWeight: 600, fontSize: '0.95rem', color: '#fff' }}>CodeBuddy CN</div>
                                        <div style={{ fontSize: '0.72rem', color: 'var(--text-secondary)' }}>Tencent Copilot (copilot.tencent.com)</div>
                                    </div>
                                </div>
                                <span className="badge badge-info">{codebuddyCnAccounts.length} Tài khoản</span>
                            </div>

                            <div style={{ background: 'rgba(0,0,0,0.2)', padding: '0.85rem', borderRadius: 'var(--radius-md)', marginBottom: '1rem' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem', marginBottom: '0.4rem' }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Trạng thái tài khoản:</span>
                                    <span style={{ color: 'var(--success)', fontWeight: 500 }}>
                                        {codebuddyCnAccounts.length > 0 ? 'Đã liên kết' : 'Chưa có tài khoản'}
                                    </span>
                                </div>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem' }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Cơ chế tiêm Token:</span>
                                    <span style={{ color: 'var(--text-primary)', fontFamily: 'var(--font-mono)', fontSize: '0.75rem' }}>
                                        planning-genie.new.accessTokencn
                                    </span>
                                </div>
                            </div>
                        </div>

                        <div style={{ display: 'flex', gap: '0.6rem', borderTop: '1px solid var(--border-light)', paddingTop: '0.85rem' }}>
                            <button
                                className="btn btn-primary"
                                style={{ flex: 1 }}
                                onClick={() => navigate('/platforms/codebuddy-cn')}
                            >
                                Quản lý & Đăng nhập
                            </button>
                            <button
                                className="btn"
                                onClick={() => navigate('/instances')}
                                title="Khởi chạy Profile"
                            >
                                <Rocket size={14} /> Chạy giả lập
                            </button>
                        </div>
                    </div>

                    {/* Platform 2: CodeBuddy Global */}
                    <div className="card" style={{ display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
                        <div>
                            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '1rem' }}>
                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                                    <img src={codebuddyIcon} alt="CodeBuddy Global" className="nav-item-icon" style={{ width: 28, height: 28 }} />
                                    <div>
                                        <div style={{ fontWeight: 600, fontSize: '0.95rem', color: '#fff' }}>CodeBuddy Global</div>
                                        <div style={{ fontSize: '0.72rem', color: 'var(--text-secondary)' }}>CodeBuddy.ai (Google OAuth / B3 Gateway)</div>
                                    </div>
                                </div>
                                <span className="badge badge-info">{codebuddyGlobalAccounts.length} Tài khoản</span>
                            </div>

                            <div style={{ background: 'rgba(0,0,0,0.2)', padding: '0.85rem', borderRadius: 'var(--radius-md)', marginBottom: '1rem' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem', marginBottom: '0.4rem' }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Trạng thái tài khoản:</span>
                                    <span style={{ color: 'var(--success)', fontWeight: 500 }}>
                                        {codebuddyGlobalAccounts.length > 0 ? 'Đã liên kết' : 'Chưa có tài khoản'}
                                    </span>
                                </div>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem' }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Cơ chế tiêm Token:</span>
                                    <span style={{ color: 'var(--text-primary)', fontFamily: 'var(--font-mono)', fontSize: '0.75rem' }}>
                                        planning-genie.new.accessToken
                                    </span>
                                </div>
                            </div>
                        </div>

                        <div style={{ display: 'flex', gap: '0.6rem', borderTop: '1px solid var(--border-light)', paddingTop: '0.85rem' }}>
                            <button
                                className="btn btn-primary"
                                style={{ flex: 1 }}
                                onClick={() => navigate('/platforms/codebuddy-global')}
                            >
                                Quản lý & Đăng nhập
                            </button>
                            <button
                                className="btn"
                                onClick={() => navigate('/instances')}
                                title="Khởi chạy Profile"
                            >
                                <Rocket size={14} /> Chạy giả lập
                            </button>
                        </div>
                    </div>

                    {/* Platform 3: Zed Editor */}
                    <div className="card" style={{ display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
                        <div>
                            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '1rem' }}>
                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                                    <img src={zedIcon} alt="Zed" className="nav-item-icon" style={{ width: 28, height: 28 }} />
                                    <div>
                                        <div style={{ fontWeight: 600, fontSize: '0.95rem', color: '#fff' }}>Zed Cloud</div>
                                        <div style={{ fontSize: '0.72rem', color: 'var(--text-secondary)' }}>Zed Editor (credentials.json)</div>
                                    </div>
                                </div>
                                <span className="badge badge-info">{zedAccounts.length} Tài khoản</span>
                            </div>

                            <div style={{ background: 'rgba(0,0,0,0.2)', padding: '0.85rem', borderRadius: 'var(--radius-md)', marginBottom: '1rem' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem', marginBottom: '0.4rem' }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Trạng thái xác thực:</span>
                                    <span style={{ color: 'var(--success)', fontWeight: 500 }}>
                                        {zedAccounts.length > 0 ? 'Sẵn sàng' : 'Chưa thêm'}
                                    </span>
                                </div>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem' }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Nơi lưu trữ Credentials:</span>
                                    <span style={{ color: 'var(--text-primary)', fontFamily: 'var(--font-mono)', fontSize: '0.75rem' }}>
                                        ~/.local/share/zed/db
                                    </span>
                                </div>
                            </div>
                        </div>

                        <div style={{ display: 'flex', gap: '0.6rem', borderTop: '1px solid var(--border-light)', paddingTop: '0.85rem' }}>
                            <button
                                className="btn btn-primary"
                                style={{ flex: 1 }}
                                onClick={() => navigate('/zed/accounts')}
                            >
                                Quản lý Zed
                            </button>
                            <button
                                className="btn"
                                onClick={() => navigate('/instances')}
                                title="Khởi chạy Profile"
                            >
                                <Rocket size={14} /> Chạy giả lập
                            </button>
                        </div>
                    </div>

                    {/* Platform 4: Cursor & Trae Simulator */}
                    <div className="card" style={{ display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
                        <div>
                            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '1rem' }}>
                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                                    <img src={cursorIcon} alt="Cursor" className="nav-item-icon" style={{ width: 28, height: 28 }} />
                                    <div>
                                        <div style={{ fontWeight: 600, fontSize: '0.95rem', color: '#fff' }}>Cursor & VS Code Forks</div>
                                        <div style={{ fontSize: '0.72rem', color: 'var(--text-secondary)' }}>Kho lưu trữ tài khoản đa phân thân</div>
                                    </div>
                                </div>
                                <span className="badge badge-success">Sẵn sàng</span>
                            </div>

                            <div style={{ background: 'rgba(0,0,0,0.2)', padding: '0.85rem', borderRadius: 'var(--radius-md)', marginBottom: '1rem' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem', marginBottom: '0.4rem' }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Giả lập Phần cứng:</span>
                                    <span style={{ color: 'var(--success)', fontWeight: 500 }}>
                                        Bảo vệ Telemetry ID
                                    </span>
                                </div>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem' }}>
                                    <span style={{ color: 'var(--text-secondary)' }}>Tiêm cơ sở dữ liệu:</span>
                                    <span style={{ color: 'var(--text-primary)', fontFamily: 'var(--font-mono)', fontSize: '0.75rem' }}>
                                        ItemTable (state.vscdb)
                                    </span>
                                </div>
                            </div>
                        </div>

                        <div style={{ display: 'flex', gap: '0.6rem', borderTop: '1px solid var(--border-light)', paddingTop: '0.85rem' }}>
                            <button
                                className="btn btn-primary"
                                style={{ flex: 1 }}
                                onClick={() => navigate('/accounts')}
                            >
                                Kho Tài khoản chung
                            </button>
                            <button
                                className="btn"
                                onClick={() => navigate('/instances')}
                                title="Quản lý Profiles"
                            >
                                <Rocket size={14} /> Mở Profiles
                            </button>
                        </div>
                    </div>
                </div>
            </div>

            {/* Quick Virtual Profiles Launcher Table */}
            <div className="card">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                        <Rocket size={20} color="var(--primary)" />
                        <h2 style={{ fontSize: '1.1rem', fontWeight: 600, color: '#fff' }}>
                            Danh sách Profile Ảo hóa (Instances)
                        </h2>
                    </div>
                    <button className="btn" onClick={() => navigate('/instances')}>
                        Xem tất cả ({profiles.length})
                    </button>
                </div>

                {profiles.length === 0 ? (
                    <div style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
                        <p>Chưa có profile nào được tạo. Hãy tạo một profile để mở IDE với phần cứng và tài khoản cô lập!</p>
                        <button className="btn btn-primary" style={{ marginTop: '1rem' }} onClick={() => navigate('/instances')}>
                            <Plus size={14} /> Tạo Profile đầu tiên
                        </button>
                    </div>
                ) : (
                    <table>
                        <thead>
                            <tr>
                                <th>Tên Profile</th>
                                <th>Nền tảng</th>
                                <th>Mã máy giả lập (Spoofing)</th>
                                <th>Trạng thái</th>
                                <th style={{ textAlign: 'right' }}>Thao tác tức thì</th>
                            </tr>
                        </thead>
                        <tbody>
                            {profiles.slice(0, 5).map((p) => {
                                const pid = runningInstances[p.id];
                                const isRunning = Boolean(pid);

                                return (
                                    <tr key={p.id}>
                                        <td>
                                            <div style={{ fontWeight: 600, color: '#fff' }}>{p.name}</div>
                                            <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>ID: {p.id.slice(0, 8)}...</div>
                                        </td>
                                        <td>
                                            <span className="badge badge-info">{p.platform_id}</span>
                                        </td>
                                        <td>
                                            <code style={{ fontSize: '0.75rem', background: 'rgba(0,0,0,0.3)', padding: '0.2rem 0.4rem', borderRadius: 4 }}>
                                                {p.hardware_fingerprint.machine_id.slice(0, 14)}...
                                            </code>
                                        </td>
                                        <td>
                                            {isRunning ? (
                                                <span className="badge badge-success">
                                                    <span className="pulse-dot" style={{ width: 6, height: 6 }} /> Đang chạy (PID {pid})
                                                </span>
                                            ) : (
                                                <span className="badge badge-warning">Đã dừng</span>
                                            )}
                                        </td>
                                        <td style={{ textAlign: 'right' }}>
                                            {isRunning ? (
                                                <button
                                                    className="btn btn-danger"
                                                    style={{ padding: '0.35rem 0.65rem', fontSize: '0.75rem' }}
                                                    onClick={() => handleQuickStop(p.id)}
                                                >
                                                    <Square size={12} /> Dừng
                                                </button>
                                            ) : (
                                                <button
                                                    className="btn btn-success"
                                                    style={{ padding: '0.35rem 0.65rem', fontSize: '0.75rem' }}
                                                    onClick={() => handleQuickLaunch(p.id)}
                                                    disabled={launchingId === p.id}
                                                >
                                                    <Play size={12} /> {launchingId === p.id ? 'Đang mở...' : 'Khởi chạy'}
                                                </button>
                                            )}
                                        </td>
                                    </tr>
                                );
                            })}
                        </tbody>
                    </table>
                )}
            </div>
        </div>
    );
}
