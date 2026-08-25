import { useEffect, useState, useMemo } from 'react';
import { useTransactionStore } from '../store/useTransactionStore';
import { usePackageStore } from '../store/usePackageStore';
import { useUserStore } from '../store/useUserStore';
import { Download, ChevronUp, ChevronDown, ChevronLeft, ChevronRight, Printer, Trash2, Layers } from 'lucide-react';
import { InvoiceModal } from '../components/InvoiceModal';
import { ConfirmModal } from '../components/ConfirmModal';
import { downloadCSV } from '../utils/exportUtils';
import type { Transaction, User } from '../../../bridge/types';
import { useTranslation, formatDateTime, formatCurrency } from '../utils/i18n';

export function TransactionManagementPage() {
    const { t } = useTranslation();
    const { transactions, isLoading: txLoading, fetchAllTransactions, removeTransaction } = useTransactionStore();
    const { packages, fetchPackages } = usePackageStore();
    const { users, fetchUsers } = useUserStore();

    // State Pagination & Sorting
    const [currentPage, setCurrentPage] = useState(1);
    const [itemsPerPage, setItemsPerPage] = useState(10);
    const [sortField, setSortField] = useState<keyof Transaction>('created_at');
    const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>('desc');

    // State Selection for Group Invoice
    const [selectedTxIds, setSelectedTxIds] = useState<string[]>([]);

    // State Invoice Modal
    const [isInvoiceOpen, setIsInvoiceOpen] = useState(false);
    const [selectedInvoiceTxs, setSelectedInvoiceTxs] = useState<Transaction[]>([]);
    const [invoiceUser, setInvoiceUser] = useState('');

    // State Confirm Delete
    const [isConfirmOpen, setIsConfirmOpen] = useState(false);
    const [txToDelete, setTxToDelete] = useState<string | null>(null);

    useEffect(() => {
        fetchAllTransactions();
        fetchPackages();
        fetchUsers();
    }, [fetchAllTransactions, fetchPackages, fetchUsers]);

    const handleSort = (field: keyof Transaction) => {
        if (sortField === field) {
            setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
        } else {
            setSortField(field);
            setSortDirection('desc');
        }
    };

    // Lọc & Sắp xếp
    const sortedTransactions = useMemo(() => {
        let result = [...transactions];
        result.sort((a, b) => {
            let valA = a[sortField];
            let valB = b[sortField];
            if (valA < valB) return sortDirection === 'asc' ? -1 : 1;
            if (valA > valB) return sortDirection === 'asc' ? 1 : -1;
            return 0;
        });
        return result;
    }, [transactions, sortField, sortDirection]);

    const totalPages = Math.ceil(sortedTransactions.length / itemsPerPage);

    useEffect(() => {
        if (currentPage > totalPages && totalPages > 0) {
            setCurrentPage(totalPages);
        }
    }, [totalPages, currentPage]);

    const paginatedTransactions = useMemo(() => {
        const start = (currentPage - 1) * itemsPerPage;
        return sortedTransactions.slice(start, start + itemsPerPage);
    }, [sortedTransactions, currentPage, itemsPerPage]);

    // Handle Selection
    const handleSelectRow = (id: string) => {
        setSelectedTxIds(prev => 
            prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id]
        );
    };

    const handleSelectAll = (e: React.ChangeEvent<HTMLInputElement>) => {
        if (e.target.checked) {
            setSelectedTxIds(paginatedTransactions.map(t => t.id));
        } else {
            setSelectedTxIds([]);
        }
    };

    const requestDeleteTransaction = (id: string) => {
        setTxToDelete(id);
        setIsConfirmOpen(true);
    };

    const confirmDeleteTransaction = async () => {
        if (!txToDelete) return;
        try {
            await removeTransaction(txToDelete);
            setSelectedTxIds(prev => prev.filter(x => x !== txToDelete));
            setIsConfirmOpen(false);
            setTxToDelete(null);
        } catch (error) {
            alert("Xóa giao dịch thất bại: " + error);
        }
    };

    const exportToCSV = () => {
        if (sortedTransactions.length === 0) {
            alert("Không có dữ liệu để xuất!");
            return;
        }

        const header = "Mã GD,Thời gian,ID Khách hàng,Tên Khách hàng,Tên Gói Dịch Vụ,Hành động,Số tiền (VNĐ)\n";
        let csvContent = header;

        sortedTransactions.forEach(tx => {
            const user = users.find(u => u.id === tx.user_id);
            const pkg = packages.find(p => p.id === tx.package_id);
            
            const date = formatDateTime(tx.created_at);
            const username = user ? user.username : tx.user_id;
            const pkgName = pkg ? pkg.name : tx.package_id;
            const actionText = tx.action === 'ASSIGN' ? t('transactions.assign') : t('transactions.renew');
            
            const row = [
                `"${tx.id}"`,
                `"${date}"`,
                `"${tx.user_id}"`,
                `"${username.replace(/"/g, '""')}"`,
                `"${pkgName.replace(/"/g, '""')}"`,
                `"${actionText}"`,
                `"${tx.amount}"`
            ].join(",");
            csvContent += row + "\n";
        });

        downloadCSV(`transactions_export_${Date.now()}.csv`, csvContent);
    };

    const handleOpenInvoice = (tx: Transaction, user: User | undefined) => {
        setSelectedInvoiceTxs([tx]);
        setInvoiceUser(user ? user.username : tx.user_id);
        setIsInvoiceOpen(true);
    };

    const handleOpenGroupInvoice = () => {
        if (selectedTxIds.length === 0) return;
        
        const selectedTxs = transactions.filter(t => selectedTxIds.includes(t.id));
        
        // Kiểm tra xem tất cả các giao dịch có thuộc về cùng 1 khách hàng không
        const userIds = new Set(selectedTxs.map(t => t.user_id));
        if (userIds.size > 1) {
            alert("Chỉ có thể gom hóa đơn cho CÙNG MỘT khách hàng!");
            return;
        }

        const userId = Array.from(userIds)[0];
        const user = users.find(u => u.id === userId);
        
        setSelectedInvoiceTxs(selectedTxs);
        setInvoiceUser(user ? user.username : userId);
        setIsInvoiceOpen(true);
    };

    return (
        <>
        <div className="animate-fade-in">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '2rem', flexWrap: 'wrap', gap: '1rem' }}>
                <h1>{t('transactions.title')}</h1>
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                    {selectedTxIds.length > 0 && (
                        <button className="btn btn-primary" onClick={handleOpenGroupInvoice}>
                            <Layers size={18} /> {t('transactions.group_invoice')} ({selectedTxIds.length})
                        </button>
                    )}
                    <button className="btn" style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border)' }} onClick={exportToCSV}>
                        <Download size={18} /> {t('common.export_csv')}
                    </button>
                </div>
            </div>

            <div style={{ display: 'flex', gap: '1rem', marginBottom: '1.5rem', background: 'var(--bg-panel)', padding: '1rem', borderRadius: '8px', border: '1px solid var(--border)', justifyContent: 'flex-end' }}>
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
                {txLoading && transactions.length === 0 ? (
                    <p>{t('common.loading')}</p>
                ) : (
                    <div className="table-container">
                        <table>
                            <thead>
                                <tr>
                                    <th style={{ width: '40px' }}>
                                        <input 
                                            type="checkbox" 
                                            checked={paginatedTransactions.length > 0 && selectedTxIds.length === paginatedTransactions.length}
                                            onChange={handleSelectAll}
                                        />
                                    </th>
                                    <th onClick={() => handleSort('id')} style={{ cursor: 'pointer' }}>
                                        {t('transactions.tx_id')} {sortField === 'id' && (sortDirection === 'asc' ? <ChevronUp size={14} style={{display:'inline'}}/> : <ChevronDown size={14} style={{display:'inline'}}/>)}
                                    </th>
                                    <th onClick={() => handleSort('created_at')} style={{ cursor: 'pointer' }}>
                                        {t('transactions.time')} {sortField === 'created_at' && (sortDirection === 'asc' ? <ChevronUp size={14} style={{display:'inline'}}/> : <ChevronDown size={14} style={{display:'inline'}}/>)}
                                    </th>
                                    <th>{t('transactions.user')}</th>
                                    <th>{t('transactions.package')}</th>
                                    <th onClick={() => handleSort('action')} style={{ cursor: 'pointer' }}>
                                        {t('transactions.action')} {sortField === 'action' && (sortDirection === 'asc' ? <ChevronUp size={14} style={{display:'inline'}}/> : <ChevronDown size={14} style={{display:'inline'}}/>)}
                                    </th>
                                    <th onClick={() => handleSort('amount')} style={{ cursor: 'pointer' }}>
                                        {t('transactions.amount')} {sortField === 'amount' && (sortDirection === 'asc' ? <ChevronUp size={14} style={{display:'inline'}}/> : <ChevronDown size={14} style={{display:'inline'}}/>)}
                                    </th>
                                    <th>{t('transactions.actions')}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {paginatedTransactions.length === 0 ? (
                                    <tr><td colSpan={8} style={{ textAlign: 'center' }}>{t('transactions.no_txs')}</td></tr>
                                ) : (
                                    paginatedTransactions.map(tx => {
                                        const user = users.find(u => u.id === tx.user_id);
                                        const pkg = packages.find(p => p.id === tx.package_id);
                                        return (
                                            <tr key={tx.id} style={{ background: selectedTxIds.includes(tx.id) ? 'var(--bg-hover)' : 'transparent' }}>
                                                <td>
                                                    <input 
                                                        type="checkbox" 
                                                        checked={selectedTxIds.includes(tx.id)}
                                                        onChange={() => handleSelectRow(tx.id)}
                                                    />
                                                </td>
                                                <td style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>{tx.id}</td>
                                                <td>{formatDateTime(tx.created_at)}</td>
                                                <td style={{ fontWeight: 600 }}>{user ? user.username : tx.user_id}</td>
                                                <td>{pkg ? pkg.name : tx.package_id}</td>
                                                <td>
                                                    <span className={`badge ${tx.action === 'ASSIGN' ? 'badge-active' : 'badge-inactive'}`}>
                                                        {tx.action === 'ASSIGN' ? t('transactions.assign') : t('transactions.renew')}
                                                    </span>
                                                </td>
                                                <td style={{ color: 'var(--success-color, #34d399)', fontWeight: 'bold' }}>
                                                    {tx.amount > 0 ? `+${formatCurrency(tx.amount)}` : '0 đ'}
                                                </td>
                                                <td>
                                                    <div style={{ display: 'flex', gap: '0.5rem' }}>
                                                        <button className="btn btn-primary" style={{ padding: '0.4rem' }} onClick={() => handleOpenInvoice(tx, user)} title="In Hóa Đơn">
                                                            <Printer size={16} />
                                                        </button>
                                                        <button className="btn" style={{ padding: '0.4rem', color: '#ef4444', border: '1px solid #ef4444' }} onClick={() => requestDeleteTransaction(tx.id)} title="Xóa giao dịch">
                                                            <Trash2 size={16} />
                                                        </button>
                                                    </div>
                                                </td>
                                            </tr>
                                        );
                                    })
                                )}
                            </tbody>
                        </table>
                    </div>
                )}

                {/* Pagination Controls */}
                {!txLoading && sortedTransactions.length > 0 && (
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '1rem', padding: '0.5rem 0' }}>
                        <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                            {t('common.showing')} {((currentPage - 1) * itemsPerPage) + 1} - {Math.min(currentPage * itemsPerPage, sortedTransactions.length)} {t('common.of_total')} {sortedTransactions.length}
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

            {/* Invoice Modal */}
            <InvoiceModal
                isOpen={isInvoiceOpen}
                transactions={selectedInvoiceTxs}
                username={invoiceUser}
                packages={packages}
                onClose={() => setIsInvoiceOpen(false)}
            />

            {/* Modal Xác nhận Xóa */}
            <ConfirmModal 
                isOpen={isConfirmOpen}
                title="Xác nhận Xóa Giao dịch"
                message="Bạn có chắc chắn muốn xóa giao dịch này? Dữ liệu sẽ bị xóa vĩnh viễn và không thể khôi phục."
                onConfirm={confirmDeleteTransaction}
                onCancel={() => {
                    setIsConfirmOpen(false);
                    setTxToDelete(null);
                }}
                isDanger={true}
            />
        </>
    );
}
