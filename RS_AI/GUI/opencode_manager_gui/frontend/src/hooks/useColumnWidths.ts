/*
[INTEGRITY NOTES]
 - Mục đích: Hook đổi độ rộng cột bảng bằng kéo thả, ghi đè lưu ở localStorage
   theo từng bảng ("providers", ...); nhấn đúp handle để về độ rộng mặc định.
- Trách nhiệm: Chỉ giữ state widths + persist; không biết cấu trúc bảng — trang
   tự map key cột sang <colgroup>/<th>. Độ rộng đo từ THẺ đang render (co giãn
   theo bảng) nên kéo không bị nhảy.
- Tương tác: `pages/ProvidersPage.tsx`, `utils/dragResize.ts`.
*/

import { useCallback, useState } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { startHorizontalDrag } from '../utils/dragResize';

export interface ResizableColumn {
    key: string;
    /** Độ rộng mặc định (px) — cũng là giá trị sau khi nhấn đúp đặt lại. */
    defaultWidth: number;
    /** Hạn dưới khi kéo, chặn ép cột nhỏ tới mức mất nội dung. */
    minWidth: number;
}

const PREFIX = 'opencode-manager:col-widths:';

function loadOverrides(storageKey: string): Record<string, number> {
    try {
        const raw = localStorage.getItem(PREFIX + storageKey);
        if (!raw) return {};
        const parsed: unknown = JSON.parse(raw);
        if (typeof parsed !== 'object' || parsed === null) return {};
        const out: Record<string, number> = {};
        for (const [k, v] of Object.entries(parsed as Record<string, unknown>)) {
            if (typeof v === 'number' && Number.isFinite(v) && v > 0) out[k] = v;
        }
        return out;
    } catch {
        return {};
    }
}

export function useColumnWidths(storageKey: string, columns: ResizableColumn[]) {
    const [overrides, setOverrides] = useState<Record<string, number>>(() => loadOverrides(storageKey));

    /** Ghi state và localStorage trong CÙNG một bản cập nhật để hai nơi không lệch nhau. */
    const applyAndPersist = useCallback((mutate: (prev: Record<string, number>) => Record<string, number>) => {
        setOverrides(prev => {
            const next = mutate(prev);
            try {
                localStorage.setItem(PREFIX + storageKey, JSON.stringify(next));
            } catch { /* storage đầy/bị chặn: vẫn đổi được trong phiên này */ }
            return next;
        });
    }, [storageKey]);

    /** Gọi từ onMouseDown của handle trong <th> — đo thẻ đang render rồi kéo theo delta. */
    const startResize = useCallback((key: string, e: ReactMouseEvent) => {
        const th = e.currentTarget.closest('th');
        if (!th) return;
        e.preventDefault();
        const min = columns.find(c => c.key === key)?.minWidth ?? 60;
        const startX = e.clientX;
        const startWidth = th.getBoundingClientRect().width;
        startHorizontalDrag(clientX => {
            const width = Math.max(min, Math.round(startWidth + clientX - startX));
            applyAndPersist(prev => ({ ...prev, [key]: width }));
        });
    }, [columns, applyAndPersist]);

    /** Nhấn đúp vào handle → bỏ ghi đè, cột về độ rộng mặc định. */
    const resetResize = useCallback((key: string) => {
        applyAndPersist(prev => {
            const next = { ...prev };
            delete next[key];
            return next;
        });
    }, [applyAndPersist]);

    /** Độ rộng hiện tại của cột: ghi đè nếu có, không thì mặc định. */
    const widthOf = useCallback(
        (key: string): number => overrides[key] ?? columns.find(c => c.key === key)?.defaultWidth ?? 120,
        [overrides, columns],
    );

    return { widthOf, startResize, resetResize };
}
