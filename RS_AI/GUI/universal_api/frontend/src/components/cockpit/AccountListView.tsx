import { Play, Trash2 } from 'lucide-react';
import type { AccountInfo } from './types';

interface AccountListViewProps {
    displayedAccounts: AccountInfo[];
    selectedIds: Set<string>;
    toggleSelectAll: () => void;
    toggleSelect: (uid: string) => void;
    onSwitchAccount: (account: AccountInfo) => void;
    onDelete: (uid: string) => void;
    maskValue: (val: string) => string;
}

export function AccountListView({
    displayedAccounts,
    selectedIds,
    toggleSelectAll,
    toggleSelect,
    onSwitchAccount,
    onDelete,
    maskValue,
}: AccountListViewProps) {
    return (
        <div className="card accounts-table-card" style={{ padding: 0, overflow: 'hidden' }}>
            <table>
                <thead>
                    <tr>
                        <th style={{ width: 40 }}>
                            <input
                                type="checkbox"
                                checked={selectedIds.size > 0 && selectedIds.size === displayedAccounts.length}
                                onChange={toggleSelectAll}
                            />
                        </th>
                        <th>Tài khoản</th>
                        <th>Gói cước</th>
                        <th>Trạng thái</th>
                        <th>Quota còn lại</th>
                        <th style={{ textAlign: 'right' }}>Thao tác</th>
                    </tr>
                </thead>
                <tbody>
                    {displayedAccounts.map((account) => (
                        <tr key={account.uid}>
                            <td>
                                <input
                                    type="checkbox"
                                    checked={selectedIds.has(account.uid)}
                                    onChange={() => toggleSelect(account.uid)}
                                />
                            </td>
                            <td>
                                <div style={{ fontWeight: 600, color: '#fff' }}>
                                    {maskValue(account.nickname || account.uid)}
                                </div>
                                <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>
                                    UID: {account.uid.slice(0, 12)}...
                                </div>
                            </td>
                            <td>
                                <span className="badge badge-info">{account.plan_tier || 'FREE'}</span>
                            </td>
                            <td>
                                <span className="badge badge-success">Normal</span>
                            </td>
                            <td>
                                <div style={{ fontSize: '0.8rem', color: 'var(--success)', fontWeight: 600 }}>
                                    0 / 100
                                </div>
                            </td>
                            <td style={{ textAlign: 'right' }}>
                                <div style={{ display: 'inline-flex', gap: '0.35rem' }}>
                                    <button
                                        className="btn btn-primary"
                                        style={{ padding: '0.3rem 0.6rem', fontSize: '0.75rem' }}
                                        onClick={() => onSwitchAccount(account)}
                                    >
                                        <Play size={12} /> Chuyển
                                    </button>
                                    <button
                                        className="btn btn-danger"
                                        style={{ padding: '0.3rem 0.6rem', fontSize: '0.75rem' }}
                                        onClick={() => onDelete(account.uid)}
                                    >
                                        <Trash2 size={12} />
                                    </button>
                                </div>
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}
