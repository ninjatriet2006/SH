/*
[INTEGRITY NOTES]
- Mục đích: Trang quản lý danh sách Người dùng và Đăng ký.
- Trách nhiệm: Hiển thị bảng User. Hỗ trợ Phân trang, Sắp xếp, Xuất CSV, và tìm kiếm.
- Tương tác: Dùng `useUserStore` và `useSubscriptionStore`.
*/

import React, { useEffect, useState, useMemo } from 'react';
import { useUserStore } from '../store/useUserStore';
import { useSubscriptionStore } from '../store/useSubscriptionStore';
import { usePackageStore } from '../store/usePackageStore';
import { UserModal } from '../components/UserModal';
import { SubscriptionModal } from '../components/SubscriptionModal';
import { ConfirmModal } from '../components/ConfirmModal';
import { downloadCSV } from '../utils/exportUtils';
import { Plus, Edit, Trash2, KeyRound, Download, ChevronUp, ChevronDown, ChevronLeft, ChevronRight } from 'lucide-react';
import type { User, Subscription } from '../../../bridge/types';
import { useTranslation, formatDateTime } from '../utils/i18n';

export function UserManagementPage() {
    const { t } = useTranslation();
    const { users, isLoading: userLoading, fetchUsers, addNewUser, editUser, removeUser } = useUserStore();
    const { subscriptions, isLoading: subLoading, fetchUserSubscriptions, addSubscription, removeSubscription, updateExpiry } = useSubscriptionStore();
    const { packages, fetchPackages } = usePackageStore();
    
    // State Modal User
    const [isUserModalOpen, setIsUserModalOpen] = useState(false);
    const [selectedUser, setSelectedUser] = useState<User | null>(null);

    // State Search & Filter
    const [searchTerm, setSearchTerm] = useState('');
    const [statusFilter, setStatusFilter] = useState<'ALL' | 'ACTIVE' | 'EXPIRING_SOON' | 'EXPIRED'>('ALL');

    // State Pagination & Sorting
    const [currentPage, setCurrentPage] = useState(1);
    const [itemsPerPage, setItemsPerPage] = useState(10);
    const [sortField, setSortField] = useState<keyof User>('created_at');
    const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('desc');

    // State Modal Subscription
    const [isSubscriptionModalOpen, setIsSubscriptionModalOpen] = useState(false);
    const [activeUserIdForSub, setActiveUserIdForSub] = useState<string>('');

    // Quản lý Confirm Delete Modal
    const [isConfirmOpen, setIsConfirmOpen] = useState(false);
    const [userToDelete, setUserToDelete] = useState<string | null>(null);

    const [selectedSubForEdit, setSelectedSubForEdit] = useState<Subscription | null>(null);

    // State Panel Chi Tiết Sub
    const [expandedUserId, setExpandedUserId] = useState<string | null>(null);

    useEffect(() => {
        fetchUsers();
        fetchPackages();
        useSubscriptionStore.getState().fetchAllSubscriptions();
    }, [fetchUsers, fetchPackages]);

    const handleAddUserClick = () => {
        setSelectedUser(null);
        setIsUserModalOpen(true);
    };

    const handleEditUserClick = (u: User) => {
        setSelectedUser(u);
        setIsUserModalOpen(true);
    };

    const requestDeleteUser = (id: string) => {
        setUserToDelete(id);
        setIsConfirmOpen(true);
    };

    const confirmDeleteUser = async () => {
        if (userToDelete) {
            await removeUser(userToDelete);
            setIsConfirmOpen(false);
            setUserToDelete(null);
        }
    };

    const handleSaveUser = async (username: string, email?: string, phone?: string, contactUrl?: string) => {
        if (selectedUser) {
            await editUser(selectedUser.id, username, email, phone, contactUrl);
        } else {
            await addNewUser(username, email, phone, contactUrl);
        }
    };

    const toggleSubDetails = async (userId: string) => {
        if (expandedUserId === userId) {
            setExpandedUserId(null); 
        } else {
            setExpandedUserId(userId);
            await fetchUserSubscriptions(userId);
        }
    };

    const handleAssignSub = (userId: string) => {
        setActiveUserIdForSub(userId);
        setSelectedSubForEdit(null);
        setIsSubscriptionModalOpen(true);
    };

    const handleEditSub = (sub: Subscription) => {
        setActiveUserIdForSub(expandedUserId || '');
        setSelectedSubForEdit(sub);
        setIsSubscriptionModalOpen(true);
    };

    const handleSaveSubscription = async (packageId: string, customExpiry?: number, amount?: number) => {
        if (selectedSubForEdit) {
            if (customExpiry) {
                await updateExpiry(selectedSubForEdit.id, customExpiry, amount);
            } else {
                alert("Vui lòng nhập ngày hết hạn mới để gia hạn!");
                return;
            }
        } else {
            await addSubscription(activeUserIdForSub, packageId, customExpiry, amount);
        }
        await fetchUserSubscriptions(activeUserIdForSub);
    };

    const handleSort = (field: keyof User) => {
        if (sortField === field) {
            setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
        } else {
            setSortField(field);
            setSortDirection('asc');
        }
    };

    // Filter, Sort, Paginate Logic
    const { allSubscriptions } = useSubscriptionStore();
    
    const filteredUsers = useMemo(() => {
        const now = Date.now();
        const SEVEN_DAYS_MS = 7 * 24 * 60 * 60 * 1000;

        let result = users.filter(u => {
            // Lọc theo Search Term
            const term = searchTerm.toLowerCase();
            const matchSearch = u.username.toLowerCase().includes(term) 
                || (u.email || '').toLowerCase().includes(term)
                || (u.phone || '').includes(term);

            if (!matchSearch) return false;

            // Lọc theo Trạng thái
            if (statusFilter === 'ALL') return true;

            const userSubs = allSubscriptions.filter(s => s.user_id === u.id);
            if (userSubs.length === 0) return statusFilter === 'EXPIRED'; // Nếu ko có gói nào, coi như Expired (hoặc có thể bỏ qua)

            let isExpiringSoon = false;
            let isActive = false;

            for (const sub of userSubs) {
                if (sub.is_active && sub.expiration_date > now) {
                    const timeRemaining = sub.expiration_date - now;
                    if (timeRemaining <= SEVEN_DAYS_MS) {
                        isExpiringSoon = true;
                    } else {
                        isActive = true;
                    }
                }
            }

            if (statusFilter === 'ACTIVE') return isActive;
            if (statusFilter === 'EXPIRING_SOON') return isExpiringSoon && !isActive;
            if (statusFilter === 'EXPIRED') return !isActive && !isExpiringSoon;

            return true;
        });

        result.sort((a, b) => {
            let valA = a[sortField];
            let valB = b[sortField];
            
            if (valA === null) valA = '';
            if (valB === null) valB = '';

            if (valA < valB) return sortDirection === 'asc' ? -1 : 1;
            if (valA > valB) return sortDirection === 'asc' ? 1 : -1;
            return 0;
        });

        return result;
    }, [users, searchTerm, sortField, sortDirection, statusFilter, allSubscriptions]);

    const totalPages = Math.ceil(filteredUsers.length / itemsPerPage);
    
    // Đảm bảo currentPage không vượt quá totalPages nếu bị filter
    useEffect(() => {
        if (currentPage > totalPages && totalPages > 0) {
            setCurrentPage(totalPages);
        }
    }, [totalPages, currentPage]);

    const paginatedUsers = useMemo(() => {
        const start = (currentPage - 1) * itemsPerPage;
        return filteredUsers.slice(start, start + itemsPerPage);
    }, [filteredUsers, currentPage, itemsPerPage]);

    const exportToCSV = () => {
        if (filteredUsers.length === 0) {
            alert("Không có dữ liệu để xuất!");
            return;
        }

        const header = "ID,Ngày tạo,Tên người dùng,Email,Số điện thoại,URL Liên hệ\n";
        let csvContent = header;

        filteredUsers.forEach(u => {
            const date = formatDateTime(u.created_at);
            // Wrap in quotes to avoid comma splitting issues
            const row = [
                `"${u.id}"`,
                `"${date}"`,
                `"${u.username.replace(/"/g, '""')}"`,
                `"${u.email || ''}"`,
                `"${u.phone || ''}"`,
                `"${u.contact_url || ''}"`
            ].join(",");
            csvContent += row + "\n";
        });

        downloadCSV(`users_export_${Date.now()}.csv`, csvContent);
    };

    return (
        <>
        <div className="animate-fade-in">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem', flexWrap: 'wrap', gap: '1rem' }}>
                <h1>{t('users.title')}</h1>
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                    <button className="btn" style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' }} onClick={exportToCSV}>
                        <Download size={18} /> {t('common.export_csv')}
                    </button>
                    <button className="btn btn-primary" onClick={handleAddUserClick}>
                        <Plus size={18} /> {t('users.add_user')}
                    </button>
                </div>
            </div>

            <div style={{ display: 'flex', gap: '1rem', marginBottom: '1.5rem', background: 'var(--bg-panel)', padding: '1rem', borderRadius: '8px', border: '1px solid var(--border)', flexWrap: 'wrap' }}>
                <div style={{ flex: 1, minWidth: '250px' }}>
                    <input 
                        type="text" 
                        className="input-field" 
                        placeholder={t('users.search')}
                        value={searchTerm}
                        onChange={e => { setSearchTerm(e.target.value); setCurrentPage(1); }}
                    />
                </div>
                <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                    <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{t('users.status')}</span>
                    <select 
                        className="input-field"
                        style={{ width: '150px' }}
                        value={statusFilter}
                        onChange={e => { setStatusFilter(e.target.value as any); setCurrentPage(1); }}
                    >
                        <option value="ALL">{t('common.all')}</option>
                        <option value="ACTIVE">{t('users.active')}</option>
                        <option value="EXPIRING_SOON">Sắp hết hạn (&lt; 7 ngày)</option>
                        <option value="EXPIRED">{t('users.inactive')}</option>
                    </select>
                </div>
                <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                    <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{t('common.show')}</span>
                    <select 
                        className="input-field"
                        style={{ width: '80px' }}
                        value={itemsPerPage}
                        onChange={e => { setItemsPerPage(Number(e.target.value)); setCurrentPage(1); }}
                    >
                        <option value={10}>10</option>
                        <option value={20}>20</option>
                        <option value={50}>50</option>
                        <option value={100}>100</option>
                    </select>
                </div>
            </div>

            <div className="glass-panel">
                {userLoading && users.length === 0 ? (
                    <p>{t('common.loading')}</p>
                ) : (
                    <div className="table-container">
                        <table>
                            <thead>
                                <tr>
                                    <th onClick={() => handleSort('created_at')} style={{ cursor: 'pointer' }}>
                                        {t('users.created_at')} {sortField === 'created_at' && (sortDirection === 'asc' ? <ChevronUp size={14} style={{display:'inline'}}/> : <ChevronDown size={14} style={{display:'inline'}}/>)}
                                    </th>
                                    <th onClick={() => handleSort('username')} style={{ cursor: 'pointer' }}>
                                        {t('users.username')} {sortField === 'username' && (sortDirection === 'asc' ? <ChevronUp size={14} style={{display:'inline'}}/> : <ChevronDown size={14} style={{display:'inline'}}/>)}
                                    </th>
                                    <th>{t('users.user_info')}</th>
                                    <th>{t('users.subs_info')}</th>
                                    <th>{t('users.actions')}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {paginatedUsers.length === 0 ? (
                                    <tr><td colSpan={5} style={{ textAlign: 'center' }}>{t('users.no_users')}</td></tr>
                                ) : (
                                    paginatedUsers.map(u => (
                                        <React.Fragment key={u.id}>
                                            <tr>
                                                <td style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
                                                    {formatDateTime(u.created_at, false)} <br/>
                                                    <span style={{ fontSize: '0.7rem', opacity: 0.5 }}>{u.id}</span>
                                                </td>
                                                <td style={{ fontWeight: 600 }}>{u.username}</td>
                                                <td>
                                                    {u.email && <div style={{ fontSize: '0.85rem' }}>📧 {u.email}</div>}
                                                    {u.phone && <div style={{ fontSize: '0.85rem' }}>📞 {u.phone}</div>}
                                                    {u.contact_url && (
                                                        <div style={{ fontSize: '0.85rem' }}>
                                                            🔗 <a href={u.contact_url} target="_blank" rel="noreferrer" style={{ color: 'var(--accent)' }}>Mở Link</a>
                                                        </div>
                                                    )}
                                                    {!u.email && !u.phone && !u.contact_url && '-'}
                                                </td>
                                                <td>
                                                    <button className="btn" style={{ background: 'rgba(255,255,255,0.05)', color: 'white', padding: '0.4rem 0.8rem' }} onClick={() => toggleSubDetails(u.id)}>
                                                        <KeyRound size={16} /> {t('users.subs_info')}
                                                    </button>
                                                </td>
                                                <td>
                                                    <div style={{ display: 'flex', gap: '0.5rem' }}>
                                                        <button className="btn btn-primary" style={{ padding: '0.4rem 0.6rem' }} onClick={() => handleEditUserClick(u)}>
                                                            <Edit size={16} />
                                                        </button>
                                                        <button className="btn" style={{ padding: '0.4rem', color: '#ef4444', border: '1px solid #ef4444' }} onClick={() => requestDeleteUser(u.id)} title="Xóa người dùng">
                                                            <Trash2 size={16} />
                                                        </button>
                                                    </div>
                                                </td>
                                            </tr>

                                            {expandedUserId === u.id && (
                                                <tr style={{ background: 'rgba(0,0,0,0.2)' }}>
                                                    <td colSpan={5} style={{ padding: '1rem 2rem' }}>
                                                        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '1rem' }}>
                                                            <h4 style={{ margin: 0 }}>Gói Đăng Ký Của: {u.username}</h4>
                                                            <button className="btn btn-primary" style={{ padding: '0.3rem 0.75rem', fontSize: '0.85rem' }} onClick={() => handleAssignSub(u.id)}>
                                                                + Gán Gói Mới
                                                            </button>
                                                        </div>
                                                        
                                                        {subLoading ? (
                                                            <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>Đang tải...</p>
                                                        ) : (
                                                            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '1rem' }}>
                                                                {subscriptions.length === 0 ? (
                                                                    <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{t('users.no_subs')}</p>
                                                                ) : (
                                                                    subscriptions.map(sub => (
                                                                        <div key={sub.id} style={{ border: '1px solid var(--border)', borderRadius: '8px', padding: '1rem', background: 'var(--bg-panel)', width: '300px' }}>
                                                                            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                                                                                <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                                                                                    {t('users.package')} <strong style={{ color: 'white' }}>{packages.find(p => p.id === sub.package_id)?.name || sub.package_id}</strong>
                                                                                </span>
                                                                                <span className={`badge ${sub.is_active ? 'badge-active' : 'badge-inactive'}`}>
                                                                                    {sub.is_active ? 'ACTIVE' : 'EXPIRED'}
                                                                                </span>
                                                                            </div>
                                                                            <div style={{ margin: '0.5rem 0' }}>
                                                                                <strong>{t('users.expiry')}</strong> {formatDateTime(sub.expiration_date)}
                                                                            </div>
                                                                            <div style={{ display: 'flex', gap: '0.5rem', marginTop: '1rem' }}>
                                                                                <button className="btn btn-primary" style={{ flex: 1, padding: '0.3rem', fontSize: '0.8rem' }} onClick={() => handleEditSub(sub)}>
                                                                                    <Edit size={14} /> {t('users.renew_edit')}
                                                                                </button>
                                                                                <button className="btn btn-danger" style={{ padding: '0.3rem', fontSize: '0.8rem' }} onClick={() => removeSubscription(sub.id)}>
                                                                                    <Trash2 size={14} />
                                                                                </button>
                                                                            </div>
                                                                        </div>
                                                                    ))
                                                                )}
                                                            </div>
                                                        )}
                                                    </td>
                                                </tr>
                                            )}
                                        </React.Fragment>
                                    ))
                                )}
                            </tbody>
                        </table>
                    </div>
                )}
                
                {/* Pagination Controls */}
                {!userLoading && filteredUsers.length > 0 && (
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '1rem', padding: '0.5rem 0' }}>
                        <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                            {t('common.showing')} {((currentPage - 1) * itemsPerPage) + 1} - {Math.min(currentPage * itemsPerPage, filteredUsers.length)} {t('common.of_total')} {filteredUsers.length}
                        </span>
                        <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                            <button 
                                className="btn" 
                                style={{ background: 'var(--bg-panel)', padding: '0.3rem' }} 
                                onClick={() => setCurrentPage(prev => Math.max(prev - 1, 1))}
                                disabled={currentPage === 1}
                            >
                                <ChevronLeft size={18} />
                            </button>
                            <span style={{ fontSize: '0.9rem', fontWeight: 600 }}>{currentPage} / {totalPages || 1}</span>
                            <button 
                                className="btn" 
                                style={{ background: 'var(--bg-panel)', padding: '0.3rem' }} 
                                onClick={() => setCurrentPage(prev => Math.min(prev + 1, totalPages))}
                                disabled={currentPage >= totalPages}
                            >
                                <ChevronRight size={18} />
                            </button>
                        </div>
                    </div>
                )}
            </div>
        </div>

            {/* Modals */}
            <UserModal isOpen={isUserModalOpen} userData={selectedUser} onClose={() => setIsUserModalOpen(false)} onSave={handleSaveUser} />
            <SubscriptionModal 
                isOpen={isSubscriptionModalOpen} 
                selectedUserId={activeUserIdForSub} 
                subscriptionData={selectedSubForEdit} 
                onClose={() => setIsSubscriptionModalOpen(false)} 
                onSave={handleSaveSubscription} 
            />

            {/* Modal Xác nhận Xóa */}
            <ConfirmModal 
                isOpen={isConfirmOpen}
                title="Xác nhận Xóa Người dùng"
                message="Bạn có chắc chắn muốn xóa người dùng này? Dữ liệu sẽ bị xóa vĩnh viễn và không thể khôi phục."
                onConfirm={confirmDeleteUser}
                onCancel={() => {
                    setIsConfirmOpen(false);
                    setUserToDelete(null);
                }}
                isDanger={true}
            />
        </>
    );
}
