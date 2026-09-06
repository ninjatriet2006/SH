/*
[INTEGRITY NOTES]
- Mục đích: Form thêm/sửa nhà cung cấp AI.
- Trách nhiệm: Chọn preset (tự điền tên + URL), nhập tên/URL/khoá, kiểm tra kết
  nối trước khi lưu, và xử lý luồng "phát hiện trùng → hỏi gộp".
- Tương tác: `store/useProviderStore.ts`, `bridge/provider_bridge.ts`.

Ghi chú thiết kế:
  - Chế độ SỬA: ô khoá để trống nghĩa là GIỮ khoá hiện tại. Không hiển thị khoá
    thật cho tới khi người dùng bấm "hiện" (gọi `getProviderSecret`) — key không
    nằm sẵn trong DOM suốt phiên.
  - Backend đã chuẩn hoá URL và phát hiện trùng; UI chỉ trình bày lại kết quả,
    không tự suy luận song song (hai nơi cùng logic là nguồn của lệch hành vi).
*/

import React, { useEffect, useState } from 'react';
import { X, Eye, EyeOff, Plug } from 'lucide-react';
import type { ProviderView, PresetView, StatusView } from '../../../bridge/types';
import { getProviderSecret, testConnection } from '../../../bridge/provider_bridge';
import { useTranslation } from '../utils/i18n';
import { StatusBadge } from './StatusBadge';

interface ProviderModalProps {
    isOpen: boolean;
    /** null = thêm mới. */
    provider: ProviderView | null;
    presets: PresetView[];
    onClose: () => void;
    /** Trả về `true` nếu đã lưu xong (modal sẽ đóng). */
    onSave: (args: {
        presetId: string;
        name: string;
        baseUrl: string;
        apiKey: string;
        forceOverwriteId?: string;
        npm: string;
        customId?: string | null;
    }) => Promise<boolean>;
}

/** Hai package AI SDK mà docs opencode khuyên dùng cho custom provider. */
export const NPM_OPTIONS = ['@ai-sdk/openai-compatible', '@ai-sdk/openai'] as const;

/** Bộ ký tự an toàn cho ID provider — phải khớp `validate_custom_id` backend. */
const ID_PATTERN = /^[A-Za-z0-9_-]{1,64}$/;

