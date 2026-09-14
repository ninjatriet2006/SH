import { useSettingsStore } from '../store/useSettingsStore';

export function useTranslation() {
    const dictionary = useSettingsStore(state => state.dictionary);

    const t = (key: string): string => {
        if (!dictionary || Object.keys(dictionary).length === 0) return key;
        let current: unknown = dictionary;
        for (const seg of key.split('.')) {
            if (typeof current !== 'object' || current === null) return key;
            current = (current as Record<string, unknown>)[seg];
            if (current === undefined) return key;
        }
        return typeof current === 'string' ? current : key;
    };

    return { t };
}

export function langLabel(code: string): string {
    try {
        const name = new Intl.DisplayNames([code], { type: 'language' }).of(code);
        if (name && name.toLowerCase() !== code.toLowerCase()) {
            return `${name} (${code})`;
        }
    } catch { /* show raw code */ }
    return code;
}
