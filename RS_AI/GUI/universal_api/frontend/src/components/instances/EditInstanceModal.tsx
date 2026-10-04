import React from 'react';
import { X } from 'lucide-react';
import type { InstanceProfile, AccountInfo } from './types';

interface EditInstanceModalProps {
    isOpen: boolean;
    onClose: () => void;
    targetInstance: InstanceProfile | null;
    platformLabel: string;
    accounts: AccountInfo[];
    editName: string;
    setEditName: (name: string) => void;
    editBindAccountId: string;
    setEditBindAccountId: (id: string) => void;
    editExtraArgs: string;
    setEditExtraArgs: (args: string) => void;
    savingEdit: boolean;
    onSubmit: (e: React.FormEvent) => void;
}

export function EditInstanceModal({
    isOpen,
    onClose,
    targetInstance,
    platformLabel,
    accounts,
    editName,
    setEditName,
    editBindAccountId,
    setEditBindAccountId,
    editExtraArgs,
    setEditExtraArgs,
    savingEdit,
    onSubmit,
}: EditInstanceModalProps) {
    if (!isOpen || !targetInstance) return null;

    const isDef = Boolean(targetInstance.is_default || targetInstance.isDefault || targetInstance.id === 'default');

    return (
        <div
            style={{
                position: 'fixed',
                inset: 0,
                backgroundColor: 'rgba(0, 0, 0, 0.75)',
                backdropFilter: 'blur(4px)',
                zIndex: 9999,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                padding: '1rem',
            }}
        >
            <div
                style={{
                    backgroundColor: '#1e293b',
                    border: '1px solid #334155',
                    borderRadius: '0.75rem',
                    width: '100%',
                    maxWidth: '520px',
                    boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.5)',
                }}
            >
                {/* Header */}
                <div
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        padding: '1.25rem',
                        borderBottom: '1px solid #334155',
                    }}
                >
                    <h3 style={{ fontSize: '1.1rem', fontWeight: 600, color: '#f8fafc', margin: 0 }}>
                        Edit {platformLabel} Instance
                    </h3>
                    <button
                        onClick={onClose}
                        style={{
                            background: 'transparent',
                            border: 'none',
                            color: '#94a3b8',
                            cursor: 'pointer',
                        }}
                    >
                        <X size={18} />
                    </button>
                </div>

                <form onSubmit={onSubmit} style={{ padding: '1.25rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                    <div>
                        <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                            Instance Name
                        </label>
                        <input
                            type="text"
                            required
                            disabled={isDef}
                            value={editName}
                            onChange={(e) => setEditName(e.target.value)}
                            style={{
                                width: '100%',
                                backgroundColor: '#0f172a',
                                border: '1px solid #334155',
                                borderRadius: '0.375rem',
                                padding: '0.5rem 0.75rem',
                                color: '#f8fafc',
                                fontSize: '0.85rem',
                                outline: 'none',
                            }}
                        />
                    </div>

                    <div>
                        <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                            Bind Account
                        </label>
                        <select
                            value={editBindAccountId}
                            onChange={(e) => setEditBindAccountId(e.target.value)}
                            style={{
                                width: '100%',
                                backgroundColor: '#0f172a',
                                border: '1px solid #334155',
                                borderRadius: '0.375rem',
                                padding: '0.5rem 0.75rem',
                                color: '#f8fafc',
                                fontSize: '0.85rem',
                                outline: 'none',
                            }}
                        >
                            <option value="">-- Do not bind account --</option>
                            {accounts.map((a) => (
                                <option key={a.uid} value={a.uid}>
                                    {a.nickname || a.uid}
                                </option>
                            ))}
                        </select>
                    </div>

                    <div>
                        <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                            Custom launch args
                        </label>
                        <input
                            type="text"
                            placeholder="e.g. --disable-gpu"
                            value={editExtraArgs}
                            onChange={(e) => setEditExtraArgs(e.target.value)}
                            style={{
                                width: '100%',
                                backgroundColor: '#0f172a',
                                border: '1px solid #334155',
                                borderRadius: '0.375rem',
                                padding: '0.5rem 0.75rem',
                                color: '#f8fafc',
                                fontSize: '0.85rem',
                                outline: 'none',
                            }}
                        />
                    </div>

                    <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.6rem', marginTop: '0.75rem' }}>
                        <button
                            type="button"
                            onClick={onClose}
                            style={{
                                backgroundColor: '#0f172a',
                                border: '1px solid #334155',
                                borderRadius: '0.375rem',
                                color: '#94a3b8',
                                padding: '0.5rem 1rem',
                                fontSize: '0.85rem',
                                cursor: 'pointer',
                            }}
                        >
                            Cancel
                        </button>
                        <button
                            type="submit"
                            disabled={savingEdit}
                            style={{
                                backgroundColor: '#0284c7',
                                border: 'none',
                                borderRadius: '0.375rem',
                                color: '#ffffff',
                                padding: '0.5rem 1.25rem',
                                fontSize: '0.85rem',
                                fontWeight: 600,
                                cursor: 'pointer',
                            }}
                        >
                            {savingEdit ? 'Saving...' : 'Save Changes'}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}