export function ProviderModal({ isOpen, provider, presets, onClose, onSave }: ProviderModalProps) {
    const { t } = useTranslation();
    const [presetId, setPresetId] = useState('custom');
    const [npm, setNpm] = useState<string>(NPM_OPTIONS[0]);
    const [name, setName] = useState('');
    const [baseUrl, setBaseUrl] = useState('');
    const [apiKey, setApiKey] = useState('');
    const [customId, setCustomId] = useState('');
    const [showKey, setShowKey] = useState(false);
    const [testStatus, setTestStatus] = useState<StatusView | null>(null);
    const [isTesting, setIsTesting] = useState(false);
    const [isSaving, setIsSaving] = useState(false);
    const [notice, setNotice] = useState<string | null>(null);

    // Reset form mỗi lần mở để không mang dữ liệu của provider trước.
    useEffect(() => {
        if (!isOpen) return;
        setTestStatus(null);
        setNotice(null);
        setShowKey(false);
        setIsSaving(false);
        if (provider) {
            setPresetId(provider.is_builtin ? provider.id : 'custom');
            setNpm(provider.npm ?? NPM_OPTIONS[0]);
            setName(provider.name);
            setBaseUrl(provider.base_url);
            setApiKey('');
            setCustomId(provider.id);
        } else {
            setPresetId('custom');
            setNpm(NPM_OPTIONS[0]);
            setName('');
            setBaseUrl('');
            setApiKey('');
            setCustomId('');
        }
    }, [isOpen, provider]);

    if (!isOpen) return null;

    const handlePresetChange = (id: string) => {
        setPresetId(id);
        const p = presets.find(x => x.id === id);
        // Preset điền sẵn tên + URL; "custom" để người dùng tự nhập.
        if (p && p.id !== 'custom') {
            setName(p.name);
            setBaseUrl(p.base_url);
            if (p.npm) setNpm(p.npm);
        }
    };

    const handleRevealKey = async () => {
        if (showKey) {
            setShowKey(false);
            return;
        }
        // Chỉ tải khoá thật khi người dùng chủ động yêu cầu.
        if (provider && apiKey === '') {
            try {
                setApiKey(await getProviderSecret(provider.id));
            } catch (err) {
                alert(`Không lấy được API key: ${err instanceof Error ? err.message : String(err)}`);
                return;
            }
        }
        setShowKey(true);
    };

    const handleTest = async () => {
        let keyToTest = apiKey.trim();
        // Sửa mà chưa nhập khoá mới → kiểm tra bằng khoá đang lưu.
        if (!keyToTest && provider) {
            try {
                keyToTest = await getProviderSecret(provider.id);
            } catch { /* để rơi vào nhánh báo thiếu bên dưới */ }
        }
        if (!baseUrl.trim() || !keyToTest) {
            setNotice('Cần cả Base URL và API Key để kiểm tra.');
            return;
        }
        setIsTesting(true);
        setNotice(null);
        try {
            setTestStatus(await testConnection(baseUrl, keyToTest));
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setIsTesting(false);
        }
    };

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!name.trim() || !baseUrl.trim()) {
            setNotice('Vui lòng nhập Tên và Base URL.');
            return;
        }

        // ID tự đặt: kiểm tra ký tự ngay ở UI cho phản hồi tức thì (backend vẫn
        // là chốt cuối). Sửa mà không đổi ID thì bỏ qua — id cũ có thể do app
        // khác tạo với ký tự ngoài bộ an toàn, không được chặn người dùng.
        const idToSave = customId.trim();
        const idChanged = !provider || idToSave !== provider.id;
        if (idToSave && idChanged && !ID_PATTERN.test(idToSave)) {
            setNotice(t('provider_modal.id_invalid'));
            return;
        }

        let keyToSave = apiKey.trim();
        if (!keyToSave) {
            if (!provider) {
                setNotice('Vui lòng nhập API Key.');
                return;
            }
            // Giữ khoá cũ khi sửa mà để trống.
            try {
                keyToSave = await getProviderSecret(provider.id);
            } catch (err) {
                setNotice(`Không lấy được khoá hiện tại: ${err instanceof Error ? err.message : String(err)}`);
                return;
            }
        }

        setIsSaving(true);
        try {
            const done = await onSave({
                presetId, name, baseUrl, apiKey: keyToSave, npm,
                customId: idToSave || null,
            });
            if (done) onClose();
        } catch (err) {
            setNotice(err instanceof Error ? err.message : String(err));
        } finally {
            setIsSaving(false);
        }
    };

    return (
        <div className="modal-overlay">
            <div className="modal-content animate-fade-in" style={{ maxWidth: '520px' }}>
                <button
                    onClick={onClose}
                    style={{ position: 'absolute', top: '1rem', right: '1rem', background: 'transparent', border: 'none', color: 'white', cursor: 'pointer' }}
                >
                    <X size={20} />
                </button>

                <h3>{provider ? t('provider_modal.edit_title') : t('provider_modal.add_title')}</h3>

                <form onSubmit={handleSubmit}>
                    <div className="form-group">
                        <label className="form-label">{t('provider_modal.lbl_preset')}</label>
                        <select
                            className="input-field"
                            value={presetId}
                            onChange={e => handlePresetChange(e.target.value)}
                        >
                            {presets.map(p => (
                                <option key={p.id} value={p.id}>
                                    {p.id === 'custom' ? t('provider_modal.custom_preset') : p.name}
                                </option>
                            ))}
                        </select>
                    </div>

                    <div className="form-group">
                        <label className="form-label">{t('provider_modal.lbl_id')}</label>
                        <input
                            type="text"
                            className="input-field"
                            style={{ fontFamily: 'monospace' }}
                            value={customId}
                            onChange={e => setCustomId(e.target.value)}
                            placeholder={t('provider_modal.id_placeholder')}
                            autoComplete="off"
                            spellCheck={false}
                        />
                        <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '4px' }}>
                            {provider ? t('provider_modal.id_hint_edit') : t('provider_modal.id_hint_add')}
                        </small>
                    </div>

                    <div className="form-group">
                        <label className="form-label">{t('provider_modal.lbl_name')}</label>
                        <input
                            type="text"
                            className="input-field"
                            value={name}
                            onChange={e => setName(e.target.value)}
                            required
                        />
                    </div>

                    <div className="form-group">
                        <label className="form-label">{t('provider_modal.lbl_url')}</label>
                        <input
                            type="text"
                            className="input-field"
                            value={baseUrl}
                            onChange={e => setBaseUrl(e.target.value)}
                            placeholder="https://api.example.com/v1"
                            required
                        />
                        <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '4px' }}>
                            {t('provider_modal.url_hint')}
                        </small>
                    </div>

                    <div className="form-group">
                        <label className="form-label">{t('provider_modal.lbl_key')}</label>
                        <div style={{ display: 'flex', gap: '0.5rem' }}>
                            <input
                                type={showKey ? 'text' : 'password'}
                                className="input-field"
                                style={{ flex: 1 }}
                                value={apiKey}
                                onChange={e => setApiKey(e.target.value)}
                                placeholder={provider ? provider.api_key_masked : 'sk-...'}
                            />
                            <button
                                type="button"
                                className="btn"
                                style={{ padding: '0.4rem 0.6rem', background: 'rgba(255,255,255,0.08)' }}
                                onClick={handleRevealKey}
                                title={showKey ? 'Ẩn' : 'Hiện'}
                            >
                                {showKey ? <EyeOff size={16} /> : <Eye size={16} />}
                            </button>
                        </div>
                        <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '4px' }}>
                            {provider ? t('provider_modal.keep_key') : t('provider_modal.key_hint')}
                        </small>
                    </div>

                    <div className="form-group">
                        <label className="form-label">{t('provider_modal.lbl_npm')}</label>
                        <select
                            className="input-field"
                            value={NPM_OPTIONS.includes(npm as typeof NPM_OPTIONS[number]) ? npm : NPM_OPTIONS[0]}
                            onChange={e => setNpm(e.target.value)}
                        >
                            <option value="@ai-sdk/openai-compatible">@ai-sdk/openai-compatible ({t('provider_modal.npm_chat')})</option>
                            <option value="@ai-sdk/openai">@ai-sdk/openai ({t('provider_modal.npm_responses')})</option>
                        </select>
                        <small style={{ color: 'var(--text-secondary)', display: 'block', marginTop: '4px' }}>
                            {t('provider_modal.npm_hint')}
                        </small>
                    </div>

                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', marginTop: '1rem', flexWrap: 'wrap' }}>
                        <button
                            type="button"
                            className="btn"
                            style={{ background: 'rgba(255,255,255,0.08)' }}
                            onClick={handleTest}
                            disabled={isTesting}
                        >
                            <Plug size={16} /> {isTesting ? t('status.checking') : t('provider_modal.test')}
                        </button>
                        {testStatus && <StatusBadge kind={testStatus.kind} message={testStatus.message} />}
                    </div>

                    {notice && (
                        <div style={{ marginTop: '1rem', padding: '0.6rem 0.75rem', background: 'rgba(239,68,68,0.12)', borderLeft: '3px solid var(--danger)', borderRadius: '4px', fontSize: '0.85rem' }}>
                            {notice}
                        </div>
                    )}

                    <div className="modal-actions">
                        <button type="button" className="btn" onClick={onClose}>{t('common.cancel')}</button>
                        <button type="submit" className="btn btn-primary" disabled={isSaving}>
                            {isSaving ? t('common.loading') : t('common.save')}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}
