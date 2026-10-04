import {
    X,
    Globe,
    Key,
    Database,
    Copy,
    Check,
    RefreshCw,
    AlertCircle,
} from 'lucide-react';
import type { CockpitPlatformId } from './types';

interface AddAccountModalProps {
    isOpen: boolean;
    onClose: () => void;
    platformId: CockpitPlatformId;
    platformLabel: string;
    modalTab: 'oauth' | 'token' | 'local';
    setModalTab: (tab: 'oauth' | 'token' | 'local') => void;
    oauthUrl: string;
    oauthPolling: boolean;
    oauthError: string | null;
    copied: boolean;
    onCopyUrl: () => void;
    onOpenBrowser: () => void;
    tokenInput: string;
    setTokenInput: (val: string) => void;
    onSaveToken: () => void;
    onImportLocal: () => void;
    hasNativeOAuth: (pid: string) => boolean;
}

export function AddAccountModal({
    isOpen,
    onClose,
    platformId,
    platformLabel,
    modalTab,
    setModalTab,
    oauthUrl,
    oauthPolling,
    oauthError,
    copied,
    onCopyUrl,
    onOpenBrowser,
    tokenInput,
    setTokenInput,
    onSaveToken,
    onImportLocal,
    hasNativeOAuth,
}: AddAccountModalProps) {
    if (!isOpen) return null;

    return (
        <div className="cockpit-modal-overlay">
            <div className="cockpit-modal">
                {/* Header */}
                <div className="cockpit-modal-header">
                    <h3 className="cockpit-modal-title">
                        Add {platformLabel} Account
                    </h3>
                    <button
                        className="cockpit-modal-close"
                        onClick={onClose}
                    >
                        <X size={18} />
                    </button>
                </div>

                {/* Segmented Tabs */}
                <div className="cockpit-modal-tabs">
                    <button
                        className={`cockpit-modal-tab ${modalTab === 'oauth' ? 'active' : ''}`}
                        onClick={() => setModalTab('oauth')}
                    >
                        <Globe size={15} />
                        <span>OAuth Authorization</span>
                    </button>
                    <button
                        className={`cockpit-modal-tab ${modalTab === 'token' ? 'active' : ''}`}
                        onClick={() => setModalTab('token')}
                    >
                        <Key size={15} />
                        <span>Token / JSON</span>
                    </button>
                    <button
                        className={`cockpit-modal-tab ${modalTab === 'local' ? 'active' : ''}`}
                        onClick={() => setModalTab('local')}
                    >
                        <Database size={15} />
                        <span>Local Import</span>
                    </button>
                </div>

                {/* Tab 1: OAuth / Web Authorization */}
                {modalTab === 'oauth' && (
                    <div>
                        <div className="cockpit-modal-desc">
                            Click button below to open browser for OAuth login or copy URL.
                        </div>

                        {oauthError && (
                            <div style={{ padding: '0.6rem 0.75rem', borderRadius: 6, background: 'rgba(239, 68, 68, 0.15)', border: '1px solid var(--danger)', color: 'var(--danger)', fontSize: '0.8rem', marginBottom: '1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                <AlertCircle size={15} />
                                <span>{oauthError}</span>
                            </div>
                        )}

                        <div className="cockpit-url-box">
                            <input
                                type="text"
                                readOnly
                                value={oauthUrl || 'Generating OAuth Login URL...'}
                                className="cockpit-url-input"
                            />
                            {oauthUrl && (
                                <button
                                    className="cockpit-url-copy"
                                    onClick={onCopyUrl}
                                    title={copied ? 'Đã sao chép!' : 'Sao chép URL'}
                                >
                                    {copied ? <Check size={15} color="var(--success)" /> : <Copy size={15} />}
                                </button>
                            )}
                        </div>

                        {hasNativeOAuth(platformId) && (
                            <div className="cockpit-expiry-meta">
                                Expires in: 600s; Poll interval: 2s
                            </div>
                        )}

                        <button
                            className="cockpit-btn-browser"
                            onClick={onOpenBrowser}
                            disabled={!oauthUrl}
                        >
                            <Globe size={16} />
                            <span>Open in Browser</span>
                        </button>

                        {hasNativeOAuth(platformId) && oauthPolling && (
                            <div className="cockpit-polling-bar">
                                <RefreshCw size={15} className="spin" />
                                <span className="cockpit-polling-text">Waiting for authorization...</span>
                            </div>
                        )}

                        {hasNativeOAuth(platformId) ? (
                            <div className="cockpit-footer-hint">
                                Once authorized, this window will update automatically
                            </div>
                        ) : (
                            <div style={{ display: 'flex', gap: '0.5rem', marginTop: '1rem' }}>
                                <button
                                    className="btn"
                                    style={{ flex: 1, padding: '0.5rem', fontSize: '0.78rem' }}
                                    onClick={() => setModalTab('token')}
                                >
                                    <Key size={14} /> Chuyển sang Token / JSON
                                </button>
                                <button
                                    className="btn"
                                    style={{ flex: 1, padding: '0.5rem', fontSize: '0.78rem' }}
                                    onClick={() => setModalTab('local')}
                                >
                                    <Database size={14} /> Chuyển sang Local Import
                                </button>
                            </div>
                        )}
                    </div>
                )}

                {/* Tab 2: Token / JSON */}
                {modalTab === 'token' && (
                    <div>
                        <div className="cockpit-modal-desc">
                            Paste your Access Token, Refresh Token, or Cockpit export JSON bundle below.
                        </div>

                        <div className="cockpit-callout">
                            <div className="cockpit-callout-title">Direct Token Entry</div>
                            <ul className="cockpit-callout-list">
                                <li>Accepts raw Bearer tokens or JSON objects containing access_token & refresh_token.</li>
                                <li>Tokens are stored securely with local AES-GCM encryption in your local vault.</li>
                            </ul>
                        </div>

                        <textarea
                            className="input"
                            placeholder='{"access_token": "ey...", "refresh_token": "..."}'
                            rows={5}
                            value={tokenInput}
                            onChange={(e) => setTokenInput(e.target.value)}
                            style={{
                                width: '100%',
                                resize: 'none',
                                fontFamily: 'var(--font-mono)',
                                fontSize: '0.8rem',
                                background: '#0c111a',
                                borderColor: 'rgba(255, 255, 255, 0.1)',
                                color: '#cbd5e1',
                                marginBottom: '1rem',
                            }}
                        />

                        <button
                            className="cockpit-btn-browser"
                            onClick={onSaveToken}
                        >
                            <Key size={16} />
                            <span>Lưu Token vào Kho</span>
                        </button>
                    </div>
                )}

                {/* Tab 3: Local Import */}
                {modalTab === 'local' && (
                    <div>
                        <div className="cockpit-modal-desc">
                            Automatically scan and import accounts from local IDE database or Cockpit vaults.
                        </div>

                        <div className="cockpit-callout">
                            <div className="cockpit-callout-title">Supported Local Sources for {platformLabel}</div>
                            <ul className="cockpit-callout-list">
                                <li>Cockpit Vaults: ~/.cockpit_tools/{platformId === 'zed' ? 'zed_accounts' : platformId === 'github_copilot' ? 'github_copilot_accounts' : platformId === 'cursor' ? 'cursor_accounts' : platformId === 'windsurf' ? 'windsurf_accounts' : platformId === 'trae' ? 'trae_accounts' : 'codebuddy_accounts'} and backup archives.</li>
                                <li>IDE Storage: ~/.config/{platformId === 'zed' ? 'zed' : platformId === 'cursor' ? 'Cursor' : platformId === 'windsurf' ? 'Windsurf' : 'Code'} and globalStorage state databases.</li>
                                <li>All extracted sessions are decrypted and imported locally without uploading.</li>
                            </ul>
                        </div>

                        <button
                            className="cockpit-btn-browser"
                            onClick={onImportLocal}
                        >
                            <Database size={16} />
                            <span>Quét & Đồng bộ từ IDE / Cockpit Cục bộ</span>
                        </button>
                    </div>
                )}
            </div>
        </div>
    );
}
