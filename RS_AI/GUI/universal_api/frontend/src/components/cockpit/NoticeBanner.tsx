import { useState } from 'react';
import { AlertCircle, ChevronDown } from 'lucide-react';

interface NoticeBannerProps {
    noticeTitle: string;
    permissionScope: string;
    networkScope: string;
}

export function NoticeBanner({
    noticeTitle,
    permissionScope,
    networkScope,
}: NoticeBannerProps) {
    const [noticeExpanded, setNoticeExpanded] = useState(true);

    return (
        <div className="flow-notice-card">
            <div className="flow-notice-header" onClick={() => setNoticeExpanded(!noticeExpanded)}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <AlertCircle size={16} />
                    <span>{noticeTitle}</span>
                </div>
                <ChevronDown
                    size={16}
                    style={{
                        transform: noticeExpanded ? 'rotate(180deg)' : 'rotate(0deg)',
                        transition: 'transform 0.2s ease',
                    }}
                />
            </div>

            {noticeExpanded && (
                <div className="flow-notice-body">
                    <div>
                        Switching accounts requires reading local auth storage and calling system credential services for decryption/re-encryption. Data is processed locally only.
                    </div>
                    <ul>
                        <li>Permission Scope: {permissionScope}</li>
                        <li>Network Scope: {networkScope}</li>
                        <li>Data Storage: Local encrypted vault (Cockpit Tools standard)</li>
                    </ul>
                </div>
            )}
        </div>
    );
}
