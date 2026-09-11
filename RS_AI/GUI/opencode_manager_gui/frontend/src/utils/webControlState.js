export function isLocalWebUrl(url) {
    if (typeof url !== 'string') return false;
    try {
        const parsed = new URL(url);
        return parsed.protocol === 'http:'
            && (parsed.hostname === '127.0.0.1' || parsed.hostname === 'localhost')
            && parsed.port !== ''
            && parsed.port !== '0'
            && parsed.username === ''
            && parsed.password === ''
            && parsed.pathname === '/'
            && parsed.search === ''
            && parsed.hash === '';
    } catch {
        return false;
    }
}

export function isNewerWebStatus(candidate, current) {
    return candidate.generation > current.generation
        || (candidate.generation === current.generation && candidate.revision >= current.revision);
}

export function webControlMatrix(status, mutationBusy = false) {
    const transitional = status.state === 'starting' || status.state === 'stopping';
    const runningWithUrl = status.state === 'running' && isLocalWebUrl(status.url);
    return {
        startDisabled: mutationBusy || transitional || status.state === 'running',
        stopDisabled: mutationBusy || transitional || !status.has_owned_child,
        copyDisabled: mutationBusy || !runningWithUrl,
        openDisabled: mutationBusy || !runningWithUrl,
    };
}

export async function copyLocalWebUrl(url, writeText) {
    if (!isLocalWebUrl(url)) throw new Error('invalid_localhost_url');
    await writeText(url);
}

export async function openLocalWebUrl(url, openUrl) {
    if (!isLocalWebUrl(url)) throw new Error('invalid_localhost_url');
    await openUrl(url);
}
