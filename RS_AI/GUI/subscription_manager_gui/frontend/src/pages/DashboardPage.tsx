/*
[INTEGRITY NOTES]
- Mục đích: Trang tổng quan (Dashboard).
- Trách nhiệm: Hiển thị các chỉ số tổng quan như tổng số lượng người dùng, số lượng gói dịch vụ đang có.
- Tương tác: Lấy dữ liệu từ store.
*/

import { useEffect } from 'react';
import { useUserStore } from '../store/useUserStore';
import { usePackageStore } from '../store/usePackageStore';
import { useTranslation } from '../utils/i18n';
import { Users, PackageSearch } from 'lucide-react';

export function DashboardPage() {
    // Lấy state và hàm fetch từ stores
    const { t } = useTranslation();
    const { users, fetchUsers } = useUserStore();
    const { packages, fetchPackages } = usePackageStore();

    // Lần đầu render, tải dữ liệu nếu chưa có
    useEffect(() => {
        if (users.length === 0) fetchUsers();
        if (packages.length === 0) fetchPackages();
    }, [users.length, packages.length, fetchUsers, fetchPackages]);

    return (
        <div className="animate-fade-in">
            <h1>{t('dashboard.title')}</h1>
            <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>{t('common.loading').replace('...', '')}</p>
            
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(250px, 1fr))', gap: '1.5rem' }}>
                {/* Thẻ thống kê Người dùng */}
                <div className="glass-panel" style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <div style={{ padding: '1rem', background: 'rgba(99, 102, 241, 0.1)', borderRadius: '12px', color: 'var(--primary)' }}>
                        <Users size={32} />
                    </div>
                    <div>
                        <h2 style={{ margin: 0, fontSize: '2rem' }}>{users.length}</h2>
                        <span style={{ color: 'var(--text-secondary)' }}>{t('dashboard.total_users')}</span>
                    </div>
                </div>
                
                {/* Thẻ thống kê Gói dịch vụ */}
                <div className="glass-panel" style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <div style={{ padding: '1rem', background: 'rgba(16, 185, 129, 0.1)', borderRadius: '12px', color: 'var(--success)' }}>
                        <PackageSearch size={32} />
                    </div>
                    <div>
                        <h2 style={{ margin: 0, fontSize: '2rem' }}>{packages.length}</h2>
                        <span style={{ color: 'var(--text-secondary)' }}>{t('sidebar.packages')}</span>
                    </div>
                </div>
            </div>
        </div>
    );
}
