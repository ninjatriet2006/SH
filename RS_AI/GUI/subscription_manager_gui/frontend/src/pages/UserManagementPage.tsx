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
import { BalanceModal } from '../components/BalanceModal';
import { ConfirmModal } from '../components/ConfirmModal';
import { downloadCSV } from '../utils/exportUtils';
import { Plus, Edit, Trash2, KeyRound, Download, ChevronUp, ChevronDown, ChevronLeft, ChevronRight, Wallet, RefreshCw } from 'lucide-react';
import type { User, Subscription } from '../../../bridge/types';
import { useTranslation, formatDateTime, formatCurrency } from '../utils/i18n';

export function UserManagementPage() {
    const { t } = useTranslation();
    const { users, isLoading: userLoading, fetchUsers, addNewUser, editUser, removeUser, changeBalance } = useUserStore();
    const { subscriptions, allSubscriptions, isLoading: subLoading, fetchAllSubscriptions, fetchUserSubscriptions, addSubscription, removeSubscription, updateExpiry, toggleAutoRenew, runAutoRenewals } = useSubscriptionStore();
    const { packages, fetchPackages } = usePackageStore();
    
    // State Modal User
    const [isUserModalOpen, setIsUserModalOpen] = useState(false);
    const [selectedUser, setSelectedUser] = useState<User | null>(null);

    // State Modal điều chỉnh số dư
    const [balanceUser, setBalanceUser] = useState<User | null>(null);

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

    // App đã rà auto-renew một lần lúc khởi động (xem `App.tsx`). Ở đây chỉ
    // ĐỌC LẠI báo cáo đó để hiển thị — không rà lần nữa, tránh trừ tiền hai
    // lượt và tránh phụ thuộc vào việc người dùng có mở trang này hay không.
    const [autoRenewNotice, setAutoRenewNotice] = useState<string | null>(null);
    const { autoRenewReport, clearAutoRenewReport } = useSubscriptionStore();
    useEffect(() => {
        if (!autoRenewReport) return;
        const { renewed, total_charged, skipped } = autoRenewReport;
        if (renewed > 0) {
            setAutoRenewNotice(
                `${t('auto_renew.renewed')}: ${renewed} — ${t('auto_renew.charged')}: ${formatCurrency(total_charged)}`
            );
        } else if (skipped.length > 0) {
            setAutoRenewNotice(`${t('auto_renew.skipped')}: ${skipped.length} (${skipped[0].reason})`);
        }
        // Đọc xong thì xoá để lần vào trang sau không hiện lại thông báo cũ.
        clearAutoRenewReport();
    }, [autoRenewReport, clearAutoRenewReport, t]);

    const handleRunAutoRenew = async () => {
        try {
            const report = await runAutoRenewals();
            await fetchUsers();
            if (report.renewed === 0 && report.skipped.length === 0) {
                setAutoRenewNotice(t('auto_renew.none'));
            } else {
                const parts = [
                    `${t('auto_renew.renewed')}: ${report.renewed}`,
                    `${t('auto_renew.charged')}: ${formatCurrency(report.total_charged)}`,
                ];
                if (report.skipped.length > 0) {
                    parts.push(`${t('auto_renew.skipped')}: ${report.skipped.length} (${report.skipped[0].reason})`);
                }
                setAutoRenewNotice(parts.join(' — '));
            }
            clearAutoRenewReport();
        } catch (err) {
            alert(`Chạy gia hạn tự động thất bại: ${err instanceof Error ? err.message : String(err)}`);
        }
    };

    const handleToggleAutoRenew = async (sub: Subscription) => {
        try {
            await toggleAutoRenew(sub.id, !sub.auto_renew);
        } catch (err) {
            alert(`Không đổi được tự động gia hạn: ${err instanceof Error ? err.message : String(err)}`);
        }
    };

    const handleAdjustBalance = async (delta: number) => {
        if (!balanceUser) return;
        await changeBalance(balanceUser.id, delta);
    };

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
            // Backend xóa cascade subscription/transaction, nạp lại để state
            // zustand không giữ xác chết làm filter trạng thái hiển thị sai.
            await fetchAllSubscriptions();
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

    const handleSaveSubscription = async (packageId: string, customExpiry?: number, amount?: number, autoRenew?: boolean) => {
        if (selectedSubForEdit) {
            if (customExpiry) {
                await updateExpiry(selectedSubForEdit.id, customExpiry, amount, autoRenew);
            } else {
                alert("Vui lòng nhập ngày hết hạn mới để gia hạn!");
                return;
            }
        } else {
            await addSubscription(activeUserIdForSub, packageId, customExpiry, amount, autoRenew);
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
    // (allSubscriptions lấy từ subscribe ở đầu component — không subscribe trùng.)
    
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
            // Không có gói nào: coi như hết hạn (quy ước hiển thị, ghi rõ ở đây
            // để không ai "sửa" thành ALL vì tưởng là bug).
            if (userSubs.length === 0) return statusFilter === 'EXPIRED';

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
            // User có gói sắp hết hạn vẫn hiện ở đây dù còn gói active dài hạn —
            // trước đây `isExpiringSoon && !isActive` giấu họ khỏi cả 2 bộ lọc.
            // Một user có thể xuất hiện ở cả ACTIVE lẫn EXPIRING_SOON: đúng ý đồ.
            if (statusFilter === 'EXPIRING_SOON') return isExpiringSoon;
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

        const header = "ID,Ngày tạo,Tên người dùng,Email,Số điện thoại,URL Liên hệ,Số dư\n";
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
                `"${u.contact_url || ''}"`,
                // Số thô (không định dạng) để mở bằng Excel còn tính toán được.
                `"${u.balance}"`
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
                    <button
                        className="btn"
                        style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' }}
                        onClick={handleRunAutoRenew}
                        title={t('auto_renew.run_now')}
                    >
                        <RefreshCw size={18} /> {t('auto_renew.run_now')}
                    </button>
                    <button className="btn" style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' }} onClick={exportToCSV}>
                        <Download size={18} /> {t('common.export_csv')}
                    </button>
                    <button className="btn btn-primary" onClick={handleAddUserClick}>
                        <Plus size={18} /> {t('users.add_user')}
                    </button>
                </div>
            </div>

            {/* Thông báo kết quả tự động gia hạn — tiền của khách bị trừ thì
                người dùng phải thấy, không thay đổi âm thầm. */}
            {autoRenewNotice && (
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '1rem', padding: '0.75rem 1rem', marginBottom: '1rem', background: 'rgba(99, 102, 241, 0.15)', borderLeft: '4px solid var(--primary)', borderRadius: '4px' }}>
                    <span style={{ fontSize: '0.9rem' }}>
                        <strong>{t('auto_renew.report_title')}:</strong> {autoRenewNotice}
                    </span>
                    <button
                        className="btn"
                        style={{ padding: '0.2rem 0.5rem', background: 'transparent', color: 'var(--text-secondary)' }}
                        onClick={() => setAutoRenewNotice(null)}
                    >
                        ✕
                    </button>
                </div>
            )}

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
                    <label htmlFor="user-status-filter" style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{t('users.status')}</label>
                    <select 
                        id="user-status-filter"
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
                    <label htmlFor="users-per-page" style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>{t('common.show')}</label>
                    <select 
                        id="users-per-page"
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
                                    <th onClick={() => handleSort('balance')} style={{ cursor: 'pointer' }}>
                                        {t('users.balance')} {sortField === 'balance' && (sortDirection === 'asc' ? <ChevronUp size={14} style={{display:'inline'}}/> : <ChevronDown size={14} style={{display:'inline'}}/>)}
                                    </th>
                                    <th>{t('users.user_info')}</th>
                                    <th>{t('users.subs_info')}</th>
                                    <th>{t('users.actions')}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {paginatedUsers.length === 0 ? (
                                    <tr><td colSpan={6} style={{ textAlign: 'center' }}>{t('users.no_users')}</td></tr>
                                ) : (
                                    paginatedUsers.map(u => (
                                        <React.Fragment key={u.id}>
                                            <tr>
                                                <td style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
                                                    {formatDateTime(u.created_at, false)} <br/>
                                                    <span style={{ fontSize: '0.7rem', opacity: 0.5 }}>{u.id}</span>
                                                </td>
                                                <td style={{ fontWeight: 600 }}>{u.username}</td>
                                                {/* Số dư: âm = công nợ, tô đỏ để nhìn ra ngay */}
                                                <td>
                                                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                                                        <span style={{ fontWeight: 600, color: u.balance < 0 ? 'var(--danger)' : (u.balance > 0 ? 'var(--success)' : 'var(--text-secondary)') }}>
                                                            {formatCurrency(u.balance)}
                                                        </span>
                                                        <button
                                                            className="btn"
                                                            style={{ padding: '0.2rem 0.4rem', background: 'rgba(255,255,255,0.08)' }}
                                                            onClick={() => setBalanceUser(u)}
                                                            title={t('users.balance_adjust')}
                                                        >
                                                            <Wallet size={14} />
                                                        </button>
                                                    </div>
                                                    {u.balance < 0 && (
                                                        <span style={{ fontSize: '0.7rem', color: 'var(--danger)' }}>{t('users.balance_debt')}</span>
                                                    )}
                                                </td>
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
                                                    <td colSpan={6} style={{ padding: '1rem 2rem' }}>
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

                                                                            {/* Tự động gia hạn: theo TỪNG gói đăng ký */}
                                                                            <label style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', cursor: 'pointer', fontSize: '0.8rem', marginTop: '0.5rem' }}>
                                                                                <input
                                                                                    type="checkbox"
                                                                                    checked={sub.auto_renew}
                                                                                    onChange={() => handleToggleAutoRenew(sub)}
                                                                                    style={{ width: '14px', height: '14px', cursor: 'pointer' }}
                                                                                />
                                                                                <RefreshCw size={12} style={{ color: sub.auto_renew ? 'var(--success)' : 'var(--text-secondary)' }} />
                                                                                <span style={{ color: sub.auto_renew ? 'var(--success)' : 'var(--text-secondary)' }}>
                                                                                    {t('users.auto_renew')}
                                                                                </span>
                                                                            </label>
                                                                            {sub.last_auto_renew_at && (
                                                                                <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', marginTop: '0.25rem' }}>
                                                                                    {t('users.last_auto_renew')} {formatDateTime(sub.last_auto_renew_at)}
                                                                                </div>
                                                                            )}

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
                subscriptionData={selectedSubForEdit} 
                onClose={() => setIsSubscriptionModalOpen(false)} 
                onSave={handleSaveSubscription} 
            />
            <BalanceModal
                isOpen={balanceUser !== null}
                user={balanceUser}
                onClose={() => setBalanceUser(null)}
                onSave={handleAdjustBalance}
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
