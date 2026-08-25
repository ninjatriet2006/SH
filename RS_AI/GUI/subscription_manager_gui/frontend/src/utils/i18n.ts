import { useSettingsStore } from '../store/useSettingsStore';

// Hook đa ngôn ngữ mô phỏng ID Linking (Langs by ID)
export function useTranslation() {
    const dictionary = useSettingsStore(state => state.dictionary);

    // Hàm lấy chuỗi bằng ID (VD: t('sidebar.dashboard'))
    const t = (key: string): string => {
        if (!dictionary || Object.keys(dictionary).length === 0) return "";

        const keys = key.split('.');
        let current: any = dictionary;

        for (const k of keys) {
            if (current[k] === undefined) {
                return ""; // Fallback trả về rỗng nếu thiếu
            }
            current = current[k];
        }

        return typeof current === 'string' ? current : "";
    };

    return { t };
}

// Hàm format ngày giờ tuân thủ múi giờ
export function formatDateTime(timestamp: number | string | Date, showTime: boolean = true): string {
    const timezone = useSettingsStore.getState().timezone;
    const date = new Date(timestamp);
    
    const options: Intl.DateTimeFormatOptions = {
        timeZone: timezone,
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
    };

    if (showTime) {
        options.hour = '2-digit';
        options.minute = '2-digit';
        options.second = '2-digit';
        options.hour12 = false; // Sử dụng hệ 24h
    }

    try {
        // Dùng locale en-GB/vi-VN để có định dạng dd/MM/yyyy. Tạm dùng 'vi-VN' mặc định formating
        return date.toLocaleString('vi-VN', options);
    } catch (e) {
        // Fallback
        return date.toLocaleString();
    }
}

// Hàm format tiền tệ
export function formatCurrency(amount: number): string {
    return amount.toLocaleString('vi-VN') + ' đ';
}
