import { useCallback, useEffect, useRef, useState } from 'react';
import { Copy, ExternalLink, Play, Square, Terminal } from 'lucide-react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { WebStatus } from '../../../bridge/types';
import {
    getWebStatus,
    launchOpenCodeTerminal,
    openWebUrl,
    startWeb,
    stopWeb,
} from '../../../bridge/web_control_bridge';
import {
    copyLocalWebUrl, isNewerWebStatus, openLocalWebUrl, webControlMatrix,
} from '../utils/webControlState.js';
import { useTranslation } from '../utils/i18n';

const INITIAL_STATUS: WebStatus = {
    state: 'stopped', url: null, error: null, has_owned_child: false, generation: 0, revision: 0,
};

interface Props {
    onError: (message: string) => void;
}

export function OpenCodeControls({ onError }: Props) {
    const { t } = useTranslation();
    const [status, setStatus] = useState<WebStatus>(INITIAL_STATUS);
    const latest = useRef<WebStatus>(INITIAL_STATUS);
    const queue = useRef<Promise<void>>(Promise.resolve());
    const [mutationBusy, setMutationBusy] = useState(false);
    const [terminalBusy, setTerminalBusy] = useState(false);
    const [copied, setCopied] = useState(false);

    const accept = useCallback((candidate: WebStatus) => {
        if (!isNewerWebStatus(candidate, latest.current)) return;
        latest.current = candidate;
        setStatus(candidate);
    }, []);

    useEffect(() => {
        let alive = true;
        let unlisten: UnlistenFn | null = null;

        listen<WebStatus>('web_status_changed', event => {
            if (alive) accept(event.payload);
        }).then(fn => {
            if (alive) unlisten = fn;
            else fn();
        }).catch(err => console.error('Lắng nghe web_status_changed lỗi:', err));

        let timer: number | undefined;

        const schedulePoll = () => {
            if (!alive) return;
            const currentState = latest.current.state;
            // Đã có Tauri event đẩy trực tiếp khi đổi trạng thái; polling chỉ là nhịp tim dự phòng.
            const delay = (currentState === 'starting' || currentState === 'stopping')
                ? 1000
                : currentState === 'running'
                    ? 5000
                    : 15000;

            timer = window.setTimeout(async () => {
                try {
                    const next = await getWebStatus();
                    if (alive) accept(next);
                } catch (error) {
                    if (alive) onError(String(error));
                } finally {
                    schedulePoll();
                }
            }, delay);
        };

        const pollImmediate = async () => {
            try {
                const next = await getWebStatus();
                if (alive) accept(next);
            } catch (error) {
                if (alive) onError(String(error));
            }
        };

        void pollImmediate();
        schedulePoll();

        const onFocus = () => void pollImmediate();
        window.addEventListener('focus', onFocus);

        return () => {
            alive = false;
            unlisten?.();
            if (timer) window.clearTimeout(timer);
            window.removeEventListener('focus', onFocus);
        };
    }, [accept, onError]);

    const mutate = (operation: () => Promise<WebStatus>) => {
        setMutationBusy(true);
        queue.current = queue.current
            .catch(() => undefined)
            .then(async () => accept(await operation()))
            .catch(error => onError(String(error)))
            .finally(() => setMutationBusy(false));
    };

    const launchTerminal = async () => {
        setTerminalBusy(true);
        try {
            await launchOpenCodeTerminal();
        } catch (error) {
            onError(String(error));
        } finally {
            setTerminalBusy(false);
        }
    };

    const copyUrl = async () => {
        if (!status.url) return;
        try {
            await copyLocalWebUrl(status.url, url => navigator.clipboard.writeText(url));
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1500);
        } catch (error) {
            onError(`${t('models.web_copy_failed')}: ${String(error)}`);
        }
    };

    const openUrl = async () => {
        if (!status.url) return;
        try {
            await openLocalWebUrl(status.url, openWebUrl);
        } catch (error) {
            onError(String(error));
        }
    };

    const disabled = webControlMatrix(status, mutationBusy);
    const displayUrl = status.url ?? t('models.web_no_url');

    return (
        <section className="glass-panel opencode-controls" aria-label={t('models.web_title')}>
            <div className="opencode-controls__link">
                <strong>{t('models.web_title')}</strong>
                <span className={`badge web-state web-state--${status.state}`}>{t(`models.web_state_${status.state}`)}</span>
                <code title={displayUrl}>{displayUrl}</code>
                {status.error && <small className="opencode-controls__error">{status.error}</small>}
            </div>
            <div className="opencode-controls__actions">
                <button className="btn" onClick={launchTerminal} disabled={terminalBusy} title={t('models.terminal_hint')}>
                    <Terminal size={16} /> {t('models.terminal_open')}
                </button>
                <button className="btn btn-primary" onClick={() => mutate(startWeb)} disabled={disabled.startDisabled}>
                    <Play size={16} /> {t('models.web_start')}
                </button>
                <button className="btn btn-danger" onClick={() => mutate(stopWeb)} disabled={disabled.stopDisabled}>
                    <Square size={16} /> {t('models.web_stop')}
                </button>
                <button className="btn" onClick={copyUrl} disabled={disabled.copyDisabled}>
                    <Copy size={16} /> {copied ? t('models.web_copied') : t('models.web_copy')}
                </button>
                <button className="btn" onClick={openUrl} disabled={disabled.openDisabled}>
                    <ExternalLink size={16} /> {t('models.web_open')}
                </button>
            </div>
        </section>
    );
}
