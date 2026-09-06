/*
[INTEGRITY NOTES]
- Mục đích: Badge trạng thái kết nối của provider (dùng ở nhiều trang).
- Trách nhiệm: Đổi `StatusKind` thành nhãn + màu. Trạng thái chưa kiểm tra và
  đang kiểm tra được phân biệt rõ — nếu gộp lại, người dùng không biết là app
  đang chạy hay chưa làm gì.
- Tương tác: `pages/ProvidersPage.tsx`, `pages/CleanupPage.tsx`.
*/

import type { StatusKind } from '../../../bridge/types';
import { useTranslation } from '../utils/i18n';

interface StatusBadgeProps {
    kind?: StatusKind;
    isChecking?: boolean;
    message?: string;
}

export function StatusBadge({ kind, isChecking, message }: StatusBadgeProps) {
    const { t } = useTranslation();

    if (isChecking) {
        return (
            <span className="badge badge-inactive" title={t('status.checking')}>
                ⏳ {t('status.checking')}
            </span>
        );
    }
    if (!kind) {
        return <span className="badge badge-inactive">{t('status.unknown')}</span>;
    }

    const map: Record<StatusKind, { cls: string; icon: string; color?: string }> = {
        alive: { cls: 'badge-active', icon: '✓' },
        no_credits: { cls: 'badge-inactive', icon: '⚠', color: 'var(--warning)' },
        invalid_key: { cls: 'badge-inactive', icon: '✗', color: 'var(--danger)' },
        offline: { cls: 'badge-inactive', icon: '⊘', color: 'var(--text-secondary)' },
    };
    const cfg = map[kind];

    return (
        <span
            className={`badge ${cfg.cls}`}
            style={cfg.color ? { color: cfg.color, borderColor: cfg.color } : undefined}
            // Lý do chi tiết từ API nằm ở tooltip: hàng bảng không đủ chỗ, mà
            // bỏ hẳn thì người dùng không biết vì sao lỗi.
            title={message || t(`status.${kind}`)}
        >
            {cfg.icon} {t(`status.${kind}`)}
        </span>
    );
}
