import { FolderOpen } from 'lucide-react';

interface SessionsTabProps {
    platformLabel: string;
    onSyncSessions: () => void;
    onCleanSessions: () => void;
}

export function SessionsTab({
    platformLabel,
    onSyncSessions,
    onCleanSessions,
}: SessionsTabProps) {
    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
            <div className="card" style={{ padding: '1.25rem' }}>
                <h3 style={{ fontSize: '1rem', fontWeight: 600, color: '#f8fafc', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <FolderOpen size={18} color="#38bdf8" /> {platformLabel} Session & Chat Threads Manager
                </h3>
                <p style={{ fontSize: '0.8rem', color: '#94a3b8', marginTop: 4 }}>
                    Quản lý các phiên trò chuyện AI, đồng bộ lịch sử hội thoại giữa các cửa sổ IDE và dọn dẹp các session rác để tiết kiệm dung lượng.
                </p>
                <div style={{ display: 'flex', gap: '0.75rem', marginTop: '1rem' }}>
                    <button className="btn btn-primary" style={{ fontSize: '0.8rem' }} onClick={onSyncSessions}>
                        Đồng bộ Session giữa các Instance
                    </button>
                    <button className="btn" style={{ fontSize: '0.8rem' }} onClick={onCleanSessions}>
                        Dọn dẹp Session rác
                    </button>
                </div>
            </div>
        </div>
    );
}
