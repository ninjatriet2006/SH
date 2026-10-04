import React from 'react';
import { X } from 'lucide-react';
import type { InstanceProfile, AccountInfo } from './types';

interface CreateInstanceModalProps {
    isOpen: boolean;
    onClose: () => void;
    platformId: string;
    platformLabel: string;
    instances: InstanceProfile[];
    accounts: AccountInfo[];
    newInstanceName: string;
    setNewInstanceName: (name: string) => void;
    initMode: 'copy_source' | 'blank' | 'existing_dir';
    setInitMode: (mode: 'copy_source' | 'blank' | 'existing_dir') => void;
    sourceInstanceId: string;
    setSourceInstanceId: (id: string) => void;
    existingDir: string;
    setExistingDir: (dir: string) => void;
    bindAccountId: string;
    setBindAccountId: (id: string) => void;
    extraArgs: string;
    setExtraArgs: (args: string) => void;
    creating: boolean;
    onSubmit: (e: React.FormEvent) => void;
}

export function CreateInstanceModal({
    isOpen,
    onClose,
    platformId,
    platformLabel,
    instances,
    accounts,
    newInstanceName,
    setNewInstanceName,
    initMode,
    setInitMode,
    sourceInstanceId,
    setSourceInstanceId,
    existingDir,
    setExistingDir,
    bindAccountId,
    setBindAccountId,
    extraArgs,
    setExtraArgs,
    creating,
    onSubmit,
}: CreateInstanceModalProps) {
    if (!isOpen) return null;

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
                    maxWidth: '560px',
                    maxHeight: '90vh',
                    overflowY: 'auto',
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
                    <div>
                        <h3 style={{ fontSize: '1.1rem', fontWeight: 600, color: '#f8fafc', margin: 0 }}>
                            Create {platformLabel} Instance
                        </h3>
                        <p style={{ fontSize: '0.8rem', color: '#94a3b8', margin: '0.25rem 0 0 0' }}>
                            Configure a dedicated profile for multi-account isolation
                        </p>
                    </div>
                    <button
                        onClick={onClose}
                        style={{
                            backgroundColor: 'transparent',
                            border: 'none',
                            color: '#94a3b8',
                            cursor: 'pointer',
                            padding: '0.25rem',
                        }}
                    >
                        <X size={18} />
                    </button>
                </div>

                {/* Form */}
                <form onSubmit={onSubmit} style={{ padding: '1.25rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                    {/* 1. Instance Name */}
                    <div>
                        <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                            Instance Name *
                        </label>
                        <input
                            type="text"
                            required
                            placeholder="e.g. antigravity_work"
                            value={newInstanceName}
                            onChange={(e) => setNewInstanceName(e.target.value)}
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

                    {/* 2. Init Mode */}
                    <div>
                        <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                            Init Mode
                        </label>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
                            {/* Option 1: Copy source instance */}
                            <label
                                style={{
                                    display: 'flex',
                                    alignItems: 'flex-start',
                                    gap: '0.6rem',
                                    padding: '0.6rem 0.75rem',
                                    backgroundColor: initMode === 'copy_source' ? 'rgba(56, 189, 248, 0.08)' : '#0f172a',
                                    border: `1px solid ${initMode === 'copy_source' ? '#0284c7' : '#334155'}`,
                                    borderRadius: '0.375rem',
                                    cursor: 'pointer',
                                }}
                            >
                                <input
                                    type="radio"
                                    name="initMode"
                                    checked={initMode === 'copy_source'}
                                    onChange={() => setInitMode('copy_source')}
                                    style={{ marginTop: '0.2rem' }}
                                />
                                <div>
                                    <div style={{ fontSize: '0.825rem', fontWeight: 600, color: '#f8fafc' }}>
                                        Copy source instance
                                    </div>
                                    <div style={{ fontSize: '0.72rem', color: '#94a3b8' }}>
                                        Clone configuration and extensions from an existing instance
                                    </div>
                                    {initMode === 'copy_source' && (
                                        <div style={{ marginTop: '0.5rem' }}>
                                            <select
                                                value={sourceInstanceId}
                                                onChange={(e) => setSourceInstanceId(e.target.value)}
                                                style={{
                                                    width: '100%',
                                                    backgroundColor: '#1e293b',
                                                    border: '1px solid #334155',
                                                    borderRadius: '0.25rem',
                                                    padding: '0.35rem 0.5rem',
                                                    color: '#f8fafc',
                                                    fontSize: '0.78rem',
                                                    outline: 'none',
                                                }}
                                            >
                                                {instances.map((i) => (
                                                    <option key={i.id} value={i.id}>
                                                        {i.name} {i.is_default || i.isDefault ? '(Default)' : ''}
                                                    </option>
                                                ))}
                                            </select>
                                        </div>
                                    )}
                                </div>
                            </label>

                            {/* Option 2: Blank instance */}
                            <label
                                style={{
                                    display: 'flex',
                                    alignItems: 'flex-start',
                                    gap: '0.6rem',
                                    padding: '0.6rem 0.75rem',
                                    backgroundColor: initMode === 'blank' ? 'rgba(56, 189, 248, 0.08)' : '#0f172a',
                                    border: `1px solid ${initMode === 'blank' ? '#0284c7' : '#334155'}`,
                                    borderRadius: '0.375rem',
                                    cursor: 'pointer',
                                }}
                            >
                                <input
                                    type="radio"
                                    name="initMode"
                                    checked={initMode === 'blank'}
                                    onChange={() => setInitMode('blank')}
                                    style={{ marginTop: '0.2rem' }}
                                />
                                <div>
                                    <div style={{ fontSize: '0.825rem', fontWeight: 600, color: '#f8fafc' }}>
                                        Blank instance
                                    </div>
                                    <div style={{ fontSize: '0.72rem', color: '#94a3b8' }}>
                                        Initialize an empty, independent user data directory
                                    </div>
                                </div>
                            </label>

                            {/* Option 3: Use existing directory */}
                            <label
                                style={{
                                    display: 'flex',
                                    alignItems: 'flex-start',
                                    gap: '0.6rem',
                                    padding: '0.6rem 0.75rem',
                                    backgroundColor: initMode === 'existing_dir' ? 'rgba(56, 189, 248, 0.08)' : '#0f172a',
                                    border: `1px solid ${initMode === 'existing_dir' ? '#0284c7' : '#334155'}`,
                                    borderRadius: '0.375rem',
                                    cursor: 'pointer',
                                }}
                            >
                                <input
                                    type="radio"
                                    name="initMode"
                                    checked={initMode === 'existing_dir'}
                                    onChange={() => setInitMode('existing_dir')}
                                    style={{ marginTop: '0.2rem' }}
                                />
                                <div style={{ flex: 1 }}>
                                    <div style={{ fontSize: '0.825rem', fontWeight: 600, color: '#f8fafc' }}>
                                        Use existing directory
                                    </div>
                                    <div style={{ fontSize: '0.72rem', color: '#94a3b8' }}>
                                        Point to an already existing user data directory
                                    </div>
                                    {initMode === 'existing_dir' && (
                                        <div style={{ marginTop: '0.5rem' }}>
                                            <input
                                                type="text"
                                                placeholder="/path/to/existing/userDataDir"
                                                value={existingDir}
                                                onChange={(e) => setExistingDir(e.target.value)}
                                                style={{
                                                    width: '100%',
                                                    backgroundColor: '#1e293b',
                                                    border: '1px solid #334155',
                                                    borderRadius: '0.25rem',
                                                    padding: '0.35rem 0.5rem',
                                                    color: '#f8fafc',
                                                    fontSize: '0.78rem',
                                                    outline: 'none',
                                                }}
                                            />
                                        </div>
                                    )}
                                </div>
                            </label>
                        </div>
                    </div>

                    {/* 3. Instance Directory Preview */}
                    <div>
                        <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                            Instance Directory
                        </label>
                        <div style={{ display: 'flex', gap: '0.5rem' }}>
                            <input
                                type="text"
                                readOnly
                                value={
                                    initMode === 'existing_dir' && existingDir
                                        ? existingDir
                                        : `~/.cockpit_tools/instances/${platformId}/${newInstanceName || '<name>'}`
                                }
                                style={{
                                    flex: 1,
                                    backgroundColor: '#0f172a',
                                    border: '1px solid #334155',
                                    borderRadius: '0.375rem',
                                    padding: '0.5rem 0.75rem',
                                    color: '#94a3b8',
                                    fontSize: '0.8rem',
                                }}
                            />
                            <button
                                type="button"
                                onClick={() => {
                                    setInitMode('existing_dir');
                                }}
                                style={{
                                    backgroundColor: '#0f172a',
                                    border: '1px solid #334155',
                                    borderRadius: '0.375rem',
                                    color: '#cbd5e1',
                                    padding: '0.5rem 0.75rem',
                                    fontSize: '0.78rem',
                                    cursor: 'pointer',
                                    whiteSpace: 'nowrap',
                                }}
                            >
                                Select Folder
                            </button>
                        </div>
                    </div>

                    {/* 4. Bind Account */}
                    <div>
                        <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                            Bind account
                        </label>
                        <select
                            value={bindAccountId}
                            onChange={(e) => setBindAccountId(e.target.value)}
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
                        <span style={{ fontSize: '0.72rem', color: '#64748b', marginTop: '0.2rem', display: 'block' }}>
                            Chỉ hiển thị các tài khoản thuộc nền tảng {platformLabel} để bảo vệ an toàn thông tin xác thực.
                        </span>
                    </div>

                    {/* 5. Custom launch args */}
                    <div>
                        <label style={{ display: 'block', fontSize: '0.8rem', fontWeight: 600, color: '#cbd5e1', marginBottom: '0.4rem' }}>
                            Custom launch args (optional)
                        </label>
                        <input
                            type="text"
                            placeholder="e.g. --disable-gpu"
                            value={extraArgs}
                            onChange={(e) => setExtraArgs(e.target.value)}
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

                    {/* Footer Buttons */}
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
                            disabled={creating}
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
                            {creating ? 'Creating...' : 'Create Instance'}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}
